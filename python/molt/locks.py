import io
import json
import os

import jsonschema
import plette.models
import six


class _JSONEncoder(json.JSONEncoder):
    """A specilized JSON encoder to convert loaded data into a lock file.

    This adds a few characteristics to the encoder:

    * The JSON is always prettified with indents and spaces.
    * The output is ASCII-only, always text, never binary, even on Python 2.
    """

    def __init__(self):
        super(_JSONEncoder, self).__init__(
            ensure_ascii=True, indent=4, separators=(",", ": "), sort_keys=True
        )

    def encode(self, obj):
        content = super(_JSONEncoder, self).encode(obj)
        if not isinstance(content, six.text_type):
            content = content.decode("ascii")
        content += u"\n"
        return content

    def iterencode(self, obj):
        for chunk in super(_JSONEncoder, self).iterencode(obj):
            if not isinstance(chunk, six.text_type):
                chunk = chunk.decode("ascii")
            yield chunk
        yield u"\n"


def _read_schema():
    p = os.path.abspath(os.path.join(__file__, "..", "locks.schema.json"))
    with io.open(p, encoding="utf-8") as f:
        return json.load(f)


_SCHEMA = _read_schema()

_SOURCE_SCHEMA = next(
    iter(_SCHEMA["properties"]["sources"]["patternProperties"].values())
)

_DEPENDENCY_SCHEMA = next(
    iter(_SCHEMA["properties"]["dependencies"]["patternProperties"].values())
)

_PYTHONPACKAGE_SCHEMA = _DEPENDENCY_SCHEMA["properties"]["python"]


class Source(plette.models.DataView):
    @classmethod
    def validate(cls, data):
        jsonschema.validate(instance=data, schema=_SOURCE_SCHEMA)


class Sources(plette.models.DataViewMapping):
    item_class = Source


class PythonPackage(plette.models.DataView):
    @classmethod
    def validate(cls, data):
        jsonschema.validate(instance=data, schema=_PYTHONPACKAGE_SCHEMA)


class Dependency(plette.models.DataView):
    @classmethod
    def validate(cls, data):
        jsonschema.validate(instance=data, schema=_DEPENDENCY_SCHEMA)


class Dependencies(plette.models.DataViewMapping):
    item_class = Dependency


class LockFile(plette.models.DataView):
    """A Molt format lock file.
    """

    @classmethod
    def load(cls, f, encoding=None):
        """Load a lock file from file.

        If `encoding` is None, `f` should be opened in UTF-8 text mode.

        If `encoding` is set, `f` should be opened in binary mode. The lock
        file will be decoded with the specified encoding.
        """
        if encoding is None:
            return cls(json.load(f))
        return cls(json.loads(f.read().decode(encoding)))

    @classmethod
    def validate(cls, data):
        jsonschema.validate(instance=data, schema=_SCHEMA)

    @property
    def sources(self):
        return Sources(self._data.get("sources", {}))

    @property
    def dependencies(self):
        return Dependencies(self._data.get("dependencies", {}))

    @property
    def hashes(self):
        return self._data.get("hashes", {})

    def dump(self, f, encoding=None):
        """Dump the lock file structure to a file.

        If `encoding` is None, `f` should be opened in UTF-8 text mode.

        If `encoding` is set, `f` should be opened in binary mode. The lock
        file will be encoded in the specified encoding and written to `f`.
        """
        encoder = _JSONEncoder()
        if encoding is None:
            for chunk in encoder.iterencode(self._data):
                f.write(chunk)
        else:
            content = encoder.encode(self._data)
            f.write(content.encode(encoding))
