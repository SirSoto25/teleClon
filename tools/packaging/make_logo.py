#!/usr/bin/env python3
"""Genera el logo DEFINITIVO de Parlo (símbolo «Brasa») como SVG vectoriales (docs/brand/brand.md §5).

El símbolo es un dibujo geométrico original: un cuadrado redondeado (radio 0.32 × lado, la forma de los
avatares de Brasa) con dos isolíneas irregulares de grosor uniforme alrededor de un punto persimón (la
«brasa»). La isolínea exterior se abre arriba a la derecha: es la «boca» de diálogo. Las curvas se
estrechan hacia la brasa (cada hueco es menor que el anterior) y su centro se desplaza hacia abajo a la
derecha, como las curvas de nivel de una loma. A tamaños pequeños (≤ 32 px) se usa la versión simple:
solo la curva abierta + el punto, con trazo más grueso.

Las curvas salen de fórmulas polares (abajo); se escriben como béziers cúbicas (Catmull-Rom → Bézier) con
coordenadas redondeadas a 0,1 en una retícula de 1024. El logotipo usa Bricolage Grotesque (OFL, ya en
assets/fonts) convertida a contornos con fontTools, así que el SVG no depende de la fuente instalada.

Salidas (assets/brand/):
  parlo-symbol.svg               icono de app, tema oscuro (grafito #120f0c + persimón #f67e42)
  parlo-symbol-light.svg         icono de app, tema claro (blanco cálido #f8f5f1 + persimón #c64913)
  parlo-symbol-small.svg         versión simple para ≤ 32 px (oscura)
  parlo-symbol-small-light.svg   versión simple para ≤ 32 px (clara)
  parlo-symbol-mono.svg          monocroma: cuadrado en tinta y motivo calado (fill currentColor)
  parlo-glyph.svg                solo el motivo, una tinta (currentColor), para poner sobre un fondo propio
  parlo-tray-symbolic.svg        bandeja/panel: motivo simple, una tinta, sin cuadrado (currentColor)
  parlo-logo-horizontal.svg      símbolo + «parlo», texto claro (para fondos oscuros)
  parlo-logo-horizontal-light.svg símbolo + «parlo», texto oscuro (para fondos claros)
  parlo-logo-horizontal-mono.svg monocromo (currentColor)

Uso: python3 tools/packaging/make_logo.py   (necesita fontTools solo para el logotipo)
Después: python3 tools/packaging/make_icons.py   (rasteriza a PNG/.ico/.icns)
"""

from __future__ import annotations

import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "brand"
FONT = ROOT / "assets" / "fonts" / "bricolage-grotesque" / "BricolageGrotesque[opsz,wdth,wght].ttf"

S = 1024.0  # lado de la retícula
RADIUS = 0.32 * S  # radio de avatar Brasa

# Paleta Brasa (docs/design/system.md §2).
DARK_BG, DARK_INK, DARK_TEXT = "#120f0c", "#f67e42", "#f3eee8"
LIGHT_BG, LIGHT_INK, LIGHT_TEXT = "#f8f5f1", "#c64913", "#1d1915"

# Centro óptico del motivo (un poco por encima del centro geométrico).
CX, CY = 524.0, 522.0
GAP_CENTER = math.radians(-38.0)  # la boca mira arriba a la derecha (y crece hacia abajo)


def wobble(t: float, phase: float) -> float:
    """Irregularidad de la isolínea: dos armónicos suaves (no es un círculo ni una elipse)."""
    return 1 + 0.075 * math.sin(2 * t + 0.9 + phase) + 0.035 * math.sin(3 * t + 2.3 + 1.4 * phase)


def blob(r: float, shift: float, phase: float, n: int = 72) -> list[tuple[float, float]]:
    """Curva cerrada irregular tipo isolínea: radio base `r`, desplazada `shift` hacia arriba-izquierda."""
    pts = []
    for i in range(n):
        t = 2 * math.pi * i / n
        k = wobble(t, phase)
        x = CX - shift * 0.8 + r * k * math.cos(t)
        y = CY - shift * 0.6 + r * k * 0.94 * math.sin(t)
        pts.append((x, y))
    return pts


def open_blob(r: float, shift: float, phase: float, gap: float, n: int = 64) -> list[tuple[float, float]]:
    """Como `blob`, pero abierta: recorre el ángulo fuera de la boca (de un labio al otro)."""
    start = GAP_CENTER + gap / 2
    span = 2 * math.pi - gap
    pts = []
    for i in range(n + 1):
        t = start + span * i / n
        k = wobble(t, phase)
        x = CX - shift * 0.8 + r * k * math.cos(t)
        y = CY - shift * 0.6 + r * k * 0.94 * math.sin(t)
        pts.append((x, y))
    return pts


def f(v: float) -> str:
    s = f"{v:.1f}"
    return s[:-2] if s.endswith(".0") else s


def catmull(pts: list[tuple[float, float]], closed: bool) -> str:
    """Polilínea suave → path SVG con béziers cúbicas (Catmull-Rom uniforme)."""
    n = len(pts)
    get = (lambda i: pts[i % n]) if closed else (lambda i: pts[max(0, min(n - 1, i))])
    d = [f"M{f(pts[0][0])} {f(pts[0][1])}"]
    last = n if closed else n - 1
    for i in range(last):
        p0, p1, p2, p3 = get(i - 1), get(i), get(i + 1), get(i + 2)
        c1 = (p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6)
        c2 = (p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6)
        d.append(f"C{f(c1[0])} {f(c1[1])} {f(c2[0])} {f(c2[1])} {f(p2[0])} {f(p2[1])}")
    if closed:
        d.append("Z")
    return "".join(d)


# --- Geometría del motivo ------------------------------------------------------------------------
# Completo: isolínea exterior abierta (boca), isolínea interior cerrada, brasa. Huecos: 112 → 84 px.
FULL = {
    "stroke": 58.0,
    "outer": (296.0, 18.0, 0.0, math.radians(60)),  # r, shift, fase, ancho de la boca
    "inner": (172.0, -18.0, 0.35),
    "dot": 76.0,
}
# Simple (≤ 32 px): una curva abierta + punto, trazo grueso para que sobreviva a 16 px.
SMALL = {
    "stroke": 118.0,
    "outer": (262.0, 0.0, 0.0, math.radians(70)),
    "dot": 112.0,
}


def dot_center(spec: dict) -> tuple[float, float]:
    # La brasa se va hacia abajo a la derecha, igual que las isolíneas se cierran hacia ella.
    return (CX + 30.0, CY + 22.0) if "inner" in spec else (CX + 10.0, CY + 10.0)


def motif(spec: dict, ink: str) -> str:
    """Elementos del motivo: trazos con remate redondo y la brasa rellena."""
    sw = f(spec["stroke"])
    r, sh, ph, gap = spec["outer"]
    parts = [
        f'<path d="{catmull(open_blob(r, sh, ph, gap), closed=False)}" fill="none" stroke="{ink}" '
        f'stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round"/>'
    ]
    if "inner" in spec:
        r2, sh2, ph2 = spec["inner"]
        parts.append(
            f'<path d="{catmull(blob(r2, sh2, ph2), closed=True)}" fill="none" stroke="{ink}" '
            f'stroke-width="{sw}" stroke-linejoin="round"/>'
        )
    dx, dy = dot_center(spec)
    parts.append(f'<circle cx="{f(dx)}" cy="{f(dy)}" r="{f(spec["dot"])}" fill="{ink}"/>')
    return "".join(parts)


def tile_path() -> str:
    """Cuadrado redondeado 1024 con radio 0.32 × lado (arcos circulares)."""
    r = f(RADIUS)
    e = f(S - RADIUS)
    return f"M{r} 0H{e}A{r} {r} 0 0 1 1024 {r}V{e}A{r} {r} 0 0 1 {e} 1024H{r}A{r} {r} 0 0 1 0 {e}V{r}A{r} {r} 0 0 1 {r} 0Z"


def svg(view: str, body: str, title: str, extra: str = "") -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view}"{extra}>'
        f"<title>{title}</title>{body}</svg>\n"
    )


def symbol(spec: dict, bg: str, ink: str) -> str:
    return f'<path d="{tile_path()}" fill="{bg}"/>' + motif(spec, ink)


def symbol_mono(spec: dict) -> str:
    """Una tinta: cuadrado en currentColor y el motivo calado con una máscara."""
    return (
        '<defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="1024" height="1024">'
        f'<rect width="1024" height="1024" fill="#fff"/>{motif(spec, "#000")}</mask></defs>'
        f'<path d="{tile_path()}" fill="currentColor" mask="url(#m)"/>'
    )


def wordmark(text: str = "parlo") -> tuple[str, float, float]:
    """Contornos del logotipo en Bricolage Grotesque peso 500 (medio). Devuelve (d, avance, altura x)."""
    from fontTools.pens.svgPathPen import SVGPathPen
    from fontTools.pens.transformPen import TransformPen
    from fontTools.ttLib import TTFont
    from fontTools.varLib.instancer import instantiateVariableFont

    font = TTFont(str(FONT))
    font = instantiateVariableFont(font, {"wght": 500, "opsz": 96, "wdth": 100})
    cmap = font.getBestCmap()
    gs = font.getGlyphSet()
    hmtx = font["hmtx"]
    pen = SVGPathPen(gs, ntos=lambda v: f(v))
    x = 0.0
    tracking = -12.0  # leve ajuste negativo: display a tamaño grande
    for ch in text:
        name = cmap[ord(ch)]
        # y de la fuente crece hacia arriba: se invierte; línea base en y = 0.
        gs[name].draw(TransformPen(pen, (1, 0, 0, -1, x, 0)))
        x += hmtx[name][0] + tracking
    x -= tracking
    return pen.getCommands(), x, float(font["OS/2"].sxHeight)


def horizontal(spec: dict, bg: str | None, ink: str, text: str, mono: bool = False) -> str:
    """Símbolo (1024) + logotipo. Altura x del texto = 0,42 × símbolo, centrada en vertical con él, de
    modo que ascendentes («l») y descendentes («p») quedan dentro del alto del símbolo."""
    d, adv, xh = wordmark()
    x_height = 0.42 * S
    scale = x_height / xh
    gap = 0.22 * S  # separación símbolo-texto (el área de respeto de 1/4 va alrededor del conjunto)
    tx = S + gap
    baseline = 512.0 + x_height / 2  # el ojo de las minúsculas queda centrado en el símbolo
    width = tx + adv * scale
    sym = symbol_mono(spec) if mono else symbol(spec, bg or "", ink)
    body = (
        f"{sym}"
        f'<path transform="translate({f(tx)} {f(baseline)}) scale({scale:.4f})" d="{d}" fill="{text}"/>'
    )
    return svg(f"0 0 {f(width)} 1024", body, "Parlo")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    files = {
        "parlo-symbol.svg": svg("0 0 1024 1024", symbol(FULL, DARK_BG, DARK_INK), "Parlo"),
        "parlo-symbol-light.svg": svg("0 0 1024 1024", symbol(FULL, LIGHT_BG, LIGHT_INK), "Parlo"),
        "parlo-symbol-small.svg": svg("0 0 1024 1024", symbol(SMALL, DARK_BG, DARK_INK), "Parlo"),
        "parlo-symbol-small-light.svg": svg("0 0 1024 1024", symbol(SMALL, LIGHT_BG, LIGHT_INK), "Parlo"),
        "parlo-symbol-mono.svg": svg("0 0 1024 1024", symbol_mono(FULL), "Parlo", ' fill="currentColor"'),
        "parlo-glyph.svg": svg("0 0 1024 1024", motif(FULL, "currentColor"), "Parlo"),
        # Bandeja: sin cuadrado, encuadre ceñido al motivo (720 centrado en él) para aprovechar 16-24 px.
        "parlo-tray-symbolic.svg": svg("164 162 720 720", motif(SMALL, "currentColor"), "Parlo"),
        "parlo-logo-horizontal.svg": horizontal(FULL, DARK_BG, DARK_INK, DARK_TEXT),
        "parlo-logo-horizontal-light.svg": horizontal(FULL, LIGHT_BG, LIGHT_INK, LIGHT_TEXT),
        "parlo-logo-horizontal-mono.svg": horizontal(FULL, None, "", "currentColor", mono=True),
    }
    for name, data in files.items():
        (OUT / name).write_text(data, encoding="utf-8")
    print(f"{len(files)} SVG escritos en {OUT.relative_to(ROOT)}/")


if __name__ == "__main__":
    main()
