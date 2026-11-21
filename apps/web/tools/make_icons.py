#!/usr/bin/env python3
"""Generates the PWA / iOS home-screen icons (pure Python, no dependencies).

The icon is the in-game logo: a 3x3 grid of rounded tiles on the app background.
Run from apps/web:  python3 tools/make_icons.py
"""
import struct
import zlib
from pathlib import Path

BG = (0x16, 0x16, 0x1B)
EMPTY = (0x2D, 0x2D, 0x37)
COLORS = [(0x4C, 0xC3, 0x8A), (0x5C, 0x8E, 0xE6), (0xF2, 0xA9, 0x3B), (0xF0, 0x6A, 0x4A), (0xC6, 0x5B, 0xF0)]


def tile_color(k):
    return COLORS[(k // 2) % 5] if k % 2 == 0 else EMPTY


def render(size, scale=0.56):
    ss = 3  # supersampling for smooth corners
    grid = size * scale
    gap = grid * 0.08
    tile = (grid - 2 * gap) / 3
    radius = tile * 0.24
    x0 = (size - grid) / 2
    tiles = []
    for k in range(9):
        r, c = divmod(k, 3)
        tx, ty = x0 + c * (tile + gap), x0 + r * (tile + gap)
        tiles.append((tx, ty, tile_color(k)))

    def inside_rounded(px, py, tx, ty):
        if not (tx <= px <= tx + tile and ty <= py <= ty + tile):
            return False
        cx = min(max(px, tx + radius), tx + tile - radius)
        cy = min(max(py, ty + radius), ty + tile - radius)
        return (px - cx) ** 2 + (py - cy) ** 2 <= radius ** 2

    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            acc = [0, 0, 0]
            for sy in range(ss):
                for sx in range(ss):
                    px, py = x + (sx + 0.5) / ss, y + (sy + 0.5) / ss
                    col = BG
                    for tx, ty, tc in tiles:
                        if inside_rounded(px, py, tx, ty):
                            col = tc
                            break
                    acc[0] += col[0]; acc[1] += col[1]; acc[2] += col[2]
            n = ss * ss
            row += bytes((acc[0] // n, acc[1] // n, acc[2] // n))
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


if __name__ == "__main__":
    out = Path(__file__).resolve().parent.parent / "icons"
    out.mkdir(exist_ok=True)
    for name, size, scale in [
        ("icon-192.png", 192, 0.56),
        ("icon-512.png", 512, 0.56),
        ("icon-maskable-512.png", 512, 0.46),  # extra padding for Android masks
        ("apple-touch-icon.png", 180, 0.56),
    ]:
        (out / name).write_bytes(render(size, scale))
        print("wrote", name)
