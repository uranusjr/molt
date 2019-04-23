import glob
import os
import shutil
import subprocess
import sys
try:
    import urllib.request as urllib_request
except ImportError:
    import urllib as urllib_request
import zipfile


def _remove(p):
    if os.path.isdir(p):
        shutil.rmtree(p)
    else:
        os.unlink(p)


BLACKLIST_PATTERNS = [
    "bin/",
    "Scripts/",
    "*.dist-info/",
    "**/__pycache__/",
    "**/*.py[co]",
]


def _populate(root, requirements_txt):
    subprocess.check_call([
        sys.executable, "-m", "pip", "install",
        "--disable-pip-version-check",
        "--target", root,
        "--requirement", requirements_txt,
        "--no-color",
        "--no-compile",
        "--no-deps",
        "--progress-bar=off",
        "--upgrade",
    ], env={"PIP_REQUIRE_VIRTUALENV": "false"})
    for entry in BLACKLIST_PATTERNS:
        for path in glob.glob(os.path.join(root, entry)):
            _remove(path)


def _populate_pep425(root):
    if not os.path.exists(root):
        os.makedirs(root)
    fn, _ = urllib_request.urlretrieve(
        "https://github.com/brettcannon/pep425/archive/master.zip",
    )
    with zipfile.ZipFile(fn) as zf:
        data = zf.read("pep425-master/pep425.py")
        with open(os.path.join(root, "pep425.py"), "wb") as f:
            f.write(data)
    os.unlink(fn)


def main():
    assets_root = os.path.dirname(__file__)
    pattern = glob.glob(os.path.join(assets_root, "requirements-*.txt"))
    for requirements_txt in pattern:
        if not os.path.isfile(requirements_txt):
            continue
        child_name = os.path.splitext(
            os.path.basename(requirements_txt),
        )[0].split("-", 1)[1]
        p = os.path.join(assets_root, child_name)
        if not os.path.exists(p):
            os.makedirs(p)
        _populate(p, requirements_txt)
    _populate_pep425(os.path.join(assets_root, "pep425"))


if __name__ == '__main__':
    main()
