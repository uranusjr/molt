import pytest

import io
import json
import os

import molt.foreign.poetry_lock

from _testcommons import SAMPLES_ROOT


@pytest.mark.parametrize("example_name", ["poetry"])
def test_to_lock_file(example_name,):
    poetry_lock_path = os.path.join(SAMPLES_ROOT, example_name, "poetry.lock")
    with io.open(poetry_lock_path, encoding="utf-8") as f:
        poetry_lock = molt.foreign.poetry_lock.load(f)

    lock = molt.foreign.poetry_lock.to_lock_file(poetry_lock)

    molt_lock_path = os.path.join(SAMPLES_ROOT, example_name, "molt.lock.json")
    with io.open(molt_lock_path, encoding="utf-8") as f:
        assert lock._data == json.load(f)


@pytest.mark.parametrize("example_name", ["poetry"])
def test_is_accounted_for(example_name):
    example_path = os.path.join(SAMPLES_ROOT, example_name)

    pipfile_lock_path = os.path.join(example_path, "poetry.lock")
    with io.open(pipfile_lock_path, encoding="utf-8") as f:
        poetry_lock = molt.foreign.poetry_lock.load(f)

    lock_file_path = os.path.join(example_path, "molt.lock.json")
    with io.open(lock_file_path, encoding="utf-8") as f:
        lock = molt.locks.LockFile.load(f)

    assert molt.foreign.poetry_lock.is_accounted_for(poetry_lock, lock)
