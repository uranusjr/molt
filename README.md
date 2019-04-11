# Molt: Dependency and environment manager for Python projects

Work in progress.

## Intended usages

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


### `molt lock`

Generate `molt.lock.json` from the manidest.


### `molt latest`

Lists latest version (globally and compatible) of a package on the package
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

`molt install` from `Pipfile.lock` or `poetry.lock` is supported (behaviour is
undetermined if both are found) by converting them to `molt.lock.json`, and
install from that.

Tip: The auto-generated molt lock file can be ignored locally by adding it to
the project’s `.git/info/exclude`, so you can use Molt to develop Pipenv or
Poetry projects without converting wholesale.

Note that locking into `Pipfile.lock` and `poetry.lock` is not supported.
You’ll need to use those tools to generate a new lock file.

Molt will try to warn you if it finds you’re running Molt commands (other than
`init`, `install`, and `run`) in a Pipenv or Poetry project:

* If a `Pipfile` is found.
* If a `pyproject.toml` is found containing `[tool.poetry]` fields.
