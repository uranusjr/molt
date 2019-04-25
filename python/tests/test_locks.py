# import pytest

# Ensure Cerberus is available.
import cerberus     # noqa

from molt import locks


def test_source():
    locks.Source({"url": "https://pypi.org/simple"})
