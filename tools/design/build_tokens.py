#!/usr/bin/env python3
import json, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from palette import build

color, pairs, _ = build()

tokens = {
  "_about": "Tokens de diseño de teleClon, dirección 'Brasa' (ADR 0002). Fuente de verdad: OKLCH [L, C, H]; 'value' es el sRGB hex ya mapeado a gama (reducción de croma). Los nombres son roles estables; replica-brand solo cambia valores. Verificar contraste: python3 contrast.py docs/design/tokens.json. Unidades: px lógicos (escala de iced), ms, rigidez/amortiguación con masa 1. Sombra de elevación = alpha.<tema>.shadow con opacity * alpha_scale. Claves que empiezan por _ son comentarios.",
  "meta": {"name": "Brasa", "version": "1.0.0", "date": "2026-11-06", "themes": ["dark", "light"], "default_theme": "system", "spec": "docs/design/system.md"},
  "color": color,
  "alpha": {
    "dark":  {"scrim": {"value": "#060403", "opacity": 0.62}, "shadow": {"value": "#000000", "opacity": 0.45}, "highlight": {"value": "#f67e42", "opacity": 0.18}, "selection": {"value": "#f67e42", "opacity": 0.30}, "pressed": {"value": "#ffffff", "opacity": 0.06}},
    "light": {"scrim": {"value": "#211c17", "opacity": 0.40}, "shadow": {"value": "#3a2a1c", "opacity": 0.14}, "highlight": {"value": "#c64913", "opacity": 0.14}, "selection": {"value": "#c64913", "opacity": 0.22}, "pressed": {"value": "#211c17", "opacity": 0.05}}
  },
  "gradient": {
    "bubble-out": {"angle_deg": 180, "stops": [[0.0, "bubble-out"], [1.0, "bubble-out-end"]], "note": "Lineal vertical; extremos ya verificados con texto y meta."}
  },
  "pairs": pairs,
  "font": {
    "ui":      {"family": "Atkinson Hyperlegible Next", "fallback": ["IBM Plex Sans", "Noto Sans", "Cantarell", "sans-serif"], "license": "OFL-1.1", "bundle": True},
    "display": {"family": "Bricolage Grotesque", "fallback": ["Atkinson Hyperlegible Next", "sans-serif"], "license": "OFL-1.1", "bundle": True, "use": "solo onboarding, estados vacíos y títulos de perfil"},
    "mono":    {"family": "JetBrains Mono", "fallback": ["Noto Sans Mono", "DejaVu Sans Mono", "monospace"], "license": "OFL-1.1", "bundle": True},
    "emoji":   {"family": "Noto Color Emoji", "license": "OFL-1.1", "bundle": False}
  },
  "type": {
    "caption":   {"size": 12, "line": 16, "weight": 500, "tracking": 0.2},
    "meta":      {"size": 13, "line": 18, "weight": 400, "tracking": 0.0},
    "ui":        {"size": 14, "line": 20, "weight": 400, "tracking": 0.0},
    "ui-strong": {"size": 14, "line": 20, "weight": 600, "tracking": 0.0},
    "body":      {"size": 15, "line": 22, "weight": 400, "tracking": 0.0},
    "title":     {"size": 16, "line": 22, "weight": 600, "tracking": -0.1},
    "heading":   {"size": 20, "line": 26, "weight": 700, "tracking": -0.2},
    "display":   {"size": 28, "line": 34, "weight": 700, "tracking": -0.4, "font": "display"},
    "emoji-only":{"size": 40, "line": 46, "weight": 400, "tracking": 0.0, "font": "emoji"},
    "code":      {"size": 13, "line": 20, "weight": 400, "tracking": 0.0, "font": "mono"}
  },
  "type_scale_user": {"min": 0.875, "max": 1.25, "step": 0.125, "default": 1.0},
  "space": {"0": 0, "0.5": 2, "1": 4, "1.5": 6, "2": 8, "3": 12, "4": 16, "5": 20, "6": 24, "8": 32, "10": 40, "12": 48, "16": 64},
  "radius": {"xs": 4, "sm": 8, "md": 12, "lg": 18, "xl": 24, "pill": 999, "avatar_ratio": 0.32, "bubble": 18, "bubble_joint": 6},
  "border_width": {"hairline": 1, "input": 1, "focus": 2, "focus_offset": 2, "selected_bar": 3},
  "elevation": {
    "0": {"surface": "bg",             "shadow": None},
    "1": {"surface": "surface",        "shadow": {"y": 1, "blur": 2,  "alpha_scale": 0.5}},
    "2": {"surface": "surface-raised", "shadow": {"y": 4, "blur": 12, "alpha_scale": 0.8}},
    "3": {"surface": "surface-raised", "shadow": {"y": 12, "blur": 32, "alpha_scale": 1.0}}
  },
  "motion": {
    "springs": {
      "rapido":   {"stiffness": 1840, "damping": 77, "mass": 1, "settle_ms": 119, "overshoot_pct": 0.2, "use": "hover, press, toggles, checkbox"},
      "estandar": {"stiffness": 720,  "damping": 47, "mass": 1, "settle_ms": 179, "overshoot_pct": 0.3, "use": "paneles, menús, entrada de burbuja, cambio de chat"},
      "suave":    {"stiffness": 600,  "damping": 45, "mass": 1, "settle_ms": 221, "overshoot_pct": 0.1, "use": "diálogos, toasts, panel de info"},
      "rebote":   {"stiffness": 900,  "damping": 43, "mass": 1, "settle_ms": 220, "overshoot_pct": 3.9, "use": "solo botón enviar, reacción añadida, badge nuevo"}
    },
    "easing_fallback": {
      "rapido":   {"ms": 120, "bezier": [0.2, 0.0, 0.0, 1.0]},
      "estandar": {"ms": 180, "bezier": [0.2, 0.8, 0.2, 1.0]},
      "suave":    {"ms": 220, "bezier": [0.3, 0.0, 0.0, 1.0]},
      "salida":   {"ms": 140, "bezier": [0.4, 0.0, 1.0, 1.0]}
    },
    "exceptions": {"highlight_fade_ms": 900, "typing_dots_period_ms": 1200, "skeleton_pulse_period_ms": 1400},
    "rules": ["animar solo posición, escala y opacidad", "nunca animar layout ni tamaño de texto", "interrumpible: un muelle nuevo parte de la posición y velocidad actuales", "reducir movimiento: duración 0 salvo opacidad <= 100 ms"]
  },
  "layout": {
    "rail_width": 72, "list_width": {"default": 340, "min": 280, "max": 440}, "info_panel_width": 360,
    "header_height": 56, "composer_min_height": 52, "composer_max_lines": 10,
    "bubble_max_width": 560, "bubble_max_ratio": 0.72, "chat_column_max": 820,
    "breakpoints": {"single": 0, "split": 720, "triple": 1180},
    "chat_item_height": {"comfortable": 68, "compact": 52},
    "avatar": {"xs": 20, "sm": 28, "md": 36, "lg": 44, "xl": 88},
    "icon": {"sm": 16, "md": 20, "lg": 24},
    "hit_target_min": 32
  },
  "pattern": {"kind": "contours", "spacing": 28, "stroke": 1, "seed_from": "chat_id", "color": "pattern-line", "noise_scale": 0.004, "redraw": "solo al redimensionar o cambiar de tema"},
  "icons": {"set": "Phosphor", "weight": "regular", "license": "MIT", "active_weight": "fill"}
}

out = sys.argv[1]
with open(out, "w", encoding="utf-8") as fh:
    json.dump(tokens, fh, ensure_ascii=False, indent=2)
print("written", out)
