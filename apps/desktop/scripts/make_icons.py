# -*- coding: utf-8 -*-
"""SX-018 图标生成：自行设计的应用图标（纯标准库，可复跑）。

设计：深色圆角底 + 青色石碑（Monolith）+ 地平线 + 小光点，
对应“遗迹探索/记录”主题；输出 Tauri 需要的 PNG/ICO/ICNS。

用法（在仓库根目录）：
  py -3 apps/desktop/scripts/make_icons.py
"""
from __future__ import annotations

import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parents[1] / "src-tauri" / "icons"
SS = 4  # 超采样倍数


def render(master: int) -> bytearray:
    """返回 master×master RGBA 像素（超采样后降采样）。"""
    big = master * SS
    px = bytearray(big * big * 4)

    cx = big / 2
    radius = big * 0.22
    corner = radius
    # 圆角矩形内的底色（含轻微纵向渐变）
    def rounded(x: int, y: int, w: int, h: int, r: float) -> bool:
        x0, y0, x1, y1 = x, y, w, h
        if x0 < 0 or y0 < 0 or x1 > big or y1 > big:
            return False
        return True

    for yy in range(big):
        for xx in range(big):
            # 圆角遮罩
            dx = max(corner - xx, xx - (big - 1 - corner), 0)
            dy = max(corner - yy, yy - (big - 1 - corner), 0)
            inside = (dx * dx + dy * dy) <= (corner * corner)
            if not inside:
                continue
            t = yy / big
            r = int(15 + (30 - 15) * t)
            g = int(23 + (41 - 23) * t)
            b = int(42 + (59 - 42) * t)
            base = (yy * big + xx) * 4
            px[base] = r
            px[base + 1] = g
            px[base + 2] = b
            px[base + 3] = 255

    def fill_rounded_rect(x: float, y: float, w: float, h: float, r: float, color: tuple[int, int, int]) -> None:
        for yy in range(int(y), int(y + h)):
            for xx in range(int(x), int(x + w)):
                if xx < 0 or yy < 0 or xx >= big or yy >= big:
                    continue
                dx = max(x + r - xx, xx - (x + w - 1 - r), 0)
                dy = max(y + r - yy, yy - (y + h - 1 - r), 0)
                if dx * dx + dy * dy <= r * r:
                    base = (yy * big + xx) * 4
                    px[base] = color[0]
                    px[base + 1] = color[1]
                    px[base + 2] = color[2]
                    px[base + 3] = 255

    def fill_circle(cx2: float, cy2: float, rad: float, color: tuple[int, int, int]) -> None:
        for yy in range(int(cy2 - rad), int(cy2 + rad) + 1):
            for xx in range(int(cx2 - rad), int(cx2 + rad) + 1):
                if xx < 0 or yy < 0 or xx >= big or yy >= big:
                    continue
                dx = xx - cx2
                dy = yy - cy2
                if dx * dx + dy * dy <= rad * rad:
                    base = (yy * big + xx) * 4
                    px[base] = color[0]
                    px[base + 1] = color[1]
                    px[base + 2] = color[2]
                    px[base + 3] = 255

    # 前景：地平线、石碑、光点
    fill_rounded_rect(big * 0.24, big * 0.70, big * 0.52, big * 0.055, big * 0.03, (14, 116, 144))
    fill_rounded_rect(big * 0.415, big * 0.30, big * 0.17, big * 0.42, big * 0.05, (34, 211, 238))
    fill_circle(big * 0.70, big * 0.30, big * 0.055, (103, 232, 249))

    # 降采样（盒式平均）
    out = bytearray(master * master * 4)
    for y in range(master):
        for x in range(master):
            r = g = b = a = 0
            for sy in range(SS):
                row = ((y * SS + sy) * big + x * SS) * 4
                for sx in range(SS):
                    base = row + sx * 4
                    r += px[base]
                    g += px[base + 1]
                    b += px[base + 2]
                    a += px[base + 3]
            n = SS * SS
            base = (y * master + x) * 4
            out[base] = r // n
            out[base + 1] = g // n
            out[base + 2] = b // n
            out[base + 3] = a // n
    return out


def png_bytes(width: int, pixels: bytearray) -> bytes:
    raw = bytearray()
    stride = width * 4
    for y in range(width):
        raw.append(0)
        raw += pixels[y * stride : (y + 1) * stride]

    def chunk(tag: bytes, payload: bytes) -> bytes:
        return (
            struct.pack(">I", len(payload))
            + tag
            + payload
            + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", width, width, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def ico_bytes(entries: list[tuple[int, bytes]]) -> bytes:
    header = struct.pack("<HHH", 0, 1, len(entries))
    offset = 6 + 16 * len(entries)
    directory = b""
    blobs = b""
    for size, blob in entries:
        dim = 0 if size >= 256 else size
        directory += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(blob), offset)
        blobs += blob
        offset += len(blob)
    return header + directory + blobs


def icns_bytes(entries: list[tuple[bytes, bytes]]) -> bytes:
    body = b""
    for tag, blob in entries:
        body += tag + struct.pack(">I", len(blob) + 8) + blob
    return b"icns" + struct.pack(">I", len(body) + 8) + body


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    png: dict[int, bytes] = {}
    for size in (16, 32, 48, 64, 128, 256, 512):
        pixels = render(size)
        blob = png_bytes(size, pixels)
        png[size] = blob
        if size in (32, 128, 256):
            name = {32: "32x32.png", 128: "128x128.png", 256: "128x128@2x.png"}[size]
            (OUT / name).write_bytes(blob)
    (OUT / "icon.ico").write_bytes(ico_bytes([(s, png[s]) for s in (16, 32, 48, 64, 128, 256)]))
    (OUT / "icon.icns").write_bytes(
        icns_bytes([(b"ic07", png[128]), (b"ic08", png[256]), (b"ic09", png[512])])
    )
    print("icons written to", OUT)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
