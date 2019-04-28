import io
import json
import os

import jsonschema
import pytest

import molt.locks

from _testcommons import SAMPLES_ROOT


def test_source():
    molt.locks.Source({"url": "https://pypi.org/simple"})


def test_source_invalid():
    with pytest.raises(jsonschema.ValidationError):
        molt.locks.Source({})


@pytest.mark.parametrize(
    "data",
    [
        {"name": "pip", "version": "19.1"},
        {"name": "pip", "version": "19.1", "source": "private"},

        {"name": "pip", "url": "https://mydomain.localhost/pip-19.1.tar.gz"},
        {
            "name": "pip",
            "url": "https://mydomain.localhost/pip-19.1.tar.gz",
            "no_verify_ssl": True,
        },

        {"name": "pip", "path": "../../pip-19.1.tar.gz"},

        {
            "name": "pip",
            "vcs": "git+https://github.com/pypa/pip.git",
            "ref": "8d0b73fc5b289c0347d7261e5efeeb40a8470382",
        },
    ],
)
def test_python_package(data):
    molt.locks.PythonPackage(data)


@pytest.mark.parametrize(
    "data",
    [
        # Missing specifier.
        {"name": "pip"},

        # Missing version.
        {"name": "pip", "source": "private"},

        # Missing VCS ref.
        {"name": "pip", "vcs": "git+https://github.com/pypa/pip.git"},
    ],
)
def test_python_package_invalid(data):
    with pytest.raises(jsonschema.ValidationError):
        molt.locks.Source(data)


@pytest.mark.parametrize(
    "example_name",
    [
        "pipenv",
        "virtenv",
    ],
)
def test_validate_sample_lock_files(example_name):
    lock_path = os.path.join(SAMPLES_ROOT, example_name, "molt.lock.json")
    with io.open(lock_path, encoding="utf-8") as f:
        molt.locks.LockFile.validate(json.load(f))
