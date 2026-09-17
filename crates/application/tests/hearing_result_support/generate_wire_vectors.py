"""Independent HRTX1 vectors for docs/hearing-results-api.md."""
from pathlib import Path
from struct import pack
from uuid import UUID


def uuid(number):
    return UUID(int=number).bytes


def vector(action, continuation, reason):
    data = b"HRTX1" + b"".join(uuid(n) for n in (1, 2, 3, 4, 5))
    data += pack(">BI", action, 0 if action == 0 else 2)
    data += pack(">I", 7) + bytes([17]) * 32 + bytes([34]) * 32
    data += bytes([continuation])
    if continuation:
        data += uuid(6) + uuid(7) + pack(">I", 9)
        data += bytes([51]) * 32 + bytes([68]) * 32
    data += bytes(range(32)) + bytes([reason is not None])
    if reason is not None:
        encoded = reason.encode("utf-8")
        data += pack(">I", len(encoded)) + encoded
    return data.hex()


if __name__ == "__main__":
    values = [vector(0, False, None), vector(1, True, "Motivo\n\u00e1"),
              vector(2, True, "Retiro")]
    Path(__file__).with_name("wire-vectors.txt").write_text("\n".join(values) + "\n")
