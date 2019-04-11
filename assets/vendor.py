import pathlib
import shutil
import subprocess
import sys
import urllib.request
import zipfile


def _remove(p):
    if p.is_dir():
        shutil.rmtree(str(p))
    else:
        p.unlink()


BLACKLIST_PATTERNS = [
    "bin",
    "Scripts",
    "**/__pycache__",
]


def _populate(root):
    requirements_txt = root.joinpath("requirements.txt")
    if not requirements_txt.is_file():
        return
    subprocess.check_call([
        sys.executable, "-m", "pip", "install",
        "--disable-pip-version-check",
        "--target", str(root),
        "--requirement", str(requirements_txt),
        "--no-color",
        "--no-compile",
        "--no-deps",
        "--progress-bar=off",
        "--upgrade",
    ])
    for entry in BLACKLIST_PATTERNS:
        for path in root.glob(entry):
            _remove(path)


def _populate_pep425(root):
    fn, _ = urllib.request.urlretrieve(
        "https://github.com/brettcannon/pep425/archive/master.zip",
    )
    with zipfile.ZipFile(fn) as zf:
        data = zf.read("pep425-master/pep425.py")
        root.joinpath("pep425.py").write_bytes(data)
    pathlib.Path(fn).unlink()


def main():
    assets_root = pathlib.Path(__file__).parent
    for p in assets_root.iterdir():
        if p.is_dir():
            _populate(p)
    _populate_pep425(assets_root.joinpath("pep425"))


if __name__ == '__main__':
    main()
