"""FontCarver's trusted local Python bootstrap (stdio JSON lines, protocol v1)."""

import builtins
import io
import json
import sys
import traceback
import types

_input = sys.stdin
_wire = sys.stdout
_message_limit = 256 * 1024
_output_limit = 64 * 1024
_output_bytes = 0


def _send(message):
    encoded = json.dumps(message, ensure_ascii=True, separators=(",", ":"))
    if len(encoded) > _message_limit:
        raise RuntimeError("Python RPC message exceeds 262144 bytes")
    _wire.write(encoded + "\n")
    _wire.flush()


class _Output(io.TextIOBase):
    def __init__(self):
        self.pending = ""

    def write(self, text):
        global _output_bytes
        _output_bytes += len(text.encode("utf-8"))
        if _output_bytes > _output_limit:
            raise RuntimeError("Python output exceeds 65536 bytes")
        self.pending += text
        while "\n" in self.pending:
            line, self.pending = self.pending.split("\n", 1)
            _send({"kind": "output", "text": line})
        return len(text)

    def flush(self):
        if self.pending:
            pending, self.pending = self.pending, ""
            _send({"kind": "output", "text": pending})


def _call(method, params=None):
    _send({"kind": "call", "method": method, "params": params or {}})
    response = _input.readline(_message_limit + 2)
    if not response or len(response) > _message_limit + 1:
        raise RuntimeError("Python RPC response missing or over message limit")
    result = json.loads(response)
    if "error" in result:
        error = result["error"]
        raise RuntimeError(str(error.get("code")) + ": " + str(error.get("message")))
    return result["result"]


_request = json.loads(_input.readline(512 * 1024))
fontcarver = types.ModuleType("fontcarver")
fontcarver.call = _call
fontcarver.context = _request["context"]
sys.modules["fontcarver"] = fontcarver
sys.stdout = _Output()
sys.stderr = sys.stdout
try:
    exec(compile(_request["source"], "<fontcarver>", "exec"), {
        "__name__": "__main__",
        "__builtins__": builtins,
        "fontcarver": fontcarver,
    })
    sys.stdout.flush()
    _send({"kind": "done", "error": None})
except BaseException:
    try:
        sys.stdout.flush()
    except BaseException:
        pass
    _send({"kind": "done", "error": traceback.format_exc(limit=16)[-16384:]})
