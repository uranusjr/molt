import pathlib
import shutil
import subprocess


VENDOR_DIR = pathlib.Path(__file__).absolute().parent.parent.joinpath("vendor")

REQUIREMENTS_TXT = VENDOR_DIR.joinpath("requirements.txt")


def _remove(p):
    if not p.exists():
        return
    if p.is_dir():
        shutil.rmtree(str(p))
    else:
        p.unlink()


def main():
    subprocess.check_call([
        "pip", "install",
        "--disable-pip-version-check",
        "--target", str(VENDOR_DIR),
        "--requirement", str(REQUIREMENTS_TXT),
        "--no-color",
        "--no-compile",
        "--no-deps",
        "--progress-bar=off",
        "--upgrade",
    ])
    for name in ["virtenv_cli.py", "bin", "Scripts"]:
        _remove(VENDOR_DIR.joinpath(name))


if __name__ == '__main__':
    main()
