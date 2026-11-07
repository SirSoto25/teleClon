#!/usr/bin/env python3
"""Genera el icono PROVISIONAL de Parlo para empaquetar (ADR-0006).

PROVISIONAL: el logo oficial no existe todavía (solo hay un brief en docs/brand/brand.md §5). Este
icono es un marcador geométrico sencillo que sigue el brief a grandes rasgos (cuadrado redondeado de
radio 0.32 × lado, grafito cálido, isolíneas y un punto «brasa» persimón) para que los instaladores
tengan algo propio. Sustituirlo cuando exista el logo definitivo: basta con reemplazar los archivos de
salida (mismos nombres) o adaptar `render()`.

Solo usa la biblioteca estándar (zlib + struct): no hace falta Pillow ni iconutil.

Salidas (rutas relativas a la raíz del repo):
  packaging/icons/parlo-{16,32,48,64,128,256,512}.png   iconos PNG (Linux, ventana)
  packaging/windows/parlo.ico                            16-256 px (BMP 32 bits + PNG en 256)
  packaging/macos/Parlo.icns                             16-1024 px (PNG embebido, retícula macOS)

Uso: python3 tools/packaging/make_icons.py
"""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# Colores del brief (docs/design/tokens.json): grafito cálido y acento persimón del tema oscuro.
BG = (0x12, 0x0F, 0x0C)
ACCENT = (0xF6, 0x7E, 0x42)
RING = (0x8A, 0x4D, 0x2E)  # acento mezclado ~50 % con el fondo: isolíneas discretas


def _cov(d: float) -> float:
    """Cobertura antialias de una distancia con signo en píxeles (negativa = dentro)."""
    return max(0.0, min(1.0, 0.5 - d))


def _rounded_rect_sdf(px: float, py: float, cx: float, cy: float, half: float, r: float) -> float:
    qx = abs(px - cx) - (half - r)
    qy = abs(py - cy) - (half - r)
    outside = math.hypot(max(qx, 0.0), max(qy, 0.0))
    inside = min(max(qx, qy), 0.0)
    return outside + inside - r


def render(size: int, margin: float) -> bytes:
    """RGBA (filas de arriba abajo) del icono a `size` px con un margen relativo `margin`."""
    s = float(size)
    half = s * (1.0 - 2.0 * margin) / 2.0
    c = s / 2.0
    radius = 0.32 * 2.0 * half
    simple = size <= 32  # a 16-32 px: una sola curva + punto (brief)
    # Isolíneas: círculos desplazados hacia arriba-izquierda, cada vez más estrechos hacia el centro.
    rings = [(0.62, 0.060, 0.00)] if simple else [(0.70, 0.045, 0.10), (0.46, 0.040, 0.05)]
    dot_r = (0.24 if simple else 0.20) * half
    dot_c = (c + 0.04 * half, c + 0.04 * half)
    gap_angle = math.radians(35.0)  # la isolínea exterior se abre abajo a la derecha: «boca» de diálogo
    out = bytearray(size * size * 4)
    for y in range(size):
        py = y + 0.5
        for x in range(size):
            px = x + 0.5
            a_bg = _cov(_rounded_rect_sdf(px, py, c, c, half, radius))
            if a_bg <= 0.0:
                continue
            col = list(BG)
            for i, (rr, width, shift) in enumerate(rings):
                rcx = c - shift * half
                rcy = c - shift * half
                dist = math.hypot(px - rcx, py - rcy)
                d = abs(dist - rr * half) - width * half / 2.0
                a = _cov(d)
                if i == 0 and not simple and a > 0.0:
                    ang = math.atan2(py - rcy, px - rcx)  # y crece hacia abajo
                    delta = abs((ang - math.radians(45.0) + math.pi) % (2 * math.pi) - math.pi)
                    a *= max(0.0, min(1.0, (delta - gap_angle / 2.0) * rr * half + 0.5))
                if a > 0.0:
                    col = [col[k] + (RING[k] - col[k]) * a for k in range(3)]
            a_dot = _cov(math.hypot(px - dot_c[0], py - dot_c[1]) - dot_r)
            if a_dot > 0.0:
                col = [col[k] + (ACCENT[k] - col[k]) * a_dot for k in range(3)]
            o = (y * size + x) * 4
            out[o : o + 4] = bytes((round(col[0]), round(col[1]), round(col[2]), round(255 * a_bg)))
    return bytes(out)


def png(size: int, rgba: bytes) -> bytes:
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    stride = size * 4
    raw = b"".join(b"\x00" + rgba[y * stride : (y + 1) * stride] for y in range(size))
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


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


def ico(images: dict[int, bytes]) -> bytes:
    entries, blobs = [], []
    offset = 6 + 16 * len(images)
    for size in sorted(images):
        data = png(size, images[size]) if size >= 256 else ico_dib(size, images[size])
        dim = 0 if size >= 256 else size
        entries.append(struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset))
        blobs.append(data)
        offset += len(data)
    return struct.pack("<HHH", 0, 1, len(images)) + b"".join(entries) + b"".join(blobs)


# Tipos ICNS con PNG embebido (macOS 10.7+): tipo -> lado en píxeles.
ICNS_TYPES = [
    (b"icp4", 16), (b"icp5", 32), (b"icp6", 64), (b"ic07", 128), (b"ic08", 256), (b"ic09", 512),
    (b"ic10", 1024), (b"ic11", 32), (b"ic12", 64), (b"ic13", 256), (b"ic14", 512),
]


def icns(images: dict[int, bytes]) -> bytes:
    body = b""
    for kind, size in ICNS_TYPES:
        data = png(size, images[size])
        body += kind + struct.pack(">I", len(data) + 8) + data
    return b"icns" + struct.pack(">I", len(body) + 8) + body


def main() -> None:
    icons = ROOT / "packaging" / "icons"
    icons.mkdir(parents=True, exist_ok=True)
    # Windows/Linux: casi a sangre. macOS: retícula de Big Sur (cuerpo ≈ 824/1024).
    flat = {s: render(s, 0.03) for s in (16, 24, 32, 48, 64, 128, 256, 512)}
    mac = {s: render(s, 0.098) for s in (16, 32, 64, 128, 256, 512, 1024)}
    for s in (16, 32, 48, 64, 128, 256, 512):
        (icons / f"parlo-{s}.png").write_bytes(png(s, flat[s]))
    (ROOT / "packaging" / "windows").mkdir(parents=True, exist_ok=True)
    (ROOT / "packaging" / "windows" / "parlo.ico").write_bytes(ico({s: flat[s] for s in (16, 24, 32, 48, 64, 128, 256)}))
    (ROOT / "packaging" / "macos").mkdir(parents=True, exist_ok=True)
    (ROOT / "packaging" / "macos" / "Parlo.icns").write_bytes(icns(mac))
    print("iconos PROVISIONALES generados en packaging/{icons,windows,macos}")


if __name__ == "__main__":
    main()
