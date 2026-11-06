use super::*;
use serde_json::Value;

fn tokens() -> Value {
    let raw = include_str!("../../../docs/design/tokens.json");
    serde_json::from_str(raw).expect("tokens.json válido")
}

fn f(v: &Value) -> f32 {
    v.as_f64().expect("número") as f32
}

#[test]
fn colores_coinciden_con_tokens_json() {
    let t = tokens();
    for (name, kind) in [("dark", ThemeKind::Dark), ("light", ThemeKind::Light)] {
        let json = t["color"][name].as_object().unwrap();
        let pal = kind.palette();
        assert_eq!(pal.roles().len(), json.len(), "roles de {name}");
        for (role, entry) in json {
            let want = Rgb::parse(entry["value"].as_str().unwrap()).unwrap();
            assert_eq!(pal.get(role), Some(want), "{name}/{role}");
        }
        for (key, entry) in t["alpha"][name].as_object().unwrap() {
            let a = match key.as_str() {
                "scrim" => kind.alphas().scrim,
                "shadow" => kind.alphas().shadow,
                "highlight" => kind.alphas().highlight,
                "selection" => kind.alphas().selection,
                "pressed" => kind.alphas().pressed,
                k => panic!("alpha desconocido {k}"),
            };
            assert_eq!(a.rgb, Rgb::parse(entry["value"].as_str().unwrap()).unwrap(), "{name}/{key}");
            assert_eq!(a.opacity, f(&entry["opacity"]), "{name}/{key} opacidad");
        }
    }
}

#[test]
fn espaciado_radios_bordes_y_elevacion_coinciden() {
    use tokens::*;
    let t = tokens();
    let sp = &t["space"];
    for (k, v) in [
        ("0", space::S0), ("0.5", space::S0_5), ("1", space::S1), ("1.5", space::S1_5), ("2", space::S2),
        ("3", space::S3), ("4", space::S4), ("5", space::S5), ("6", space::S6), ("8", space::S8),
        ("10", space::S10), ("12", space::S12), ("16", space::S16),
    ] {
        assert_eq!(f(&sp[k]), v, "space/{k}");
    }
    assert_eq!(sp.as_object().unwrap().len(), 13);
    let r = &t["radius"];
    for (k, v) in [
        ("xs", radius::XS), ("sm", radius::SM), ("md", radius::MD), ("lg", radius::LG), ("xl", radius::XL),
        ("pill", radius::PILL), ("avatar_ratio", radius::AVATAR_RATIO), ("bubble", radius::BUBBLE),
        ("bubble_joint", radius::BUBBLE_JOINT),
    ] {
        assert_eq!(f(&r[k]), v, "radius/{k}");
    }
    let b = &t["border_width"];
    for (k, v) in [
        ("hairline", border_width::HAIRLINE), ("input", border_width::INPUT), ("focus", border_width::FOCUS),
        ("focus_offset", border_width::FOCUS_OFFSET), ("selected_bar", border_width::SELECTED_BAR),
    ] {
        assert_eq!(f(&b[k]), v, "border_width/{k}");
    }
    for (i, e) in ELEVATION.iter().enumerate() {
        let j = &t["elevation"][i.to_string()];
        assert_eq!(j["surface"].as_str().unwrap(), e.surface);
        match e.shadow {
            None => assert!(j["shadow"].is_null()),
            Some(s) => {
                assert_eq!(f(&j["shadow"]["y"]), s.y);
                assert_eq!(f(&j["shadow"]["blur"]), s.blur);
                assert_eq!(f(&j["shadow"]["alpha_scale"]), s.alpha_scale);
            }
        }
    }
}

#[test]
fn tipografia_y_fuentes_coinciden() {
    use tokens::text::*;
    let t = tokens();
    for (k, s) in [
        ("caption", CAPTION), ("meta", META), ("ui", UI), ("ui-strong", UI_STRONG), ("body", BODY),
        ("title", TITLE), ("heading", HEADING), ("display", DISPLAY), ("emoji-only", EMOJI_ONLY), ("code", CODE),
    ] {
        let j = &t["type"][k];
        assert_eq!(f(&j["size"]), s.size, "{k} size");
        assert_eq!(f(&j["line"]), s.line, "{k} line");
        assert_eq!(j["weight"].as_u64().unwrap() as u16, s.weight, "{k} weight");
        assert_eq!(f(&j["tracking"]), s.tracking, "{k} tracking");
        let font = match j["font"].as_str().unwrap_or("ui") {
            "ui" => FontRole::Ui,
            "display" => FontRole::Display,
            "mono" => FontRole::Mono,
            "emoji" => FontRole::Emoji,
            o => panic!("fuente {o}"),
        };
        assert_eq!(font, s.font, "{k} font");
    }
    let fam = &t["font"];
    assert_eq!(fam["ui"]["family"], fonts::UI_FAMILY);
    assert_eq!(fam["display"]["family"], fonts::DISPLAY_FAMILY);
    assert_eq!(fam["mono"]["family"], fonts::MONO_FAMILY);
    assert_eq!(fam["emoji"]["family"], fonts::EMOJI_FAMILY);
    for (i, n) in fonts::UI_FALLBACK.iter().enumerate() {
        assert_eq!(fam["ui"]["fallback"][i], *n);
    }
    let su = &t["type_scale_user"];
    assert_eq!(f(&su["min"]), tokens::type_scale_user::MIN);
    assert_eq!(f(&su["max"]), tokens::type_scale_user::MAX);
    assert_eq!(f(&su["step"]), tokens::type_scale_user::STEP);
    assert_eq!(f(&su["default"]), tokens::type_scale_user::DEFAULT);
    // Las fuentes incluidas son ficheros TrueType/OpenType reales.
    for bytes in fonts::BUNDLED {
        assert!(bytes.len() > 10_000);
        assert!(matches!(&bytes[..4], [0, 1, 0, 0] | b"true" | b"OTTO"));
    }
}

#[test]
fn layout_coincide() {
    use tokens::layout::*;
    let l = &tokens()["layout"];
    for (v, want) in [
        (&l["rail_width"], RAIL_WIDTH), (&l["list_width"]["default"], LIST_WIDTH_DEFAULT),
        (&l["list_width"]["min"], LIST_WIDTH_MIN), (&l["list_width"]["max"], LIST_WIDTH_MAX),
        (&l["info_panel_width"], INFO_PANEL_WIDTH), (&l["header_height"], HEADER_HEIGHT),
        (&l["composer_min_height"], COMPOSER_MIN_HEIGHT), (&l["bubble_max_width"], BUBBLE_MAX_WIDTH),
        (&l["bubble_max_ratio"], BUBBLE_MAX_RATIO), (&l["chat_column_max"], CHAT_COLUMN_MAX),
        (&l["breakpoints"]["split"], BREAKPOINT_SPLIT), (&l["breakpoints"]["triple"], BREAKPOINT_TRIPLE),
        (&l["chat_item_height"]["comfortable"], CHAT_ITEM_HEIGHT_COMFORTABLE),
        (&l["chat_item_height"]["compact"], CHAT_ITEM_HEIGHT_COMPACT),
        (&l["avatar"]["xs"], AVATAR_XS), (&l["avatar"]["sm"], AVATAR_SM), (&l["avatar"]["md"], AVATAR_MD),
        (&l["avatar"]["lg"], AVATAR_LG), (&l["avatar"]["xl"], AVATAR_XL),
        (&l["icon"]["sm"], ICON_SM), (&l["icon"]["md"], ICON_MD), (&l["icon"]["lg"], ICON_LG),
        (&l["hit_target_min"], HIT_TARGET_MIN),
    ] {
        assert_eq!(f(v), want);
    }
    assert_eq!(l["composer_max_lines"].as_u64().unwrap() as u32, COMPOSER_MAX_LINES);
}

#[test]
fn movimiento_coincide() {
    use motion::*;
    let m = &tokens()["motion"];
    for (k, s) in [
        ("rapido", Spring::RAPIDO), ("estandar", Spring::ESTANDAR), ("suave", Spring::SUAVE),
        ("rebote", Spring::REBOTE),
    ] {
        let j = &m["springs"][k];
        assert_eq!(f(&j["stiffness"]), s.stiffness, "{k}");
        assert_eq!(f(&j["damping"]), s.damping, "{k}");
        assert_eq!(f(&j["mass"]), s.mass, "{k}");
    }
    for (k, e) in [
        ("rapido", Easing::RAPIDO), ("estandar", Easing::ESTANDAR), ("suave", Easing::SUAVE),
        ("salida", Easing::SALIDA),
    ] {
        let j = &m["easing_fallback"][k];
        assert_eq!(j["ms"].as_u64().unwrap() as u32, e.ms, "{k}");
        for i in 0..4 {
            assert_eq!(f(&j["bezier"][i]), e.bezier[i], "{k}[{i}]");
        }
    }
    let x = &m["exceptions"];
    assert_eq!(x["highlight_fade_ms"].as_u64().unwrap() as u32, HIGHLIGHT_FADE_MS);
    assert_eq!(x["typing_dots_period_ms"].as_u64().unwrap() as u32, TYPING_DOTS_PERIOD_MS);
    assert_eq!(x["skeleton_pulse_period_ms"].as_u64().unwrap() as u32, SKELETON_PULSE_PERIOD_MS);
}

#[test]
fn muelles_convergen_y_rebote_se_pasa() {
    use motion::*;
    for (k, s) in [("rapido", Spring::RAPIDO), ("estandar", Spring::ESTANDAR), ("suave", Spring::SUAVE)] {
        let want = f(&tokens()["motion"]["springs"][k]["settle_ms"]);
        let got = s.settle_ms(0.02);
        assert!((got - want).abs() <= want * 0.35, "{k}: {got} vs {want}");
    }
    // Interrumpible y con overshoot solo notable en `rebote`.
    let peak = |s: Spring| {
        let mut st = SpringState::default();
        let mut p = 0.0_f32;
        for _ in 0..500 {
            st = s.step(st, 1.0, 0.001);
            p = p.max(st.position);
        }
        p
    };
    assert!(peak(Spring::REBOTE) > 1.02);
    assert!(peak(Spring::RAPIDO) < 1.01);
}

#[test]
fn movimiento_reducido() {
    use motion::*;
    let full = MotionPreference::Full;
    let red = MotionPreference::from_reduced(true);
    assert!(red.is_reduced() && !full.is_reduced());
    assert_eq!(full.duration_ms(180, Property::Position), 180);
    assert_eq!(red.duration_ms(180, Property::Position), 0);
    assert_eq!(red.duration_ms(180, Property::Scale), 0);
    assert_eq!(red.duration_ms(60, Property::Opacity), 60);
    assert_eq!(red.duration_ms(900, Property::Opacity), 100);
    assert!(!red.animates(Property::Scale) && red.animates(Property::Opacity));
}

#[test]
fn contraste_wcag_aa_pares_de_texto_principales() {
    let main = [
        ("text", "bg"), ("text-muted", "bg"), ("text", "surface"), ("text-muted", "surface"),
        ("text", "surface-raised"), ("text-muted", "surface-raised"), ("text-placeholder", "surface"),
        ("accent-text", "surface"), ("on-accent", "accent"), ("on-bubble-out", "bubble-out"),
        ("on-bubble-out", "bubble-out-end"), ("bubble-out-meta", "bubble-out"),
        ("bubble-out-meta", "bubble-out-end"), ("on-bubble-in", "bubble-in"),
        ("bubble-in-meta", "bubble-in"), ("on-danger", "danger-solid"),
    ];
    for kind in [ThemeKind::Dark, ThemeKind::Light] {
        let p = kind.palette();
        for (fg, bg) in main {
            let c = p.get(fg).unwrap().contrast(p.get(bg).unwrap());
            assert!(c >= AA_NORMAL, "{kind:?} {fg} sobre {bg}: {c:.2}");
        }
    }
}

#[test]
fn contraste_todos_los_pares_declarados_en_tokens_json() {
    let t = tokens();
    let pairs = t["pairs"].as_array().unwrap();
    assert!(pairs.len() > 100);
    for p in pairs {
        let (fg, bg, kind) = (p[0].as_str().unwrap(), p[1].as_str().unwrap(), p[2].as_str().unwrap());
        let split = |s: &str| {
            let (th, role) = s.split_once('-').unwrap();
            let k = if th == "dark" { ThemeKind::Dark } else { ThemeKind::Light };
            (k, role.to_string())
        };
        let ((k1, r1), (k2, r2)) = (split(fg), split(bg));
        assert_eq!(k1, k2);
        let c = k1.palette().get(&r1).unwrap().contrast(k1.palette().get(&r2).unwrap());
        let min = if kind == "ui" { AA_UI } else { AA_NORMAL };
        assert!(c >= min, "{fg} sobre {bg}: {c:.2} < {min}");
    }
}

#[test]
fn contraste_formula_conocida() {
    let blanco = Rgb::hex(0xffffff);
    let negro = Rgb::hex(0x000000);
    assert!((blanco.contrast(negro) - 21.0).abs() < 1e-9);
    assert!((blanco.contrast(blanco) - 1.0).abs() < 1e-9);
}

#[test]
fn patron_tokens_y_determinismo() {
    use pattern::*;
    let p = &tokens()["pattern"];
    assert_eq!(f(&p["spacing"]), ContourParams::DEFAULT.spacing);
    assert_eq!(f(&p["stroke"]), ContourParams::DEFAULT.stroke);
    assert_eq!(f(&p["noise_scale"]), ContourParams::DEFAULT.noise_scale);
    assert_eq!(ContourParams::SECRET.spacing * 2.0, ContourParams::DEFAULT.spacing);

    let a = contours(42, 400.0, 300.0, ContourParams::DEFAULT);
    let b = contours(42, 400.0, 300.0, ContourParams::DEFAULT);
    assert!(!a.is_empty());
    assert_eq!(a, b, "mismo id, mismo patrón");
    let c = contours(43, 400.0, 300.0, ContourParams::DEFAULT);
    assert_ne!(a, c, "otro id, otro patrón");
    let neg = contours(-1_001_234_567_890, 400.0, 300.0, ContourParams::DEFAULT);
    assert!(!neg.is_empty());
}

#[test]
fn patron_geometria_y_densidad() {
    use pattern::*;
    let (w, h) = (500.0, 400.0);
    let normal = contours(7, w, h, ContourParams::DEFAULT);
    let secreto = contours(7, w, h, ContourParams::SECRET);
    assert!(secreto.len() > normal.len() * 3 / 2, "{} vs {}", secreto.len(), normal.len());
    let eps = 1e-3;
    for s in &normal {
        for (x, y) in [s.a, s.b] {
            assert!(x >= -eps && x <= w + 8.0 + eps && y >= -eps && y <= h + 8.0 + eps);
        }
        // Cada extremo está sobre un nivel exacto del campo (interpolación lineal ≈ nivel).
    }
    assert!(contours(7, 0.0, 100.0, ContourParams::DEFAULT).is_empty());
    // El patrón es un prefijo coherente: un lienzo mayor contiene los segmentos del menor.
    let pequeno = contours(7, 100.0, 100.0, ContourParams::DEFAULT);
    let grande = contours(7, 200.0, 200.0, ContourParams::DEFAULT);
    assert!(pequeno.iter().all(|s| grande.contains(s)));
    // El campo está acotado.
    for i in 0..50 {
        let v = pattern::field(7, i as f32 * 37.0, i as f32 * 53.0, 0.004);
        assert!((0.0..=224.0).contains(&v));
    }
}
