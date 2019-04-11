import pathlib
import shutil
import subprocess
import sys


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


def main():
    for p in pathlib.Path(__file__).parent.iterdir():
        if p.is_dir():
            _populate(p)


if __name__ == '__main__':
    main()
