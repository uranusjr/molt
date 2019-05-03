# Molt: Dependency and environment manager for Python projects

[![Build Status](https://travis-ci.com/uranusjr/molt.svg?branch=master)](https://travis-ci.com/uranusjr/molt)

Work in progress.


## Specify a Python

All subcommands take a `--py` flag *before* it to specify the Python to use.
The Python command can be:

* Prefixed by a dash (e.g. `-3.6`). This is passed directly to the Python
  launcher (`py` command) if available.
* Treated as a path if containing path separators.
* A command to be looked up in PATH.

Examples:

```bash
# Runs "init" by invoking `py -2` as Python.
molt --py -2 init

# Runs "run" by invoking the specified executable.
molt --py /usr/bin/python run

# Runs "init" by looking up `python3.6` in PATH.
molt --py python3.6 init
```


## Subcommands

### `molt init`

Creates a Python environment for the project. An environment is created in
`./__pypackages__/<compatibility-tag>`.

The compatibility tag is used to identify Python package compatibility, based
on [PEP 425]. The strictest compatibility tag is used to identify a platform.
The value of which for a given Python interpreter can be unambiguously
determined by invoking Brett Cannon’s [pep425] tool:

```python
import pep425
print(next(pep425.sys_tags()))
```

[PEP 425]: https://www.python.org/dev/peps/pep-0425/
[pep425]: https://github.com/brettcannon/pep425


### `molt install`

Install packages into the environment from `molt.lock.json`.


### `molt run`

`molt run <command>` executes a command available in the environment. This is
NOT a regular executable call; Molt manages commands during package
installation, and resolve them at runtime to emulate Setuptools and pip’s
entry point scripts.

Note that only commands installed via entry points work with `molt run`.


### `molt py`

Access the base interpreter. For example, `molt --py=python3.6 py myscript.py`
would use interpreter `python3.6` to execute file `myscript.py`.


### `molt lock`

Generate `molt.lock.json` from the manidest.


### `molt latest`

List latest version (globally and compatible) of a package on the package
index. Useful if you want to add a pinned package to the manifest, but don’t
immediately know what version to use.


## Manifest

TODO.


## Lock file

See [design/lock-file.md](./design/lock-file.md).


## Interoperability

### Other project management tools

`molt install` from `Pipfile.lock`, `poetry.lock`, or `requirements.txt` is
supported. The former two have precedence over `requirements.txt`; behaviour is
undetermined if both are found. This is done by converting them to
`molt.lock.json`, and install from that.

Tip: The auto-generated `molt.lock.json` can be ignored locally by adding it to
the project’s `.git/info/exclude`, so you can use Molt to develop Pipenv or
Poetry projects without converting wholesale.

Locking into those files is not supported. You’ll need to use the respective
tool to generate a new lock file.


## Try it out

Requires Cargo, and a Python interpreter with `pip` available. The Python
interpreter should be either available in PATH, discoverable by `py`, or set in
the environment variable `MOLT_BUILD_PYTHON`.

```
cargo build --release
```

This gives you a binary in `target/release`. Copy it somewhere in your PATH.


## Run lints/tests

Rust requirements:

* [Clippy]

```
cargo clippy
cargo test
```

Python requirements:

* [Flake8]
* [Black]
* [Tox]

```
flake8 python vendor
black python vendor
tox
```

[Clippy]: https://github.com/rust-lang/rust-clippy
[Flake8]: http://flake8.pycqa.org/en/latest/
[Black]: https://github.com/python/black
[Tox]: https://tox.readthedocs.io/en/latest/
