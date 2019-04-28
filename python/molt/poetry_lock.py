import collections
import warnings

import tomlkit

from packaging.utils import canonicalize_name

from .locks import LockFile


class PoetryLockError(Exception):
    pass


class PackageSpecifierNotSupported(PoetryLockError, ValueError):
    pass


class SourceDropped(UserWarning):
    def __init__(self, package_name):
        super(SourceDropped, self).__init__(
            "Source dropped for package {!r} (invalid in this context)".format(
                package_name,
            )
        )
        self.package_name = package_name


class DuplicateSourceDropped(UserWarning):
    def __init__(self, source, dropping_url):
        super(DuplicateSourceDropped, self).__init__(
            "Source URL {!r} dropped (duplicate name {!r} to {!r})".format(
                dropping_url, source.name, source.url,
            )
        )
        self.source = source
        self.dropping_url = dropping_url


def parse(f, encoding=None):
    """Parse a poetry.lock file.

    If `encoding` is specified, `f` is treated as binary; if `encoding` is
    not specified or `None`, `f` should be opened in text mode.
    """
    text = f.read()
    if encoding is not None:
        text = f.read().decode(encoding)
    # Yes, this simply returns a dict. I guess it is enough since we only want
    # to convert it to molt.lock.json anyway?
    return tomlkit.parse(text)


Package = collections.namedtuple("Package", ["name", "version"])

Source = collections.namedtuple("Source", ["name", "url"])


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
            "ref": source["reference"],
        }

    try:
        version = package["version"]
    except KeyError:
        raise PackageSpecifierNotSupported(package)
    return {"version": version}


def _parse_source(package):
    try:
        source = package["source"]
    except KeyError:
        return None
    if source["type"] != "legacy":
        return None
    return Source(source["reference"], source["url"])


def _generate_packages(package_data_list):
    for package_data in package_data_list:
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

        yield (
            canonicalize_name(name),
            result,
            package_data.get("marker"),
            source,
        )


def to_lock_file(poetry_lock):
    hashes = {
        canonicalize_name(k): sorted("sha256:{}".format(h) for h in v)
        for k, v in poetry_lock["metadata"]["hashes"].items()
        if v    # Poetry produces an empty list for non-hash-required packages.
    }

    sources = {}
    dependencies = {}

    poetry_packages = poetry_lock["package"]
    for key, res, marker, src in _generate_packages(poetry_packages):
        if src is not None:
            if src.name in sources and sources[src.name]["url"] != src.url:
                warnings.warn(DuplicateSourceDropped(
                    src, sources[src.name].url,
                ))
            sources[src.name] = {"url": src.url}
        dependencies[key] = res

    # TODO: Resolve dependency graph.

    data = {
        "sources": sources,
        "dependencies": dependencies,
        "hashes": hashes,
    }

    return LockFile(data)
