#!/usr/bin/env python3
"""OKLCH -> sRGB hex for teleClon 'Brasa' palette, with gamut mapping (chroma reduction) and WCAG contrast."""
import json, math, sys

def oklch_to_linear(L, C, h):
    a = C * math.cos(math.radians(h)); b = C * math.sin(math.radians(h))
    l_ = L + 0.3963377774*a + 0.2158037573*b
    m_ = L - 0.1055613458*a - 0.0638541728*b
    s_ = L - 0.0894841775*a - 1.2914855480*b
    l, m, s = l_**3, m_**3, s_**3
    r = 4.0767416621*l - 3.3077115913*m + 0.2309699292*s
    g = -1.2684380046*l + 2.6097574011*m - 0.3413193965*s
    bb = -0.0041960863*l - 0.7034186147*m + 1.7076147010*s
    return r, g, bb

def in_gamut(rgb, eps=1e-4):
    return all(-eps <= c <= 1+eps for c in rgb)

def enc(c):
    c = min(max(c, 0.0), 1.0)
    return 12.92*c if c <= 0.0031308 else 1.055*c**(1/2.4) - 0.055

def to_hex(L, C, h):
    """Returns hex, final chroma, clipped flag."""
    C0 = C
    if not in_gamut(oklch_to_linear(L, C, h)):
        lo, hi = 0.0, C
        for _ in range(40):
            mid = (lo+hi)/2
            if in_gamut(oklch_to_linear(L, mid, h)): lo = mid
            else: hi = mid
        C = lo
    r, g, b = (round(enc(x)*255) for x in oklch_to_linear(L, C, h))
    return "#%02x%02x%02x" % (r, g, b), C, C < C0 - 1e-4

def lum(hexs):
    v = [int(hexs[i:i+2], 16)/255 for i in (1, 3, 5)]
    v = [c/12.92 if c <= 0.04045 else ((c+0.055)/1.055)**2.4 for c in v]
    return 0.2126*v[0] + 0.7152*v[1] + 0.0722*v[2]

def ratio(a, b):
    x, y = sorted((lum(a), lum(b)), reverse=True)
    return (x+0.05)/(y+0.05)

NAME_HUES = [25, 60, 100, 150, 195, 240, 285, 330]

DARK = {
    "bg":              (0.170, 0.008, 65),
    "surface":         (0.205, 0.009, 65),
    "surface-raised":  (0.250, 0.010, 65),
    "surface-sunken":  (0.150, 0.007, 65),
    "surface-hover":   (0.240, 0.010, 65),
    "surface-selected":(0.300, 0.050, 45),
    "border":          (0.300, 0.010, 65),
    "border-strong":   (0.400, 0.012, 65),
    "border-input":    (0.560, 0.012, 65),
    "text":            (0.950, 0.008, 75),
    "text-muted":      (0.760, 0.012, 70),
    "text-placeholder":(0.680, 0.012, 70),
    "accent":          (0.720, 0.165, 45),
    "accent-hover":    (0.770, 0.150, 48),
    "accent-pressed":  (0.670, 0.160, 43),
    "on-accent":       (0.200, 0.030, 45),
    "accent-text":     (0.790, 0.130, 52),
    "bubble-out":      (0.370, 0.075, 42),
    "bubble-out-end":  (0.335, 0.072, 36),
    "on-bubble-out":   (0.965, 0.015, 60),
    "bubble-out-meta": (0.840, 0.040, 55),
    "tick-read":       (0.880, 0.110, 70),
    "bubble-in":       (0.250, 0.010, 65),
    "on-bubble-in":    (0.950, 0.008, 75),
    "bubble-in-meta":  (0.740, 0.012, 70),
    "danger":          (0.700, 0.180, 22),
    "danger-solid":    (0.580, 0.200, 22),
    "on-danger":       (0.985, 0.005, 22),
    "success":         (0.760, 0.150, 155),
    "warning":         (0.830, 0.140, 85),
    "focus-ring":      (0.820, 0.130, 58),
    "pattern-line":    (0.215, 0.012, 65),
}
LIGHT = {
    "bg":              (0.972, 0.007, 80),
    "surface":         (0.995, 0.003, 80),
    "surface-raised":  (1.000, 0.000, 0),
    "surface-sunken":  (0.952, 0.008, 78),
    "surface-hover":   (0.957, 0.008, 75),
    "surface-selected":(0.935, 0.040, 52),
    "border":          (0.905, 0.008, 75),
    "border-strong":   (0.820, 0.010, 70),
    "border-input":    (0.600, 0.012, 65),
    "text":            (0.230, 0.012, 60),
    "text-muted":      (0.480, 0.014, 65),
    "text-placeholder":(0.530, 0.012, 65),
    "accent":          (0.570, 0.170, 40),
    "accent-hover":    (0.535, 0.165, 38),
    "accent-pressed":  (0.500, 0.150, 36),
    "on-accent":       (0.990, 0.005, 60),
    "accent-text":     (0.530, 0.160, 40),
    "bubble-out":      (0.930, 0.050, 58),
    "bubble-out-end":  (0.910, 0.058, 52),
    "on-bubble-out":   (0.230, 0.020, 50),
    "bubble-out-meta": (0.460, 0.060, 48),
    "tick-read":       (0.500, 0.150, 38),
    "bubble-in":       (1.000, 0.000, 0),
    "on-bubble-in":    (0.230, 0.012, 60),
    "bubble-in-meta":  (0.480, 0.014, 65),
    "danger":          (0.530, 0.200, 25),
    "danger-solid":    (0.540, 0.210, 25),
    "on-danger":       (0.990, 0.005, 25),
    "success":         (0.500, 0.130, 155),
    "warning":         (0.540, 0.120, 72),
    "focus-ring":      (0.560, 0.170, 42),
    "pattern-line":    (0.935, 0.012, 78),
}
for i, h in enumerate(NAME_HUES):
    DARK["name-%d" % (i+1)] = (0.790, 0.120, h)
    LIGHT["name-%d" % (i+1)] = (0.500, 0.140, h)

def pairs_for(p):
    P = []
    def add(fg, bg, size="normal"): P.append([f"{p}-{fg}", f"{p}-{bg}", size])
    for bg in ["bg", "surface", "surface-raised", "surface-sunken", "surface-hover", "surface-selected"]:
        add("text", bg); add("text-muted", bg)
    for bg in ["surface", "surface-sunken", "surface-raised"]:
        add("text-placeholder", bg)
    for bg in ["bg", "surface", "surface-raised", "surface-selected", "bubble-in"]:
        add("accent-text", bg)
    add("on-accent", "accent"); add("on-accent", "accent-hover"); add("on-accent", "accent-pressed")
    add("text", "border-strong"); add("focus-ring", "surface-selected", "ui")
    add("on-bubble-out", "bubble-out"); add("on-bubble-out", "bubble-out-end")
    add("bubble-out-meta", "bubble-out"); add("bubble-out-meta", "bubble-out-end")
    add("tick-read", "bubble-out", "ui"); add("tick-read", "bubble-out-end", "ui")
    add("on-bubble-in", "bubble-in"); add("bubble-in-meta", "bubble-in")
    add("on-danger", "danger-solid")
    for fg in ["danger", "success", "warning"]:
        add(fg, "surface"); add(fg, "surface-raised"); add(fg, "bg")
    for bg in ["surface", "surface-sunken"]:
        add("border-input", bg, "ui")
    for bg in ["bg", "surface", "surface-raised", "bubble-in", "bubble-out"]:
        add("focus-ring", bg, "ui")
    add("accent", "surface", "ui"); add("accent", "bg", "ui")
    add("accent", "bubble-in", "ui"); add("success", "bubble-in"); add("success", "surface-sunken")
    add("on-bubble-out", "bubble-out-end", "ui")
    for i in range(len(NAME_HUES)):
        add("name-%d" % (i+1), "bubble-in"); add("name-%d" % (i+1), "bubble-out")
    return P

def build():
    out, report = {}, []
    for theme, pal in (("dark", DARK), ("light", LIGHT)):
        out[theme] = {}
        for k, (L, C, h) in pal.items():
            hx, Cf, clipped = to_hex(L, C, h)
            out[theme][k] = {"oklch": [round(L, 3), round(Cf, 3), h], "value": hx}
            if clipped: report.append(f"clip {theme}-{k}: C {C}->{Cf:.3f}")
    pairs = pairs_for("dark") + pairs_for("light")
    return out, pairs, report

if __name__ == "__main__":
    col, pairs, rep = build()
    flat = {f"{t}-{k}": v["value"] for t in col for k, v in col[t].items()}
    fails = 0
    for fg, bg, size in pairs:
        r = ratio(flat[fg], flat[bg]); need = 4.5 if size == "normal" else 3.0
        ok = r >= need; fails += not ok
        if not ok or "-v" in sys.argv: print(f"{'OK ' if ok else 'FAIL'} {r:5.2f} {fg} on {bg} ({size})")
    print("\n".join(rep)); print(len(pairs), "pairs,", fails, "fail")
    if "--json" in sys.argv:
        json.dump({"color": col, "pairs": pairs}, open(sys.argv[-1], "w"), indent=1)
