# Molt: Dependency and environment manager for Python projects

Work in progress.

## Specify a Python

All subcommands take a `--py=<python-command>` option before the subcommand
that specifies the Python to use. The Python command is looked up in PATH, or
it can be a path (either relative or absolute).

An environment variable `MOLT_PY` can be set to specify the Python command to
use if this is not provided.


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


## Menifest

TODO.


## Lock file

TODO. This would be a more inclusive proposal based on discussions on
[Structured, exchangable lock file format].

[Structured, exchangable lock file format]: https://discuss.python.org/t/structured-exchangeable-lock-file-format-requirements-txt-2-0/876


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
