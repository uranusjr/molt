import collections
import warnings

from packaging.utils import canonicalize_name

from molt.locks import LockFile


SUPPORTED_SPECS = {6}


class PipfileLockError(Exception):
    pass


class PipfileSpecNotSupported(PipfileLockError, ValueError):
    pass


class InvalidVersion(PipfileLockError, ValueError):
    pass


class _EditablePackage(ValueError):
    pass


class PackageSpecifierNotSupported(PipfileLockError, ValueError):
    pass


class PipfileLockWarning(UserWarning):
    pass


class EditablePackageDropped(PipfileLockWarning):
    def __init__(self, name):
        super(EditablePackageDropped, self).__init__(
            "Editable package {!r} dropped".format(name)
        )
        self.package_name = name


class VCSPackageNotEditable(PipfileLockWarning):
    def __init__(self, name):
        super(VCSPackageNotEditable, self).__init__(
            "VCS package {!r} converted as non-editable".format(name)
        )
        self.package_name = name


def _parse_vcs_info(package):
    try:
        rev = package.ref
    except AttributeError:
        return None
    for vcs in ["git", "hg", "bzr", "svn"]:
        try:
            url = getattr(package, vcs)
        except AttributeError:
            continue
        else:
            return vcs, url, rev
    return None


def _parse_spec(name, package):
    editable = getattr(package, "editable", False)

    vcs_info = _parse_vcs_info(package)
    if vcs_info is not None:
        # Keep editable VCS (but drop editable flag) because people generally
        # specify it to work around a pip bug and force dependency resolution.
        # The editable property itself likely does not matter.
        if editable:
            warnings.warn(VCSPackageNotEditable(name))
        vcs, url, rev = vcs_info
        return {"vcs": "{}+{}".format(vcs, url), "rev": rev}

    # Other than VCS, people generally specify editable to get its specific
    # behavior. We can't support that yet.
    if editable:
        raise _EditablePackage(package)

    try:
        v = package.url
    except AttributeError:
        pass
    else:
        return {"url": v}

    try:
        v = package.path
    except AttributeError:
        pass
    else:
        return {"path": v}

    # This is tried last because Pipenv liberally stick versions into other
    # kinds of requirement specifications.
    try:
        v = package.version
    except AttributeError:
        pass
    else:
        if not v.startswith("=="):
            raise InvalidVersion(v)
        return {"version": v.lstrip("=")}  # Is === possible here?

    raise PackageSpecifierNotSupported(package._data)


def _generate_packages(section):
    for key, package in section.items():
        result = {"name": key}

        try:
            spec = _parse_spec(key, package)
        except _EditablePackage:
            warnings.warn(EditablePackageDropped(key))
            continue
        result.update(spec)

        # TODO: Validate this against the source mapping?
        try:
            result["source"] = package.index
        except AttributeError:
            pass

        yield (
            canonicalize_name(key),
            result,
            package.get("markers"),
            package.get("hashes"),
        )


def _add_dependency(parent, child, marker):
    curr_marker = parent.get(child)
    if curr_marker is None:
        parent[child] = None if marker is None else [marker]
    elif marker is None:
        parent[child] = None
    else:
        parent[child] = [marker]


def _generate_sources(sources):
    for source in sources:
        result = {"url": source.url}
        if not source.verify_ssl:
            result["no_verify_ssl"] = True
        yield source["name"], result


def to_lock_file(pfl):
    """Convert a Pipfile.lock to a Molt lock file.

    `pfl` should be a `plette.Lockfile` instance. Returns an instance of
    `molt.locks.LockFile`.
    """
    if pfl.meta.pipfile_spec not in SUPPORTED_SPECS:
        raise PipfileSpecNotSupported(pfl.meta.pipfile_spec)

    hashes = collections.defaultdict(set)

    dependencies = {}
    default = {}
    develop = {}

    for key, result, marker, package_hashes in _generate_packages(pfl.develop):
        if package_hashes is not None:
            hashes[key].update(package_hashes)
        dependencies[key] = {"python": result}
        _add_dependency(develop, key, marker)
    for key, result, marker, package_hashes in _generate_packages(pfl.default):
        if package_hashes is not None:
            hashes[key].update(package_hashes)
        # TODO: Merge entries with same keys from default and develop?
        dependencies[key] = {"python": result}
        _add_dependency(default, key, marker)

    dependencies[""] = {"dependencies": default}
    dependencies["[dev]"] = {"dependencies": develop}

    data = {
        "sources": dict(_generate_sources(pfl.meta.sources)),
        "dependencies": dependencies,
        "hashes": {k: sorted(v) for k, v in hashes.items()},
    }

    return LockFile(data)


def _is_package_accounted_for(package_info, section, lock):
    key, result, marker, package_hashes = package_info

    # Check the package information match.
    try:
        package = lock.dependencies[key]["python"].copy()
    except KeyError:
        return False
    if canonicalize_name(package.pop("name", None)) != key:
        return False
    result.pop("name", None)
    if package != result:
        return False

    # Check the marker is included.
    try:
        dep_markers = lock.dependencies[section]["dependencies"][key]
    except KeyError:
        return False
    else:
        if marker is not None:
            if marker not in dep_markers:
                return False
        elif dep_markers is not None:
            return False

    # Check all hashes in Pipfile.lock are included.
    if package_hashes:
        try:
            lock_hashes = set(lock.hashes[key])
        except KeyError:
            return False
        if not (set(package_hashes) <= lock_hashes):
            return False

    return True


def is_accounted_for(pfl, lock):
    """Whether a lock file accounts for all information in given Pipfile.lock.
    """
    # Check all sources in Pipfile.lock are accounted for.
    for k, v in _generate_sources(pfl.meta.sources):
        try:
            source = lock.sources[k]
        except KeyError:
            return False
        if source["url"] != v["url"]:
            return False
        if source.get("no_verify_ssl", False) != v.get("no_verify_ssl", False):
            return False

    # Check all packages in Pipfile.lock have equivalents.
    for info in _generate_packages(pfl.default):
        if not _is_package_accounted_for(info, "", lock):
            return False
    for info in _generate_packages(pfl.develop):
        if not _is_package_accounted_for(info, "[dev]", lock):
            return False

    return True
