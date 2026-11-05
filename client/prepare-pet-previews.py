"""Build silent UI-only copies of pet movies without changing combat assets.

Preview loaders execute a movie before the extracted static body is attached.
Some movies also start Sound.play channels outside the display tree, so muting
the display object cannot contain that audio. These derivatives retain sound
symbol IDs/bindings but replace their samples with silence and remove timeline
sound triggers/streams. Combat continues loading the original pets/ directory.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import lzma
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / ".tools/client/pets"
TARGET = ROOT / ".tools/client/pet-previews"
SOUND_TRIGGERS = {15, 89}  # StartSound, StartSound2
SOUND_STREAMS = {18, 19, 45}  # SoundStreamHead, SoundStreamBlock, SoundStreamHead2
PREVIEW_VERSION = 4
MAX_SOUND_SAMPLES = 16_000_000  # Bound temporary PCM allocation per symbol.


def unpack_movie(wire: bytes) -> tuple[int, bytes, int]:
    if len(wire) < 8:
        raise ValueError("Truncated SWF header")
    signature = wire[:3]
    if signature == b"FWS":
        body = wire[8:]
    elif signature == b"CWS":
        body = zlib.decompress(wire[8:])
    elif signature == b"ZWS":
        if len(wire) < 17:
            raise ValueError("Truncated LZMA SWF header")
        filters = [lzma._decode_filter_properties(lzma.FILTER_LZMA1, wire[12:17])]
        body = lzma.decompress(wire[17:], format=lzma.FORMAT_RAW, filters=filters)
    else:
        raise ValueError("Unsupported SWF signature")
    if len(body) + 8 != struct.unpack_from("<I", wire, 4)[0]:
        raise ValueError("SWF declared length does not match decoded content")
    if not body:
        raise ValueError("Missing SWF frame header")
    tags_offset = (5 + 4 * (body[0] >> 3) + 7) // 8 + 4
    if tags_offset > len(body):
        raise ValueError("Truncated SWF frame header")
    return wire[3], body, tags_offset


def tags(data: bytes, offset: int = 0):
    while offset < len(data):
        if offset + 2 > len(data):
            raise ValueError("Truncated SWF tag header")
        header = struct.unpack_from("<H", data, offset)[0]
        offset += 2
        kind, length = header >> 6, header & 63
        if length == 63:
            if offset + 4 > len(data):
                raise ValueError("Truncated SWF long tag header")
            length = struct.unpack_from("<I", data, offset)[0]
            offset += 4
        if offset + length > len(data):
            raise ValueError("Truncated SWF tag body")
        yield kind, data[offset:offset + length]
        offset += length


def encode_tag(kind: int, data: bytes) -> bytes:
    if len(data) < 63:
        return struct.pack("<H", (kind << 6) | len(data)) + data
    return struct.pack("<HI", (kind << 6) | 63, len(data)) + data


def quiet_tags(data: bytes, offset: int = 0, depth: int = 0) -> bytes:
    if depth > 64:
        raise ValueError("Excessively nested sprite timeline")
    output = bytearray()
    for kind, payload in tags(data, offset):
        if kind in SOUND_TRIGGERS | SOUND_STREAMS:
            continue
        if kind == 14:  # DefineSound: retain SoundId for SymbolClass/ExportAssets.
            if len(payload) < 7:
                raise ValueError("Truncated DefineSound")
            samples = struct.unpack_from("<I", payload, 3)[0]
            if samples > MAX_SOUND_SAMPLES:
                raise ValueError("Sound sample count exceeds preview preparation bound")
            # Preserve duration/rate so SoundComplete loops keep their timing.
            # PCM format 0, original rate, 16-bit sample size, mono channels.
            flags = (payload[2] & 0x0C) | 0x02
            payload = payload[:2] + bytes([flags]) + struct.pack("<I", samples) + bytes(samples * 2)
        elif kind == 39:  # DefineSprite has its own nested control tag stream.
            if len(payload) < 4:
                raise ValueError("Truncated DefineSprite")
            payload = payload[:4] + quiet_tags(payload, 4, depth + 1)
        output.extend(encode_tag(kind, payload))
    return bytes(output)


def inspect_tags(data: bytes, offset: int = 0, *, require_quiet: bool = False):
    sound_ids = []
    symbol_bindings = []
    triggers = 0
    for kind, payload in tags(data, offset):
        if kind in SOUND_TRIGGERS | SOUND_STREAMS:
            triggers += 1
            if require_quiet:
                raise ValueError(f"Audible timeline tag {kind} remains in preview")
        elif kind == 14:
            if len(payload) < 7:
                raise ValueError("Truncated sound symbol")
            samples = struct.unpack_from("<I", payload, 3)[0]
            sound_ids.append((payload[:2], payload[2] & 0x0C, samples))
            if require_quiet:
                if samples > MAX_SOUND_SAMPLES or payload[2] & 0xF3 != 0x02:
                    raise ValueError("Preview sound is not bounded mono 16-bit PCM")
                if len(payload) != 7 + samples * 2 or any(payload[7:]):
                    raise ValueError("Non-silent sound samples remain in preview")
        elif kind in {56, 76}:  # ExportAssets, SymbolClass
            symbol_bindings.append((kind, payload))
        elif kind == 39:
            if len(payload) < 4:
                raise ValueError("Truncated sprite")
            nested_ids, nested_bindings, nested_triggers = inspect_tags(payload, 4, require_quiet=require_quiet)
            sound_ids.extend(nested_ids)
            symbol_bindings.extend(nested_bindings)
            triggers += nested_triggers
    return sound_ids, symbol_bindings, triggers


def make_silent_preview(wire: bytes) -> tuple[bytes, int, int]:
    version, body, offset = unpack_movie(wire)
    original_ids, original_bindings, removed = inspect_tags(body, offset)
    quiet_body = body[:offset] + quiet_tags(body, offset)
    quiet_ids, quiet_bindings, _ = inspect_tags(quiet_body, offset, require_quiet=True)
    if quiet_ids != original_ids or quiet_bindings != original_bindings:
        raise ValueError("Sound symbols, duration/rate or exported character bindings changed")
    result = b"CWS" + bytes([version]) + struct.pack("<I", len(quiet_body) + 8) + zlib.compress(quiet_body)
    return result, len(quiet_ids), removed


def sha256(wire: bytes) -> str:
    return hashlib.sha256(wire).hexdigest()


def prepare(*, check: bool = False) -> None:
    movies = sorted(SOURCE.glob("*.swf"))
    if not movies:
        raise ValueError("No local pet movies found; prepare the local client first")
    if TARGET.resolve() == SOURCE.resolve() or not TARGET.resolve().is_relative_to((ROOT / ".tools").resolve()):
        raise ValueError("Preview output must be a separate ignored local directory")
    original_hashes = {path.name: sha256(path.read_bytes()) for path in movies}
    manifest_path = TARGET / "manifest.json"
    if check:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if manifest["version"] != PREVIEW_VERSION or manifest["original_sha256"] != original_hashes:
            raise ValueError("Silent previews are missing or stale; run the preparation helper")
        for path in movies:
            wire = (TARGET / path.name).read_bytes()
            _, body, offset = unpack_movie(wire)
            inspect_tags(body, offset, require_quiet=True)
            if sha256(wire) != manifest["preview_sha256"][path.name]:
                raise ValueError(f"Preview hash changed: {path.name}")
        print(f"Verified {len(movies)} silent pet previews; combat movie hashes unchanged.")
        return
    TARGET.mkdir(parents=True, exist_ok=True)
    preview_hashes = {}
    replaced_sounds = removed_tags = 0
    for path in movies:
        wire, symbols, removed = make_silent_preview(path.read_bytes())
        output = TARGET / path.name
        staged = output.with_suffix(".preparing.swf")
        staged.write_bytes(wire)
        staged.replace(output)
        preview_hashes[path.name] = sha256(wire)
        replaced_sounds += symbols
        removed_tags += removed
    if any(sha256(path.read_bytes()) != original_hashes[path.name] for path in movies):
        raise ValueError("Combat pet assets changed during preparation")
    manifest_path.write_text(json.dumps({
        "version": PREVIEW_VERSION,
        "original_sha256": original_hashes,
        "preview_sha256": preview_hashes,
        "silent_sound_symbols": replaced_sounds,
        "removed_audio_tags": removed_tags,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"Prepared {len(movies)} silent pet previews: {replaced_sounds} sound symbols silenced, {removed_tags} audio tags removed. Combat assets unchanged.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    prepare(check=parser.parse_args().check)
