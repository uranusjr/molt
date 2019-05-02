import pytest

import io
import json
import os

import molt.foreign.poetry_lock

from _testcommons import SAMPLES_ROOT


@pytest.mark.parametrize(
    "example_name",
    [
        "poetry",
    ],
)
def test_to_lock_file(example_name,):
    poetry_lock_path = os.path.join(
        SAMPLES_ROOT,
        example_name,
        "poetry.lock",
    )
    with io.open(poetry_lock_path, encoding="utf-8") as f:
        poetry_lock = molt.poetry_lock.load(f)

    lock = molt.poetry_lock.to_lock_file(poetry_lock)

    molt_lock_path = os.path.join(SAMPLES_ROOT, example_name, "molt.lock.json")
    with io.open(molt_lock_path, encoding="utf-8") as f:
        assert lock._data == json.load(f)
