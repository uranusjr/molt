import json

import plette.models
import six


class _JSONEncoder(json.JSONEncoder):
    """A specilized JSON encoder to convert loaded data into a lock file.

    This adds a few characteristics to the encoder:

    * The JSON is always prettified with indents and spaces.
    * The output is always UTF-8-encoded text, never binary, even on Python 2.
    """
    def __init__(self):
        super(_JSONEncoder, self).__init__(
            ensure_ascii=True,
            indent=4,
            separators=(",", ": "),
            sort_keys=True,
        )

    def encode(self, obj):
        content = super(_JSONEncoder, self).encode(obj)
        if not isinstance(content, six.text_type):
            content = content.decode("utf-8")
        content += "\n"
        return content

    def iterencode(self, obj):
        for chunk in super(_JSONEncoder, self).iterencode(obj):
            if not isinstance(chunk, six.text_type):
                chunk = chunk.decode("utf-8")
            yield chunk
        yield "\n"


class Source(plette.models.DataView):
    __SCHEMA__ = {
        "url": {"type": "string", "required": True},
        "no_verify_ssl": {"type": "boolean"},
    }


class Sources(plette.models.DataViewMapping):
    item_class = Source


class PythonPackage(plette.models.DataView):
    __SCHEMA__ = {
        "name": {"type": "string", "required": True},
        "version": {"type": "string", "excludes": ["url"], "required": True},
        "url": {"type": "string", "excludes": ["version"], "required": True},
        "source": {"type": "string", "nullable": True},
    }


class Marker(plette.models.DataView):
    __SCHEMA__ = {
        "__marker__": {
            "type": "list",
            "minlength": 1,
            "maxlength": 1,
            "schema": {"type": "string", "required": True},
        },
    }

    @classmethod
    def validate(cls, data):
        super(Marker, cls).validate({"__marker__": list(data.items())})


class DependencyMarkers(plette.models.DataViewSequence):
    item_class = Marker


class DependencyDependencies(plette.models.DataViewMapping):
    item_class = DependencyMarkers


class Dependency(plette.models.DataView):
    __SCHEMA__ = {
        "python": {"type": "dict"},
        "dependencies": {"type": "dict"},
    }


class Dependencies(plette.models.DataViewMapping):
    item_class = Dependency


class DependencyHashes(plette.models.DataViewSequence):
    item_class = plette.models.Hash


class Hashes(plette.models.DataViewMapping):
    item_class = DependencyHashes


class LockFile(plette.models.DataView):
    __SCHEMA__ = {
        "sources": {"type": "dict"},
        "dependencies": {"type": "dict"},
        "hashes": {"type": "dict"},
    }

    @classmethod
    def validate(cls, data):
        super(LockFile, cls).validate(data)
        Sources.validate(data.get("sources", {}))
        Dependencies.validate(data.get("dependencies", {}))
        Hashes.validate(data.get("hashes", {}))

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
