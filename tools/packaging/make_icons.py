#!/usr/bin/env python3
"""Genera los iconos de paquete de Parlo a partir del logo DEFINITIVO (docs/brand/brand.md §5, ADR-0006).

Fuente: los SVG de assets/brand/ (los escribe tools/packaging/make_logo.py). Por tamaño:
  ≤ 32 px  parlo-symbol-small.svg   (una curva abierta + brasa, trazo grueso: legible a 16 px)
  ≥ 48 px  parlo-symbol.svg         (dos isolíneas + brasa)

Rasteriza con `rsvg-convert` (librsvg; paquete `librsvg2-bin` en Debian/Ubuntu, `librsvg` en Arch/Fedora,
`brew install librsvg` en macOS). El resto (PNG, .ico y .icns) es biblioteca estándar: sin Pillow ni iconutil.

Salidas (rutas relativas a la raíz del repo; mismos nombres que el icono provisional, así que la ventana
—`include_bytes!` de parlo-256.png en crates/tc-app/src/main.rs— y `build.rs` —parlo.ico— no cambian):
  packaging/icons/parlo-{16,32,48,64,128,256,512}.png      app (Linux hicolor, ventana)
  packaging/icons/parlo.svg                                app escalable (hicolor/scalable)
  packaging/icons/parlo-symbolic.svg                       bandeja/panel, una tinta (hicolor/symbolic)
  packaging/icons/parlo-tray-{16,22,24,32}{,-dark}.png     bandeja una tinta: claro (#f3eee8, paneles
                                                           oscuros) y `-dark` (#1d1915, paneles claros)
  packaging/windows/parlo.ico                              16-256 px (BMP 32 bits; PNG en 256)
  packaging/macos/Parlo.icns                               16-1024 px (PNG, retícula de Big Sur)

Uso: python3 tools/packaging/make_icons.py
"""

from __future__ import annotations

import shutil
import struct
import subprocess
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BRAND = ROOT / "assets" / "brand"
SMALL_MAX = 32  # hasta aquí, versión simple

TRAY_LIGHT = "#f3eee8"  # tinta para paneles oscuros (text del tema oscuro)
TRAY_DARK = "#1d1915"  # tinta para paneles claros (text del tema claro)


def _inner(svg_text: str) -> tuple[str, str]:
    """(viewBox, contenido) de un SVG de una sola raíz escrito por make_logo.py."""
    head, _, rest = svg_text.partition(">")
    view = head.split('viewBox="', 1)[1].split('"', 1)[0]
    body = rest.rsplit("</svg>", 1)[0]
    return view, body


def rasterize(source: Path, size: int, margin: float = 0.0, color: str | None = None) -> bytes:
    """PNG RGBA de `source` a `size` px, con un margen relativo transparente y `currentColor` → `color`."""
    exe = shutil.which("rsvg-convert")
    if exe is None:
        raise SystemExit("falta rsvg-convert (librsvg): instálalo para rasterizar los SVG de assets/brand/")
    view, body = _inner(source.read_text(encoding="utf-8"))
    vx, vy, vw, vh = (float(v) for v in view.split())
    pad = vw * margin / (1.0 - 2.0 * margin) if margin else 0.0
    if color:
        body = body.replace("currentColor", color)
    wrapped = (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vx - pad} {vy - pad} {vw + 2 * pad} {vh + 2 * pad}">'
        f"{body}</svg>"
    )
    out = subprocess.run(
        [exe, "-w", str(size), "-h", str(size), "-f", "png"],
        input=wrapped.encode("utf-8"),
        capture_output=True,
        check=True,
    )
    return out.stdout


def decode_png(data: bytes) -> tuple[int, int, bytes]:
    """PNG RGBA de 8 bits sin entrelazar (lo que produce rsvg-convert) → (ancho, alto, RGBA)."""
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat, w, h = 8, b"", 0, 0
    while pos < len(data):
        (n,) = struct.unpack(">I", data[pos : pos + 4])
        kind, chunk = data[pos + 4 : pos + 8], data[pos + 8 : pos + 8 + n]
        if kind == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", chunk)
            if (depth, ctype, interlace) != (8, 6, 0):
                raise ValueError("se esperaba PNG RGBA de 8 bits sin entrelazar")
        elif kind == b"IDAT":
            idat += chunk
        pos += 12 + n
    raw = zlib.decompress(idat)
    stride, bpp = w * 4, 4
    out = bytearray(h * stride)
    prev = bytearray(stride)
    for y in range(h):
        ftype = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if ftype == 1:
                line[i] = (line[i] + a) & 0xFF
            elif ftype == 2:
                line[i] = (line[i] + b) & 0xFF
            elif ftype == 3:
                line[i] = (line[i] + (a + b) // 2) & 0xFF
            elif ftype == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + pred) & 0xFF
        out[y * stride : (y + 1) * stride] = line
        prev = line
    return w, h, bytes(out)


def ico_dib(size: int, rgba: bytes) -> bytes:
    """Entrada BMP de un .ico: BITMAPINFOHEADER, BGRA de abajo arriba y máscara AND de 1 bit."""
    header = struct.pack("<IiiHHIIiiII", 40, size, size * 2, 1, 32, 0, 0, 0, 0, 0, 0)
    stride = size * 4
    pixels = bytearray()
    for y in reversed(range(size)):
        row = rgba[y * stride : (y + 1) * stride]
        for x in range(size):
            r, g, b, a = row[x * 4 : x * 4 + 4]
            pixels += bytes((b, g, r, a))
    mask_stride = ((size + 31) // 32) * 4
    mask = bytearray()
    for y in reversed(range(size)):
        bits = bytearray(mask_stride)
        for x in range(size):
            if rgba[(y * size + x) * 4 + 3] == 0:
                bits[x // 8] |= 0x80 >> (x % 8)
        mask += bits
    return header + bytes(pixels) + bytes(mask)


def ico(pngs: dict[int, bytes]) -> bytes:
    entries, blobs = [], []
    offset = 6 + 16 * len(pngs)
    for size in sorted(pngs):
        data = pngs[size] if size >= 256 else ico_dib(size, decode_png(pngs[size])[2])
        dim = 0 if size >= 256 else size
        entries.append(struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset))
        blobs.append(data)
        offset += len(data)
    return struct.pack("<HHH", 0, 1, len(pngs)) + b"".join(entries) + b"".join(blobs)


# Tipos ICNS con PNG embebido (macOS 10.7+): tipo -> lado en píxeles.
ICNS_TYPES = [
    (b"icp4", 16), (b"icp5", 32), (b"icp6", 64), (b"ic07", 128), (b"ic08", 256), (b"ic09", 512),
    (b"ic10", 1024), (b"ic11", 32), (b"ic12", 64), (b"ic13", 256), (b"ic14", 512),
]


def icns(pngs: dict[int, bytes]) -> bytes:
    body = b""
    for kind, size in ICNS_TYPES:
        data = pngs[size]
        body += kind + struct.pack(">I", len(data) + 8) + data
    return b"icns" + struct.pack(">I", len(body) + 8) + body


def app_png(size: int, margin: float) -> bytes:
    src = BRAND / ("parlo-symbol-small.svg" if size <= SMALL_MAX else "parlo-symbol.svg")
    return rasterize(src, size, margin)


def main() -> None:
    icons = ROOT / "packaging" / "icons"
    icons.mkdir(parents=True, exist_ok=True)
    # Windows/Linux: casi a sangre. macOS: retícula de Big Sur (cuerpo ≈ 824/1024).
    flat = {s: app_png(s, 0.03) for s in (16, 24, 32, 48, 64, 128, 256, 512)}
    mac = {s: app_png(s, 0.098) for s in (16, 32, 64, 128, 256, 512, 1024)}
    for s in (16, 32, 48, 64, 128, 256, 512):
        (icons / f"parlo-{s}.png").write_bytes(flat[s])
    (icons / "parlo.svg").write_text((BRAND / "parlo-symbol.svg").read_text(encoding="utf-8"), encoding="utf-8")
    tray = BRAND / "parlo-tray-symbolic.svg"
    # Symbolic de freedesktop: una tinta gris que el tema recolorea.
    (icons / "parlo-symbolic.svg").write_text(
        tray.read_text(encoding="utf-8").replace("currentColor", "#bebebe"), encoding="utf-8"
    )
    for s in (16, 22, 24, 32):
        (icons / f"parlo-tray-{s}.png").write_bytes(rasterize(tray, s, 0.06, TRAY_LIGHT))
        (icons / f"parlo-tray-{s}-dark.png").write_bytes(rasterize(tray, s, 0.06, TRAY_DARK))
    (ROOT / "packaging" / "windows").mkdir(parents=True, exist_ok=True)
    (ROOT / "packaging" / "windows" / "parlo.ico").write_bytes(ico({s: flat[s] for s in (16, 24, 32, 48, 64, 128, 256)}))
    (ROOT / "packaging" / "macos").mkdir(parents=True, exist_ok=True)
    (ROOT / "packaging" / "macos" / "Parlo.icns").write_bytes(icns(mac))
    print("iconos generados en packaging/{icons,windows,macos} desde assets/brand/")


if __name__ == "__main__":
    main()
