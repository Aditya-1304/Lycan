#!/usr/bin/env python3
"""Rebuild original backup-ID cartridges and check their frozen identities."""
from pathlib import Path
import hashlib
import json
root = Path(__file__).resolve().parents[2]
fixtures={'sram':b'SRAM_V113','sram-fast':b'SRAM_F_V103','eeprom':b'EEPROM_V124','flash64':b'FLASH512_V131','flash64-old':b'FLASH_V123','flash128':b'FLASH1M_V103','unknown':b'', 'conflicting':b'SRAM_V113\0\0\0FLASH1M_V103','override':b'EEPROM_V124','same-family':b'FLASH_V123\0\0\0FLASH512_V131','malformed':b'SRAM_Vxyz'}
records = []
for name, signature in fixtures.items():
    # Direct ARM entry loops harmlessly; the identifier begins after the header.
    data = bytearray(192)
    data[:4] = bytes.fromhex('feffffea')
    data[0xa0:0xac] = b'BACKUP PROBE'
    data.extend(signature)
    data.extend(b'\0' * (-len(data) % 4))
    (root / f'roms/backup/{name}.gba').write_bytes(data)
    records.append({'name':name,'sha256':hashlib.sha256(data).hexdigest()})
assert records == json.loads((root / "roms/backup/manifest.json").read_text()), "generated cartridges differ from frozen manifest"
print("Rebuilt 11 backup cartridges; frozen SHA-256 identities match")
