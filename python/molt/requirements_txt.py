import collections
import warnings

from packaging.utils import canonicalize_name
from requirements import parse as parse_requirements

from .locks import LockFile


class NoIndexIgnored(UserWarning):
    def __init__(self):
        super(NoIndexIgnored, self).__init__("--no-index option ignored")


class RequirementsTxt(object):
    def __init__(self, text, filename, encoding):
        self._text = text
        self._filename = filename
        self._encoding = encoding

    @classmethod
    def load(cls, f, encoding=None):
        text = f.read()
        if encoding is None:
            encoding = f.encoding
        else:
            text = text.decode(encoding)
        return cls(text, "", encoding)

    def iter_requirements(self):
        """Iterate through requirement objects found.
        """
        for r in parse_requirements(self._text):
            yield r


class RequirementsTxtError(Exception):
    pass


class RequirementNotLocked(RequirementsTxtError, ValueError):
    pass


class RequirementNotNamed(RequirementsTxtError, ValueError):
    pass


class _EditablePackage(ValueError):
    pass


class EditablePackageDropped(UserWarning):
    def __init__(self, req):
        super(EditablePackageDropped, self).__init__(
            "Editable package {!r} dropped".format(req.line)
        )
        self.requirement = req


class VCSPackageNotEditable(UserWarning):
    def __init__(self, req):
        super(VCSPackageNotEditable, self).__init__(
            "VCS package {!r} converted to non-editable".format(req.line)
        )
        self.requirement = req


class FindLinksDropped(UserWarning):
    def __init__(self, source):
        super(FindLinksDropped, self).__init__(
            "find-links {!r} dropped".format(source.url)
        )
        self.source = source


def _is_specifier_locked(req):
    # Is === possible here?
    return len(req.specs) == 1 and req.specs[0][0].startswith("==")


def _parse_spec(req):
    if req.vcs is not None:
        if req.editable:
            warnings.warn(VCSPackageNotEditable(req))
        # This does not prevent the user from supplying a mutable VCS rev, e.g.
        # a Git branch, but there's no way to reliably identify those (without
        # raising false positives). Assume good intention.
        if req.revision is None:
            raise RequirementNotLocked(req)
        return {"vcs": req.uri, "rev": req.revision}
    if req.editable:
        raise _EditablePackage(req)
    if req.uri:
        return {"url": req.uri}
    if req.path:
        return {"path": req.path}
    if not _is_specifier_locked(req):
        raise RequirementNotLocked(req)
    return {"version": req.specs[0][1]}


def to_lock_file(requirements_txt):
    hashes = collections.defaultdict(set)
    dependencies = {}
    default_deps = set()

    for req in requirements_txt.iter_requirements():
        if not req.name:
            raise RequirementNotNamed(req)
        result = {"name": req.name}
        try:
            spec = _parse_spec(req)
        except _EditablePackage:
            warnings.warn(EditablePackageDropped(req))
            continue
        result.update(spec)

        # TODO: Warn if there are conflicting requirements.
        key = canonicalize_name(req.name)
        dependencies[key] = {"python": result}
        default_deps.add(key)

        # requirements-parser only supports hash in URL, not --hash.
        if req.hash_name and req.hash:
            hashes[key].add("{}:{}".format(req.hash_name, req.hash))

    dependencies[""] = {"dependencies": sorted(default_deps)}

    # TODO: Support index-urls, trusted-host, etc. The host thing is
    # particularly difficult because we need to also trust URLs in package
    # specifications :( Also, how does scoping work with these options?

    data = {
        "dependencies": dependencies,
        "hashes": {k: list(v) for k, v in hashes.items()},
    }

    return LockFile(data)
