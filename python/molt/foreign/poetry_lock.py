import collections
import warnings

import tomlkit

from packaging.utils import canonicalize_name

from molt.locks import LockFile


class PoetryLockError(Exception):
    pass


class PackageSpecifierNotSupported(PoetryLockError, ValueError):
    pass


class SourceDropped(UserWarning):
    def __init__(self, package_name):
        super(SourceDropped, self).__init__(
            "Source dropped for package {!r} (invalid in this context)".format(
                package_name
            )
        )
        self.package_name = package_name


class DuplicateSourceDropped(UserWarning):
    def __init__(self, source, dropping_url):
        super(DuplicateSourceDropped, self).__init__(
            "Source URL {!r} dropped (duplicate name {!r} to {!r})".format(
                dropping_url, source.name, source.url
            )
        )
        self.source = source
        self.dropping_url = dropping_url


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


def _generate_dependencies(poetry_lock):
    undepended_packages = {
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
            undepended_packages.pop(dep, None)
            package_name = canonicalize_name(package_data["name"])
            yield package_name, dep, packages_markers.get(dep)

    for package_data in undepended_packages.values():
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

    for key, result, src in _generate_packages(poetry_lock):
        if src is not None:
            if src.name in sources and sources[src.name]["url"] != src.url:
                warnings.warn(
                    DuplicateSourceDropped(src, sources[src.name].url)
                )
            sources[src.name] = {"url": src.url}
        dependencies[key] = {"python": result}

    for dependent, depended, marker in _generate_dependencies(poetry_lock):
        if dependent not in dependencies:
            dependencies[dependent] = {"dependencies": {}}
        elif "dependencies" not in dependencies[dependent]:
            dependencies[dependent]["dependencies"] = {}
        markers = [marker] if marker else None
        dependencies[dependent]["dependencies"][depended] = markers

    data = {"sources": sources, "dependencies": dependencies, "hashes": hashes}

    return LockFile(data)
