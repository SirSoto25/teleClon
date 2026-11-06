//! Traducción de los tokens de `tc-theme` («Brasa») a estilos de iced. Los componentes solo
//! consumen roles de la paleta; aquí no se escribe ningún hex (ver docs/design/system.md §2.1).

use iced::border::Radius;
use iced::widget::{button, checkbox, container, scrollable, text_editor, text_input};
use iced::{Background, Border, Color, Font, Shadow, Theme, Vector, font, gradient};
use tc_theme::tokens::{ELEVATION, FontRole, TextStyle, border_width, radius};
use tc_theme::{Alpha, Alphas, Palette, Rgb, ThemeKind, fonts};

/// Paleta resuelta del tema activo. `Copy` para poder capturarla en los cierres de estilo.
#[derive(Clone, Copy, Debug)]
pub struct Brasa {
    pub kind: ThemeKind,
    pub p: &'static Palette,
    pub a: &'static Alphas,
}

impl Brasa {
    pub fn new(kind: ThemeKind) -> Self {
        Self { kind, p: kind.palette(), a: kind.alphas() }
    }

    /// Tema de iced con los roles básicos (por si algún widget usa el estilo por defecto).
    pub fn iced_theme(self) -> Theme {
        let p = self.p;
        let name = match self.kind {
            ThemeKind::Dark => "Brasa oscuro",
            ThemeKind::Light => "Brasa claro",
        };
        Theme::custom(
            name,
            iced::theme::Palette {
                background: c(p.bg),
                text: c(p.text),
                primary: c(p.accent),
                success: c(p.success),
                warning: c(p.warning),
                danger: c(p.danger),
            },
        )
    }

    /// Color de nombre de remitente/avatar (`name-1…8`) según el id.
    pub fn name_color(self, id: i64) -> Color {
        c(name_rgb(self.p, id))
    }

    /// Avatar sin foto: fondo `name-n` del tema oscuro en ambos temas (§8.2).
    pub fn avatar_color(id: i64) -> Color {
        c(name_rgb(ThemeKind::Dark.palette(), id))
    }
}

fn name_rgb(p: &Palette, id: i64) -> Rgb {
    let names = [p.name_1, p.name_2, p.name_3, p.name_4, p.name_5, p.name_6, p.name_7, p.name_8];
    names[(id.unsigned_abs() % 8) as usize]
}

/// Iniciales sobre avatar: `on-accent` del tema oscuro (9:1 sobre cualquier `name-n`).
pub fn avatar_ink() -> Color {
    c(ThemeKind::Dark.palette().on_accent)
}

pub fn c(rgb: Rgb) -> Color {
    Color::from_rgb8(rgb.r, rgb.g, rgb.b)
}

pub fn ca(a: Alpha) -> Color {
    Color { a: a.opacity, ..c(a.rgb) }
}

/// Multiplica la opacidad de un color (animaciones de opacidad).
pub fn fade(color: Color, opacity: f32) -> Color {
    Color { a: color.a * opacity.clamp(0.0, 1.0), ..color }
}

pub const TRANSPARENT: Color = Color::TRANSPARENT;

// ---------------------------------------------------------------- tipografía

pub fn weight(w: u16) -> font::Weight {
    match w {
        0..=450 => font::Weight::Normal,
        451..=550 => font::Weight::Medium,
        551..=650 => font::Weight::Semibold,
        _ => font::Weight::Bold,
    }
}

/// Fuente de un estilo tipográfico (familia por rol + peso de la fuente variable).
pub fn font_of(style: TextStyle) -> Font {
    let family = match style.font {
        FontRole::Ui => fonts::UI_FAMILY,
        FontRole::Display => fonts::DISPLAY_FAMILY,
        FontRole::Mono => fonts::MONO_FAMILY,
        FontRole::Emoji => fonts::EMOJI_FAMILY,
    };
    Font { weight: weight(style.weight), ..Font::with_name(family) }
}

pub fn ui_font() -> Font {
    Font::with_name(fonts::UI_FAMILY)
}

/// Estilo con otro peso (p. ej. nombre de chat no leído).
pub const fn with_weight(style: TextStyle, w: u16) -> TextStyle {
    TextStyle { weight: w, ..style }
}

// ---------------------------------------------------------------- geometría

pub fn shadow(b: Brasa, level: usize) -> Shadow {
    match ELEVATION.get(level).and_then(|e| e.shadow) {
        Some(s) if level >= 2 => Shadow {
            color: Color { a: b.a.shadow.opacity * s.alpha_scale, ..c(b.a.shadow.rgb) },
            offset: Vector::new(0.0, s.y),
            blur_radius: s.blur,
        },
        // Nivel 1 se separa por borde, no por sombra (§5).
        _ => Shadow::default(),
    }
}

pub fn r(all: f32) -> Radius {
    Radius::from(all)
}

/// Radio de avatar: cuadrado redondeado, 0.32 × lado.
pub fn avatar_radius(size: f32) -> Radius {
    Radius::from(size * radius::AVATAR_RATIO)
}

/// Posición de una burbuja dentro de su grupo (misma persona, < 5 min).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupPos {
    Single,
    First,
    Middle,
    Last,
}

/// Radios de burbuja: 18 salvo las esquinas del lado del remitente que unen con la vecina (6). Sin cola.
pub fn bubble_radius(outgoing: bool, pos: GroupPos) -> Radius {
    let (b, j) = (radius::BUBBLE, radius::BUBBLE_JOINT);
    let (top_join, bottom_join) = match pos {
        GroupPos::Single => (false, false),
        GroupPos::First => (false, true),
        GroupPos::Middle => (true, true),
        GroupPos::Last => (true, false),
    };
    let top = if top_join { j } else { b };
    let bottom = if bottom_join { j } else { b };
    if outgoing {
        Radius { top_left: b, top_right: top, bottom_right: bottom, bottom_left: b }
    } else {
        Radius { top_left: top, top_right: b, bottom_right: b, bottom_left: bottom }
    }
}

// ---------------------------------------------------------------- contenedores

pub fn app_style(b: Brasa) -> iced::theme::Style {
    iced::theme::Style { background_color: c(b.p.bg), text_color: c(b.p.text) }
}

pub fn surface(b: Brasa) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(c(b.p.surface).into()),
        text_color: Some(c(b.p.text)),
        ..Default::default()
    }
}

/// Superficie de nivel 1 con borde hairline en un lado (`right`, `bottom`, `top`): se simula con
/// un contenedor exterior de color `border` y relleno de 1 px en ese lado.
pub fn border_fill(b: Brasa) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(c(b.p.border).into()), ..Default::default() }
}

pub fn fill(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(color.into()), ..Default::default() }
}

pub fn rounded_fill(color: Color, radius: Radius) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(color.into()),
        border: Border { radius, ..Default::default() },
        ..Default::default()
    }
}

/// Píldora flotante (separador de fecha, contador): `surface-raised`, elevación 2.
pub fn pill_raised(b: Brasa) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(c(b.p.surface_raised).into()),
        border: Border { radius: r(radius::PILL), width: border_width::HAIRLINE, color: c(b.p.border) },
        shadow: shadow(b, 2),
        text_color: Some(c(b.p.text_muted)),
        snap: true,
    }
}

/// Panel elevado (menús, diálogos, barra de acciones flotante).
pub fn raised(b: Brasa, level: usize, rad: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(c(b.p.surface_raised).into()),
        border: Border { radius: r(rad), width: border_width::HAIRLINE, color: c(b.p.border) },
        shadow: shadow(b, level),
        text_color: Some(c(b.p.text)),
        snap: true,
    }
}

/// Burbuja: propia con degradado vertical `bubble-out` → `bubble-out-end`; ajena plana.
pub fn bubble(b: Brasa, outgoing: bool, pos: GroupPos, opacity: f32, menu_open: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let background = if outgoing {
            let (top, bottom) = tc_theme::bubble_out_gradient(b.kind);
            Background::Gradient(
                gradient::Linear::new(std::f32::consts::PI)
                    .add_stop(0.0, fade(c(top), opacity))
                    .add_stop(1.0, fade(c(bottom), opacity))
                    .into(),
            )
        } else {
            Background::Color(fade(c(b.p.bubble_in), opacity))
        };
        container::Style {
            background: Some(background),
            border: Border {
                radius: bubble_radius(outgoing, pos),
                width: if menu_open { border_width::HAIRLINE } else { 0.0 },
                color: c(b.p.border_strong),
            },
            text_color: Some(fade(c(if outgoing { b.p.on_bubble_out } else { b.p.on_bubble_in }), opacity)),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Cita de respuesta dentro de una burbuja: fondo `pressed`, radio `sm`.
pub fn quote(b: Brasa) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(ca(b.a.pressed).into()),
        border: Border { radius: r(radius::SM), ..Default::default() },
        ..Default::default()
    }
}

pub fn badge(b: Brasa, muted: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let (bg, fg) = if muted { (b.p.border_strong, b.p.text) } else { (b.p.accent, b.p.on_accent) };
        container::Style {
            background: Some(c(bg).into()),
            text_color: Some(c(fg)),
            border: Border { radius: r(radius::PILL), ..Default::default() },
            ..Default::default()
        }
    }
}

/// Campo hundido (buscador, compositor).
pub fn sunken_field(b: Brasa, focused: bool, rad: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(c(b.p.surface_sunken).into()),
        border: Border {
            radius: r(rad),
            width: if focused { border_width::FOCUS } else { border_width::INPUT },
            color: c(if focused { b.p.focus_ring } else { b.p.border }),
        },
        ..Default::default()
    }
}

pub fn scrim(b: Brasa, opacity: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(fade(ca(b.a.scrim), opacity).into()), ..Default::default() }
}

// ---------------------------------------------------------------- botones

/// Botón fantasma / icono: transparente, hover `surface-hover`, pulsado capa `pressed`.
pub fn ghost(b: Brasa, rad: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = match status {
            button::Status::Hovered => Some(c(b.p.surface_hover).into()),
            button::Status::Pressed => Some(ca(b.a.pressed).into()),
            _ => None,
        };
        button::Style {
            background: bg,
            text_color: c(if status == button::Status::Disabled { b.p.text_placeholder } else { b.p.text }),
            border: Border { radius: r(rad), ..Default::default() },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Fila de lista de chats: default `surface`, hover `surface-hover`, seleccionada `surface-selected`.
pub fn chat_row(b: Brasa, selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if selected {
            b.p.surface_selected
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed => b.p.surface_hover,
                _ => b.p.surface,
            }
        };
        button::Style {
            background: Some(c(bg).into()),
            text_color: c(b.p.text),
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Primario: relleno `accent`, hover `accent-hover`, pulsado `accent-pressed`; deshabilitado al 45 %.
pub fn primary(b: Brasa, rad: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let (bg, alpha) = match status {
            button::Status::Hovered => (b.p.accent_hover, 1.0),
            button::Status::Pressed => (b.p.accent_pressed, 1.0),
            button::Status::Disabled => (b.p.accent, 0.45),
            button::Status::Active => (b.p.accent, 1.0),
        };
        button::Style {
            background: Some(fade(c(bg), alpha).into()),
            text_color: fade(c(b.p.on_accent), alpha),
            border: Border { radius: r(rad), ..Default::default() },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Botón enviar deshabilitado (compositor vacío): `border-strong`, sin anillo.
pub fn send_disabled(b: Brasa) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, _| button::Style {
        background: Some(c(b.p.border_strong).into()),
        text_color: c(b.p.text_muted),
        border: Border { radius: r(radius::PILL), ..Default::default() },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn danger_solid(b: Brasa) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let alpha = if status == button::Status::Hovered { 0.9 } else { 1.0 };
        button::Style {
            background: Some(fade(c(b.p.danger_solid), alpha).into()),
            text_color: c(b.p.on_danger),
            border: Border { radius: r(radius::MD), ..Default::default() },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Píldora de reacción: `surface-raised`, borde `border`; la propia con borde `accent`.
pub fn reaction_chip(b: Brasa, mine: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(c(if status == button::Status::Hovered { b.p.surface_hover } else { b.p.surface_raised }).into()),
        text_color: c(b.p.text),
        border: Border {
            radius: r(radius::PILL),
            width: border_width::HAIRLINE,
            color: c(if mine { b.p.accent } else { b.p.border }),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Celda del raíl de carpetas.
pub fn rail_cell(b: Brasa) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: match status {
            button::Status::Hovered => Some(c(b.p.surface_hover).into()),
            button::Status::Pressed => Some(ca(b.a.pressed).into()),
            _ => None,
        },
        text_color: c(b.p.text_muted),
        border: Border { radius: r(radius::MD), ..Default::default() },
        shadow: Shadow::default(),
        snap: true,
    }
}

// ---------------------------------------------------------------- campos

/// Campo de texto transparente: el marco lo pinta el contenedor (`sunken_field`).
pub fn bare_input(b: Brasa) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, _| text_input::Style {
        background: TRANSPARENT.into(),
        border: Border::default(),
        icon: c(b.p.text_muted),
        placeholder: c(b.p.text_placeholder),
        value: c(b.p.text),
        selection: ca(b.a.selection),
    }
}

/// Compositor: `surface-sunken`, radio `lg`, borde `border` → `border-input` + anillo en foco.
pub fn composer(b: Brasa) -> impl Fn(&Theme, text_editor::Status) -> text_editor::Style {
    move |_, status| {
        let focused = matches!(status, text_editor::Status::Focused { .. });
        text_editor::Style {
            background: c(b.p.surface_sunken).into(),
            border: Border {
                radius: r(radius::LG),
                width: if focused { border_width::FOCUS } else { border_width::INPUT },
                color: c(if focused { b.p.focus_ring } else { b.p.border }),
            },
            placeholder: c(b.p.text_placeholder),
            value: c(b.p.text),
            selection: ca(b.a.selection),
        }
    }
}

pub fn check(b: Brasa) -> impl Fn(&Theme, checkbox::Status) -> checkbox::Style {
    move |_, status| {
        let on = match status {
            checkbox::Status::Active { is_checked }
            | checkbox::Status::Hovered { is_checked }
            | checkbox::Status::Disabled { is_checked } => is_checked,
        };
        checkbox::Style {
            background: c(if on { b.p.accent } else { b.p.surface_sunken }).into(),
            icon_color: c(b.p.on_accent),
            border: Border {
                radius: r(radius::XS),
                width: border_width::INPUT,
                color: c(if on { b.p.accent } else { b.p.border_input }),
            },
            text_color: Some(c(b.p.text)),
        }
    }
}

pub fn thin_scroll(b: Brasa) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |_, status| {
        let active = !matches!(status, scrollable::Status::Active { .. });
        let rail = scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: fade(c(b.p.border_strong), if active { 1.0 } else { 0.7 }).into(),
                border: Border { radius: r(radius::PILL), ..Default::default() },
            },
        };
        scrollable::Style {
            container: container::Style::default(),
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: c(b.p.surface_raised).into(),
                border: Border { radius: r(radius::PILL), ..Default::default() },
                shadow: shadow(b, 2),
                icon: c(b.p.text),
            },
        }
    }
}

// ---------------------------------------------------------------- login, stickers, chats secretos

/// Campo de formulario (login, filtros): `surface-sunken`, borde `border-input`, hover
/// `border-strong`, foco con anillo 2 px `focus-ring`; con error, borde `danger` (§8.8).
pub fn field_input(b: Brasa, error: bool) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, status| {
        let (width, color) = match status {
            text_input::Status::Focused { .. } => (border_width::FOCUS, if error { b.p.danger } else { b.p.focus_ring }),
            _ if error => (border_width::FOCUS, b.p.danger),
            text_input::Status::Hovered => (border_width::INPUT, b.p.border_strong),
            _ => (border_width::INPUT, b.p.border_input),
        };
        text_input::Style {
            background: c(b.p.surface_sunken).into(),
            border: Border { radius: r(radius::MD), width, color: c(color) },
            icon: c(b.p.text_muted),
            placeholder: c(b.p.text_placeholder),
            value: c(b.p.text),
            selection: ca(b.a.selection),
        }
    }
}

/// Celda del selector de stickers: hover `surface-hover`; el cursor de teclado lleva el anillo de foco.
pub fn sticker_cell(b: Brasa, focused: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: match status {
            button::Status::Hovered => Some(c(b.p.surface_hover).into()),
            button::Status::Pressed => Some(ca(b.a.pressed).into()),
            _ if focused => Some(c(b.p.surface_hover).into()),
            _ => None,
        },
        text_color: c(b.p.text),
        border: Border {
            radius: r(radius::MD),
            width: if focused { border_width::FOCUS } else { 0.0 },
            color: c(b.p.focus_ring),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Pestaña (selector de stickers) u opción de una lista (TTL, contactos): la activa en `surface-selected`.
pub fn option_row(b: Brasa, active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if active {
            Some(c(b.p.surface_selected).into())
        } else {
            match status {
                button::Status::Hovered => Some(c(b.p.surface_hover).into()),
                button::Status::Pressed => Some(ca(b.a.pressed).into()),
                _ => None,
            }
        };
        button::Style {
            background: bg,
            text_color: c(b.p.text),
            border: Border { radius: r(radius::MD), width: if active { border_width::HAIRLINE } else { 0.0 }, color: c(b.p.border_strong) },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Etiqueta «EXPERIMENTAL»: texto `warning` sobre `warning` al 14 %, borde `warning`, píldora.
pub fn experimental_pill(b: Brasa) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(fade(c(b.p.warning), 0.14).into()),
        border: Border { radius: r(radius::PILL), width: border_width::HAIRLINE, color: c(b.p.warning) },
        text_color: Some(c(b.p.warning)),
        ..Default::default()
    }
}

/// Franja de aviso bajo la cabecera (chats secretos): `warning` al 10 % sobre la superficie.
pub fn warning_banner(b: Brasa) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(fade(c(b.p.warning), 0.10).into()),
        text_color: Some(c(b.p.text)),
        ..Default::default()
    }
}

/// QR: siempre módulos `#211c17` sobre `#ffffff` en ambos temas (§8, inicio de sesión), para que
/// lo lea cualquier cámara. Son los dos únicos colores fijos fuera de la paleta, por diseño.
pub const QR_INK: Color = Color::from_rgb(0x21 as f32 / 255.0, 0x1c as f32 / 255.0, 0x17 as f32 / 255.0);
pub const QR_PAPER: Color = Color::WHITE;
