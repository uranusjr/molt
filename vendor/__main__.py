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
    "__pycache__/",
    "bin/",
    "Scripts/",
    "*.dist-info/",
    "**/__pycache__/",
    "**/*.py[co]",
]


def _populate(root, requirements_txt):
    env = os.environ.copy()
    env.update({
        "PIP_NO_COLOR": "false",
        "PIP_NO_COMPILE": "false",
        "PIP_PROGRESS_BAR": "off",
        "PIP_REQUIRE_VIRTUALENV": "false",
    })
    subprocess.check_call([
        sys.executable, "-m", "pip", "install",
        "--disable-pip-version-check",
        "--target", root,
        "--requirement", requirements_txt,
        "--no-deps",
        "--upgrade",
    ], env=env)
    for entry in BLACKLIST_PATTERNS:
        for path in glob.glob(os.path.join(root, entry)):
            _remove(path)


def _populate_molt(src, root):
    if not os.path.exists(root):
        os.makedirs(root)
    shutil.copytree(os.path.join(src, "molt"), os.path.join(root, "molt"))


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
    project_root = os.path.abspath(os.path.join(__file__, "..", ".."))
    target_root = os.path.join(project_root, "target", "assets")

    pattern = os.path.join(os.path.dirname(__file__), "*.txt")
    for requirements_txt in glob.glob(pattern):
        if not os.path.isfile(requirements_txt):
            continue
        child_name = os.path.splitext(os.path.basename(requirements_txt))[0]
        p = os.path.join(target_root, child_name)
        if not os.path.exists(p):
            os.makedirs(p)
        _populate(p, requirements_txt)
    _populate_molt(
        os.path.join(project_root, "python"),
        os.path.join(target_root, "molt"),
    )
    _populate_pep425(os.path.join(target_root, "pep425"))


if __name__ == '__main__':
    main()
