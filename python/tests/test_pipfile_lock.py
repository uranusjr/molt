import pytest

import io
import json
import os
import warnings

import plette

import molt.pipfile_lock

from _testcommons import SAMPLES_ROOT


@pytest.mark.parametrize(
    "example_name, editables",
    [
        ("pipenv", {"passa", "pipenv", "pytest-pypi", "towncrier"}),
        ("virtenv", {"virtenv"}),
    ],
)
def test_to_lock_file(example_name, editables):
    pipfile_lock_path = os.path.join(
        SAMPLES_ROOT,
        example_name,
        "Pipfile.lock",
    )
    with io.open(pipfile_lock_path, encoding="utf-8") as f:
        pipfile_lock = plette.Lockfile.load(f)

    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter(
            "always", molt.pipfile_lock.EditablePackageDropped,
        )
        lock = molt.pipfile_lock.to_lock_file(pipfile_lock)

        assert {m.message.package_name for m in w} == editables

    molt_lock_path = os.path.join(SAMPLES_ROOT, example_name, "molt.lock.json")
    with io.open(molt_lock_path, encoding="utf-8") as f:
        assert lock._data == json.load(f)
