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
        import typing   # noqa
    except ImportError:
        sys.modules["typing"] = _MockTyping()


def _patch_functools():
    """Inject lru_cache into functools for TOMLKit.

    TOMLKit detects functools32 by default, but that is not vendorable, so
    we improvise.
    """
    try:
        from functools import lru_cache     # noqa
    except ImportError:
        from backports import functools_lru_cache
        sys.modules["functools32"] = functools_lru_cache


def _patch_enum():
    """Consolidate enum and enum34 packages.

    The enum34 package is vendored as "enum34" (instead of "enum"), and we want
    to make it available when enum is not available from stdlib.
    """
    try:
        import enum     # noqa
    except ImportError:
        import enum34
        sys.modules["enum"] = enum34


def patch():
    _patch_typing()
    _patch_functools()
    _patch_enum()
