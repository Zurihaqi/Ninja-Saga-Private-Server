"""Binary-level safety checks for UI-only silent pet movie generation."""
import importlib.util
import lzma
from pathlib import Path
import struct
import unittest
import zlib

SCRIPT = Path(__file__).resolve().parents[1] / "prepare-pet-previews.py"
SPEC = importlib.util.spec_from_file_location("pet_previews", SCRIPT)
PREVIEWS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PREVIEWS)


def movie(signature):
    tag = PREVIEWS.encode_tag
    symbol = struct.pack("<HH", 1, 7) + b"PetVoice\0"
    sound = struct.pack("<HBI", 7, 0x2F, 44100) + b"AUDIBLE-SOUND-DATA"
    nested = struct.pack("<HH", 99, 1) + tag(18, b"stream-header") + tag(19, b"AUDIBLE-STREAM") + tag(15, struct.pack("<HB", 7, 0)) + tag(2, b"preserve-shape") + tag(1, b"") + tag(0, b"")
    body = b"\x08\x00\x00\x18\x01\x00" + tag(14, sound) + tag(76, symbol) + tag(56, symbol) + tag(39, nested) + tag(89, b"PetVoice\0\0") + tag(1, b"") + tag(0, b"")
    header = signature + b"\x0d" + struct.pack("<I", len(body) + 8)
    if signature == b"FWS":
        return header + body
    if signature == b"CWS":
        return header + zlib.compress(body)
    filters = [{"id": lzma.FILTER_LZMA1, "dict_size": 1 << 20, "lc": 3, "lp": 0, "pb": 2}]
    compressed = lzma.compress(body, format=lzma.FORMAT_RAW, filters=filters)
    return header + struct.pack("<I", len(compressed) + 5) + lzma._encode_filter_properties(filters[0]) + compressed


class PetPreviewTests(unittest.TestCase):
    def test_all_compression_formats_preserve_symbols_and_visuals(self):
        for signature in (b"FWS", b"CWS", b"ZWS"):
            with self.subTest(signature=signature):
                original = movie(signature)
                quiet, symbols, removed = PREVIEWS.make_silent_preview(original)
                self.assertEqual(symbols, 1)
                self.assertEqual(removed, 4)
                _, body, offset = PREVIEWS.unpack_movie(quiet)
                controls = list(PREVIEWS.tags(body, offset))
                sound = next(payload for kind, payload in controls if kind == 14)
                sound_id, flags, samples = struct.unpack_from("<HBI", sound)
                self.assertEqual((sound_id, flags, samples), (7, 0x0E, 44100))
                self.assertEqual(sound[7:], bytes(44100 * 2))
                bindings = [payload for kind, payload in controls if kind in (56, 76)]
                self.assertEqual(bindings, [struct.pack("<HH", 1, 7) + b"PetVoice\0"] * 2)
                sprite = next(payload for kind, payload in controls if kind == 39)
                self.assertEqual(struct.unpack_from("<HH", sprite), (99, 1))
                self.assertEqual(list(PREVIEWS.tags(sprite, 4)), [(2, b"preserve-shape"), (1, b""), (0, b"")])
                self.assertNotIn(b"AUDIBLE", body)
                self.assertNotEqual(quiet, original)

    def test_rejects_truncated_or_inconsistent_movies(self):
        for data in (b"FWS", movie(b"FWS")[:-1], b"BAD" + movie(b"FWS")[3:]):
            with self.subTest(length=len(data)):
                with self.assertRaises(ValueError):
                    PREVIEWS.make_silent_preview(data)


if __name__ == "__main__":
    unittest.main()
