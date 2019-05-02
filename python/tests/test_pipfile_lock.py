import pytest

import io
import json
import os
import warnings

import plette

import molt.foreign.pipfile_lock

from _testcommons import SAMPLES_ROOT


@pytest.mark.parametrize(
    "example_name, editables, vcsreqs",
    [
        ("pipenv", {"pipenv", "pytest-pypi"}, {"passa", "towncrier"}),
        ("virtenv", {"virtenv"}, set()),
    ],
)
def test_to_lock_file(example_name, editables, vcsreqs):
    pipfile_lock_path = os.path.join(
        SAMPLES_ROOT,
        example_name,
        "Pipfile.lock",
    )
    with io.open(pipfile_lock_path, encoding="utf-8") as f:
        pipfile_lock = plette.Lockfile.load(f)

    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        lock = molt.foreign.pipfile_lock.to_lock_file(pipfile_lock)

        assert len(w) == (len(editables) + len(vcsreqs))

        assert editables == {
            m.message.package_name for m in w
            if m.category == molt.foreign.pipfile_lock.EditablePackageDropped
        }
        assert vcsreqs == {
            m.message.package_name for m in w
            if m.category == molt.foreign.pipfile_lock.VCSPackageNotEditable
        }

    molt_lock_path = os.path.join(SAMPLES_ROOT, example_name, "molt.lock.json")
    with io.open(molt_lock_path, encoding="utf-8") as f:
        assert lock._data == json.load(f)
