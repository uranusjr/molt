import collections
import sys
import warnings

import tomlkit

from packaging.specifiers import SpecifierSet
from packaging.utils import canonicalize_name

from molt.locks import LockFile


class PoetryLockError(Exception):
    pass


class PackageSpecifierNotSupported(PoetryLockError, ValueError):
    pass


class SourceNameDuplicated(PoetryLockError):
    pass


class SourceDropped(UserWarning):
    def __init__(self, package_name):
        super(SourceDropped, self).__init__(
            "Source dropped for package {!r} (invalid in this context)".format(
                package_name
            )
        )
        self.package_name = package_name


def load(f, encoding=None):
    """Parse a poetry.lock file.

    If `encoding` is specified, `f` is treated as binary; if `encoding` is
    not specified or `None`, `f` should be opened in text mode.
    """
    text = f.read()
    if encoding is not None:
        text = text.decode(encoding)
    # Yes, this simply returns a dict. I guess it is enough since we only want
    # to convert it to molt.lock.json anyway?
    return tomlkit.parse(text)


def _supports_this_python(requires_python):
    if requires_python is None or requires_python == "*":
        return True
    spec = SpecifierSet(requires_python)
    return ".".join(str(x) for x in sys.version_info[:3]) in spec


def _parse_spec(package):
    try:
        source = package["source"]
    except KeyError:
        # A package without source must be named.
        return {"version": package["version"]}

    source_type = source["type"]
    if source_type == "file":
        # Poetry seems to only support local file reference, although the key
        # is confusingly named "url".
        return {"path": source["url"]}
    elif source_type in ["git", "hg", "bzr", "svn"]:
        return {
            "vcs": "{}+{}".format(source_type, source["url"]),
            "rev": source["reference"],
        }

    try:
        version = package["version"]
    except KeyError:
        raise PackageSpecifierNotSupported(package)
    return {"version": version}


_Source = collections.namedtuple("_Source", ["name", "url"])


def _parse_source(package):
    try:
        source = package["source"]
    except KeyError:
        return None
    if source["type"] != "legacy":
        return None
    return _Source(source["reference"], source["url"])


def _generate_packages(poetry_lock):
    for package_data in poetry_lock["package"]:
        name = package_data["name"]
        result = {"name": name}

        spec = _parse_spec(package_data)
        result.update(spec)

        source = _parse_source(package_data)
        if source is not None:
            if "version" not in result:
                warnings.warn(SourceDropped(name))
            else:
                result["source"] = source.name

        yield (canonicalize_name(name), result, source)


def _remove_if_same_section(top_level_packages, dependent, depended_name):
    try:
        depended = top_level_packages[depended_name]
    except KeyError:
        return
    # The depended does not need to be a top-level if it's in the same section.
    # It will be collected when the dependant is traversed.
    if depended["category"] == dependent["category"]:
        del top_level_packages[depended_name]


def _generate_dependencies(poetry_lock):
    top_level_packages = {
        canonicalize_name(p["name"]): p for p in poetry_lock["package"]
    }
    packages_markers = {
        canonicalize_name(p["name"]): p["marker"].replace('"', "'")
        for p in poetry_lock["package"]
        if "marker" in p
    }

    for package_data in poetry_lock["package"]:
        for dep in package_data.get("dependencies", ()):
            dep = canonicalize_name(dep)
            _remove_if_same_section(top_level_packages, package_data, dep)
            package_name = canonicalize_name(package_data["name"])
            yield package_name, dep, packages_markers.get(dep)

    # A package is a top-level dependency if it is not referenced by anyone.
    for package_data in top_level_packages.values():
        if package_data.get("optional"):
            continue
        if package_data["category"] == "main":
            key = ""
        else:
            key = "[{}]".format(package_data["category"])
        dep = canonicalize_name(package_data["name"])
        yield key, dep, packages_markers.get(dep)

    for extra_name, extra_pkg_names in poetry_lock.get("extras", {}).items():
        for dep in extra_pkg_names:
            dep = canonicalize_name(dep)
            yield "[{}]".format(extra_name), dep, packages_markers.get(dep)


def to_lock_file(poetry_lock):
    """Convert a poetry.lock to a Molt lock file.

    `poetry_lock` should be an instance returned by `load()`. Returns an
    instance of `molt.locks.LockFile`.
    """
    hashes = {
        canonicalize_name(k): sorted("sha256:{}".format(h) for h in v)
        for k, v in poetry_lock["metadata"]["hashes"].items()
        if v  # Poetry produces an empty list for non-hash-required packages.
    }

    sources = {}
    dependencies = {}
    aliases = {}

    # Generate sources and packages information in depenency entries.
    for key, result, src in _generate_packages(poetry_lock):
        if src is not None:
            if src.name in sources and sources[src.name]["url"] != src.url:
                raise SourceNameDuplicated(src.name)
            sources[src.name] = {"url": src.url}

        # If there are no duplicates, good, insert by the package name.
        if key not in aliases:
            aliases[key] = []
            dependencies[key] = {"python": result}
            continue

        # If there is an duplicate, move the previous entry to an alias.
        alias1 = "{}@{}".format(key, len(aliases[key]))
        dependencies[alias1] = dependencies.pop(key)
        aliases[key].append(alias1)

        # And record this new entry with another alias.
        alias2 = "{}@{}".format(key, len(aliases[key]))
        dependencies[alias2] = {"python": result}
        aliases[key].append(alias2)

        # Move the hashes too.
        hashes[alias1] = hashes[alias2] = hashes.pop(key)

    # Link the dependencies of each entry.
    for dependent, depended, marker in _generate_dependencies(poetry_lock):
        if dependent not in dependencies:
            dependencies[dependent] = {"dependencies": {}}
        elif "dependencies" not in dependencies[dependent]:
            dependencies[dependent]["dependencies"] = {}
        markers = [marker] if marker else None
        for k in aliases.get(depended) or [depended]:
            dependencies[dependent]["dependencies"][k] = markers

    data = {"sources": sources, "dependencies": dependencies, "hashes": hashes}

    return LockFile(data)


def is_accounted_for(poetry_lock, lock):
    """Whether a lock file accounts for all information in given poetry.lock.

    It is too involved to compare a poetry.lock directly, so we take the easy
    way out: generate a new Molt lock, and compare it with the existing. Maybe
    we can improve this in the future.
    """
    new_lock = to_lock_file(poetry_lock)

    for key, src in new_lock.sources.items():
        try:
            source = lock.sources[key]
        except KeyError:
            return False
        if source.url != src.url:
            return False

    for key, dep in new_lock.dependencies.items():
        try:
            dependency = lock.dependencies[key]
        except KeyError:
            return False
        if dep.python != dependency.python:
            return False
        if dep.dependencies != dependency.dependencies:
            return False

    for key, hs in new_lock.hashes.items():
        try:
            hashes = lock.hashes[key]
        except KeyError:
            return False
        if not (set(hs) <= set(hashes)):
            return False

    return True
