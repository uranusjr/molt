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


def patch():
    _patch_typing()
    _patch_functools()
