"""Make a local-only test copy by replacing URL constants in DoABC string pools.

Preserves bytecode and all non-URL constants. Does not alter the pinned reference.
"""
from pathlib import Path
import lzma
import shutil
import struct
import subprocess
import sys
import zlib

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'reference/upstream/Custom Client/NS Custom Client V1'
TARGET = ROOT / '.tools/client'
REPLACEMENTS = {
    'https://ninjasage.id/merchants': 'http://127.0.0.1:8080/app/',
    'https://ninjasage.id/en/merchants?buy-now': 'http://127.0.0.1:8080/app/',
    'https://ninjasage.test/clan': 'http://127.0.0.1:8080/clan',
    'https://ninjasage.test/crew': 'http://127.0.0.1:8080/crew',
    'https://ninjasage.test/amf': 'http://127.0.0.1:8080/amf',
    'https://ninjasage.test/game_data/': 'http://127.0.0.1:8080/game_data/',
    'https://ninjasage.id/api/id-custom-color': 'http://127.0.0.1:8080/api/id-custom-color',
    'https://ninjasage.id/api/event/sw': 'http://127.0.0.1:8080/api/event/sw',
}

def read_u30(data, offset):
    value = 0
    for shift in range(0, 35, 7):
        byte = data[offset]
        offset += 1
        value |= (byte & 127) << shift
        if not byte & 128:
            return value, offset
    raise ValueError('Invalid ABC integer')

def u30(value):
    result = bytearray()
    while value > 127:
        result.append((value & 127) | 128)
        value >>= 7
    result.append(value)
    return bytes(result)

def patch_abc(data):
    offset = 4  # minor + major version
    for _ in range(2):
        count, offset = read_u30(data, offset)
        for _ in range(max(0, count - 1)):
            _, offset = read_u30(data, offset)
    count, offset = read_u30(data, offset)
    offset += max(0, count - 1) * 8
    count, offset = read_u30(data, offset)
    output = bytearray(data[:offset])
    changed = []
    for _ in range(max(0, count - 1)):
        length, offset = read_u30(data, offset)
        value = data[offset:offset + length]
        offset += length
        replacement = REPLACEMENTS.get(value.decode('utf8', errors='replace'))
        if replacement:
            changed.append(value.decode())
            value = replacement.encode()
        output.extend(u30(len(value)))
        output.extend(value)
    output.extend(data[offset:])
    return bytes(output), changed

def patch_swf(wire):
    if wire[:3] == b'ZWS':
        filters = [lzma._decode_filter_properties(lzma.FILTER_LZMA1, wire[12:17])]
        body = lzma.decompress(wire[17:], format=lzma.FORMAT_RAW, filters=filters)
    elif wire[:3] == b'CWS':
        body = zlib.decompress(wire[8:])
    elif wire[:3] == b'FWS':
        body = wire[8:]
    else:
        raise ValueError('Unsupported SWF')
    offset = (5 + 4 * (body[0] >> 3) + 7) // 8 + 4
    output = bytearray(body[:offset])
    changed = []
    while offset < len(body):
        tag = struct.unpack_from('<H', body, offset)[0]
        offset += 2
        kind, length = tag >> 6, tag & 63
        if length == 63:
            length = struct.unpack_from('<I', body, offset)[0]
            offset += 4
        data = body[offset:offset + length]
        if len(data) != length:
            raise ValueError('Truncated SWF tag')
        offset += length
        if kind == 82:
            start = data.index(b'\0', 4) + 1
            abc, replacements = patch_abc(data[start:])
            data = data[:start] + abc
            changed.extend(replacements)
        output.extend(struct.pack('<HI', (kind << 6) | 63, len(data)))
        output.extend(data)
    header = b'CWS' + wire[3:4] + struct.pack('<I', len(output) + 8)
    return header + zlib.compress(output), changed

if __name__ == '__main__':
    TARGET.mkdir(parents=True, exist_ok=True)
    for directory in ['Adobe AIR', 'META-INF', 'AppIconsForPublish', 'items', 'skills', 'enemy', 'mission', 'npcs', 'pets', 'sounds', 'swfpanels', 'consumables', 'essentials', 'materials']:
        shutil.copytree(SOURCE / directory, TARGET / directory, dirs_exist_ok=True)
    shutil.copy2(SOURCE / 'NSCUSTOM.exe', TARGET / 'NSCUSTOM.exe')
    patched, changed = patch_swf((SOURCE / 'NinjaSage.swf').read_bytes())
    if not set(REPLACEMENTS).issubset(changed):
        raise ValueError(f'Expected endpoints not found: {set(REPLACEMENTS) - set(changed)}')
    (TARGET / 'NinjaSage.swf').write_bytes(patched)
    # Use an independently named AIR data folder to avoid saved production accounts.
    manifest = TARGET / 'META-INF/AIR/application.xml'
    xml = manifest.read_text(encoding='utf8')
    xml = xml.replace('<id>id.ninjasage</id>', '<id>id.ninjasage.rust.local</id>')
    xml = xml.replace('<fullScreen>true</fullScreen>', '<fullScreen>false</fullScreen>')
    manifest.write_text(xml, encoding='utf8')
    print(f'Prepared local client; replaced {len(changed)} endpoint constants. Reference files unchanged.')
    # A fresh copy must include the panel fixes and free-product actions too.
    subprocess.run([sys.executable, str(ROOT / 'client/patch-client-panels.py')], cwd=ROOT, check=True)
