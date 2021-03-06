# The Lock File

The structure of the lock file is defined in the accompanying [JSON schema]
file.

[JSON schema]: ../python/molt/locks.schema.json

## Design

### File format and name

The file name must ends in `.lock.json`. The extension is chosen to be both
semantically readable, and easy to identify for a tool (such as an editor to
correctly apply syntax highlighting).

The format is JSON. A tool generating the lock file should normalize the output
to make it human-readable and easy to edit. If using Python, the preferred
configuration to output (using the built-in `json` module) is:

```python
import json

json.dumps(
    ...,
    ensure_ascii=True,
    indent=4,
    separators=(',', ': '),
    sort_keys=True,
)
```

### Top-level fields

There are three top-level fields:

* *dependencies* contains a mapping of dependencies to use. A dependency may
  be a real package, or a “virtual” dependency that describes a set of
  dependencies.
* *sources* specifies places to find packages. Each dependency may specify
  one or more specific sources to fetch artifacts from, or leave this for the
  tool to decide.
* *hashes* is a mapping of hashes used to check the integrity of downloaded
  packages.

Implementers can store their tool-specific data in fields prefixed with an
underscore (`_`). To avoid conflicts, each tool should only use one field name.
For example, Molt stores its data in a fields named `_molt`.

#### `sources`

This section holds a mapping of sources used to find packages. Only the
“Simple” repository API defined in [PEP 503] is supported at the current
time. (This may change in the future to include other formats, e.g. Conda
channels.)

[PEP 503]: https://www.python.org/dev/peps/pep-0503/

Each source entry should contain a key, URL, that points to the API’s root URL,
e.g. `https://pypi.org/simple` for PyPI. If the optional key `no_verify_ssl`
is specified as true, SSL errors are ignored when accessing the API (the same
as supplying `--trusted-host` to pip).


#### `dependencies`

This section describes a directed graph that represents the dependency tree.
Each key in the mapping uniquely identifies a node in the graph.[1]

[1]: The identifier does *not* represent the name of a Python package (although
     the package name could be a suitable identifier). See below if you’re
     interested in how Molt decides what keys to use.

Each dependency entry may contain one or both of the following keys:

* *dependencies* is an object to list other dependency entries this one depends
  on. Each key should be a key in the top-level *dependencies* object. The
  value should be a list of markers, meaning that the dependency should be
  activated if any of the markers evaluate to true. Specify `null` to activate
  the dependency unconditionally. (Note: No marker merging is done here; see
  discussion below for reasoning.)
* *python*, if specified, is an object specifying a concrete package to
  install. This object must contain one key, *name* to specify the package, and
  other keys to specify how the package should be found.

(The *python* key should specify a package using the Python package format,
such those available on PyPI. This may be extended in the future to include
other package sources like Conda.)

##### Specify a Python package to find

A Python package can contain exactly one of the following keys to specify how
it is found:

* If `version` is present, this is a *named requirement*, and `version`
  specifies the version to look for. An optional key `source` may be used to
  specify where the package should be looked for. The value of `source`, if a
  string, should be the key to an entry in the top-level `sources` section. If
  `sources` is null or left out, the tool should decide how and where the
  package is fetched (e.g. consulting [pip configurations]).
* If `url` is present, its value is used to download the package. If the
  optional key `no_verify_ssl` is specified as true, SSL errors are ignored
  when downloading the package (the same as supplying `--trusted-host` to pip).
* If `path` is present, its value should be a path relative to the directory
  containing the lock file. The file located at the path will be used as the
  package.
* If `vcs` is present, the value should be a VCS URL, as specified by
  [pip VCS support]. An additional key `rev` is required that points to an
  exact revision of the VCS repository, e.g. a Git commit.

[pip configurations]: https://pip.pypa.io/en/stable/user_guide/#config-file
[pip VCS support]: https://pip.pypa.io/en/stable/reference/pip_install/#vcs-support


#### `hashes`

This section holds a mapping of hashes for each dependency entry. If provided,
each key should point to a key in `dependencies`; the value should be an array
of hashes, used to verify that the artifact fetched on install time matches
one of those expected at resolve time.

Each hash entry should be a string of the following form:

    <hashname>:<hashvalue>

where `<hashname>` and `<hashvalue>` follow the definition in [PEP 503]:
`<hashname>` is the lowercase name of the hash function (such as
`sha256`), and `<hashvalue>` is the hex encoded digest.

If a dependency’s key is missing from the hashes mapping, any artifact
downloaded to satify it is assumed to be valid.

### Discussions

#### File format

The JSON format is chosen because it is ubiquitous across platforms and
languages. It is more difficult to generate by hand, but this is not as
important since it is very unlikely to be the case. It is also possible to
edit the tool-generated output, since it is formatted.

TOML is considered, but ultimately rejected since the output is too volitile.
The same data structure can generate many equivalent TOML outputs, and it makes
interoperability more difficult. Lock files generated by different tools would
potentially have very different contents, even if the underlying structure is
similar, even identical. Normalized JSON makes it easier for a human to
understand how much has changed by simply inspecting diff.

#### Marker merging

Marker merging refers to the action taken when a conditional dependency has at
least one conditional dependency. For example, say dependency A is required
only on `python_version < '3.5'`, and A depends on B when
`python_version < '3.4'`. With marker merging, B’s marker in the dependency
graph would become `python_version < '3.4' and python_version < '3.4'`. The
proposed lock file format DOES NOT expect tools to do this.

The most important benefit of marker merging is during installation. Since
each marker is of its final form, the implementation would be able to determine
whether a dependency is required merely by looking at the entry itself.
Othereise it needs to reconstruct a full dependency graph must be, and
traverses all paths to the representing node to find out. Marker merging is
proved to be problematic in practice, unfortunately, since some low-level
packages tend to have a lot of paths pointing to them, and the resulting
marker becomes quite unwieldy to both record and decipher.

By not merging markers when the lock file is generated. Marker evaluation is
pushed to installation time. While it would be more work to reconstruct the
graph, it is usually what tools want to do anyway (to order the packages for
installation).

#### Out-of-line hash definition

Unlike Pipfile.lock the hashes are defined in their own mapping, instead of
embedded inside `dependencies`. This decision is made to improve readability
since both the list of hashes and hashes themselves are not usually
consumable for humans, and tend to be quite long, making the dependency list
difficult to read on smaller screens.

#### Out-of-line source definition

Unlike poetry.lock, the sources are defined in their own mapping, instead of
embedded inside `dependencies`. This decision is made to improve readability
since the source definition can be repetitive, and not easily identifiable for
humans if the same source occurs multiple times in a lock file.

## How Molt specifies Python packages

(Note: For brevity, examples given in this section only show partial content,
and do not use the recommended format.)

While this lock file design does not specify how dependency keys are specified,
Molt uses a rule similar to [PEP 508].

[PEP 508]: https://www.python.org/dev/peps/pep-0508/

For a straightforward Python package dependency, the key is the (canonical)
package name, e.g.

```json
{
    "django": {
        "python": {"name": "Django", "version": "2.2.0"},
        "dependencies": {"pytz": null, "sqlparse": null}
    }
}
```

For a Python package extra, the key is the package name plus extra, e.g.

```json
{
    "requests[socks]": {
        "dependencies": {"requests": null, "pysocks": null}
    },
    "requests": {
        "python": {"name": "requests", "version": "2.21.0"},
        "dependencies": {
            "certifi": null,
            "chardet": null,
            "idna": null,
            "urllib3": null
        }
    }
}
```

Note that the extra-ed entry does not contain the Requests package itself, but
references an extra-less entry. This enabled the dependency tree to be
unambigiously reconstructed from the lock information.

If a package needs different versions in different environments (e.g. operating
system), a marker is attached to dinstinguish different entries. Personally, I
think it is a terrible idea to have such a dependency tree, but people seems to
expect this feature, and it fits well in the design anyway, so :shrug:

```json
{
    "pyarrow": {
        "dependencies": {
            "pyarrow;sys_platform=='darwin'": ["sys_platform == 'darwin'"],
            "pyarrow;sys_platform!='darwin'": ["sys_platform != 'darwin'"]
        }
    },
    "pyarrow;sys_platform=='darwin'": {
        "python": {"name": "pyarrow", "version": "0.9.0.post1"},
        "dependencies": {"numpy": null, "six": null}
    },
    "pyarrow;sys_platform!='darwin'": {
        "python": {"name": "pyarrow", "version": "0.9.0"},
        "dependencies": {"numpy": null, "six": null}
    }
}
```

An empty key is treated specially to indicate the “top-level” dependencies.
This is useful when you’re developing a Python project without a package name,
e.g. a Django website. This can have extras as well, to supply situational
dependencies like dev-only, CI-only, documentation tools, etc.

```json
{
    "": {"dependencies": {"django": null}},
    "[doc]": {"dependencies": {"sphinx": null}},
    "[test]": {"dependencies": {"": null, "pytest-django": null}},
    "[dev]": {"dependencies": {"[test]": null, "django-debug-toolbar": null}}
}
```

This would

* Install Django and its dependencies when you run `molt install`.
* Install Sphinx and its dependencies (but not Django!) on `molt install doc`.
* Install Django, pytest-django, Sphinx, and their dependencies on
  `molt install doc test`.
* Install Django, pytest-django, django-debug-toolbar, and their dependencies
  on `molt install dev`.

and so on.

