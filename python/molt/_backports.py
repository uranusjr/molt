import sys


class _MockTyping(object):
    def __getattr__(self, _):
        return None


def _patch_typing():
    """Replace typing with a stub for TOMLKit.

    This supplies a minimal-effort stub, since TOMLKit does not use it for
    anything at runtime.
    """
    try:
        import typing  # noqa
    except ImportError:
        sys.modules["typing"] = _MockTyping()


def _patch_functools():
    """Inject lru_cache into functools for TOMLKit and jsonschema.

    They always use `functools32.lru_cache` on Python 2 (instead of using
    feature detection [sign]). functools32 is not vendorable, so we improvise.
    """
    try:
        from functools import lru_cache  # noqa
    except ImportError:
        from backports import functools_lru_cache

        sys.modules["functools32"] = functools_lru_cache


def _patch_enum():
    """Consolidate enum and enum34 packages.

    The enum34 package is vendored as "enum34" (instead of "enum"), and we want
    to make it available when enum is not available from stdlib.
    """
    try:
        import enum  # noqa
    except ImportError:
        import enum34

        sys.modules["enum"] = enum34


def _patch_pkg_resources():
    """jsonschema wants to read its installation record, but we don't have it.

    Patch `pkg_resources.get_distribution()` to return a fake record.
    """
    import pkg_resources

    pkg_resources_get_distribution = pkg_resources.get_distribution

    def get_distribution(*args, **kwargs):
        if args[0] == "jsonschema":
            return pkg_resources.Distribution(version="")
        return pkg_resources_get_distribution(*args, **kwargs)

    pkg_resources.get_distribution = get_distribution


def patch():
    _patch_typing()
    _patch_functools()
    _patch_enum()
    _patch_pkg_resources()
