"""Independent HTXN1 vectors for schedule, replacement and cancellation."""
from pathlib import Path
from uuid import UUID
import struct

out=[]
for action in range(3):
    data=b"HTXN1"+b"".join(UUID(int=n).bytes for n in [1,2,3,4])+bytes([action])
    data+=struct.pack(">I",0 if action==0 else 7)
    data+=b"\0" if action==2 else b"\1"+struct.pack(">II",11,13)
    data+=bytes(range(32))
    reason="Motivo\n\u00e1".encode("utf-8")
    data+=b"\0" if action==0 else b"\1"+struct.pack(">I",len(reason))+reason
    out.append(data.hex())
Path(__file__).with_name("wire-vectors.txt").write_text("\n".join(out)+"\n",encoding="ascii")
print("HTXN1 vector lengths:", [len(bytes.fromhex(row)) for row in out])
