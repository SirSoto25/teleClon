//! Burbujas de audio (Brasa): nota de voz con forma de onda y audio con título y barra de progreso.
//!
//! ```text
//!  ( ▶ )  ▁▃▅▇▅▃▁▂▅▇▇▅▂▁▃▅▃▁      nota de voz: pulsar en la forma de onda salta a ese punto
//!         0:03 / 0:12   1.5x
//! ```
use iced::widget::canvas::{self, Path};
use iced::widget::{button, canvas as canvas_widget, column, container, row};
use iced::{Alignment, Border, Color, Element, Length, Point, Rectangle, Renderer, Shadow, Size, Theme, mouse};
use tc_core::{Media, MediaKind, Message, PeerRef, waveform};
use tc_theme::tokens::{radius, space, text as ts};

use crate::app::{App, Msg};
use crate::audio::{self, AudioMsg, TrackState, TrackView};
use crate::icon::{Icon, icon, icon_fill};
use crate::style::{self, c, fade, with_weight};
use crate::widgets::t1;

const BUTTON: f32 = 40.0;
const WAVE_H: f32 = 22.0;
const BAR_W: f32 = 2.0;
const BAR_GAP: f32 = 2.0;
const PROGRESS_H: f32 = 4.0;

/// Colores de la burbuja según sea saliente o entrante.
#[derive(Clone, Copy)]
struct Ink {
    button_bg: Color,
    button_fg: Color,
    played: Color,
    rest: Color,
    text: Color,
    meta: Color,
    error: Color,
}

fn ink(app: &App, outgoing: bool, opacity: f32) -> Ink {
    let p = app.brasa.p;
    let i = if outgoing {
        let on = c(p.on_bubble_out);
        Ink {
            button_bg: on,
            button_fg: c(p.bubble_out),
            played: on,
            rest: Color { a: 0.45, ..on },
            text: on,
            meta: c(p.bubble_out_meta),
            error: on,
        }
    } else {
        Ink {
            button_bg: c(p.accent),
            button_fg: c(p.on_accent),
            played: c(p.accent),
            rest: Color { a: 0.35, ..c(p.text_muted) },
            text: c(p.text),
            meta: c(p.text_muted),
            error: c(p.danger),
        }
    };
    let f = |x: Color| fade(x, opacity);
    Ink {
        button_bg: f(i.button_bg),
        button_fg: f(i.button_fg),
        played: f(i.played),
        rest: f(i.rest),
        text: f(i.text),
        meta: f(i.meta),
        error: f(i.error),
    }
}

/// Burbuja de una nota de voz o un audio. `sz`: el hueco que reservó la maquetación.
pub fn bubble<'a>(app: &'a App, peer: PeerRef, m: &'a Message, md: &'a Media, sz: Size, opacity: f32) -> Element<'a, Msg> {
    let k = ink(app, m.outgoing, opacity);
    let duration = md.duration_secs.map(|s| std::time::Duration::from_secs(u64::from(s)));
    let local = crate::conv::is_local(m) || md.file.is_none();
    let v = match &md.file {
        Some(f) if !local => app.audio.view(f.id, duration, &app.downloads),
        _ => TrackView { state: TrackState::Idle, active: false, position: Default::default(), progress: 0.0 },
    };

    let play = play_button(&v, k, (!local).then_some(Msg::Audio(AudioMsg::Toggle { peer, msg: m.id })));
    let inner_w = (sz.width - BUTTON - space::S2).max(40.0);

    let total = duration.map(audio::clock);
    let time = match &v.state {
        TrackState::Failed(e) => format!("{e} · reintentar"),
        TrackState::Loading(Some(f)) => format!("{:.0} %", f * 100.0),
        _ if v.active && v.position > std::time::Duration::ZERO => match &total {
            Some(t) => format!("{} / {t}", audio::clock(v.position)),
            None => audio::clock(v.position),
        },
        _ => total.clone().unwrap_or_else(|| "--:--".into()),
    };
    let time_c = if matches!(v.state, TrackState::Failed(_)) { k.error } else { k.meta };
    let mut info = row![t1(time, ts::META, time_c)].spacing(space::S2).align_y(Alignment::Center);
    if v.active && matches!(v.state, TrackState::Playing | TrackState::Paused) {
        info = info.push(speed_pill(app, k));
    }

    let seekable = v.active && matches!(v.state, TrackState::Playing | TrackState::Paused);
    let id = m.id;
    let seek = move |fraction: f32| Msg::Audio(AudioMsg::Seek { peer, msg: id, fraction });
    let body: Element<'a, Msg> = if md.kind == MediaKind::Voice {
        let wave = md.audio.as_ref().map(|a| a.waveform.as_slice()).unwrap_or_default();
        let n = ((inner_w + BAR_GAP) / (BAR_W + BAR_GAP)).floor().max(1.0) as usize;
        let bars = if wave.is_empty() { vec![3; n] } else { waveform::resample(wave, n) };
        let wave = canvas_widget(Wave { bars, progress: v.progress, played: k.played, rest: k.rest, seek: (!local).then_some(Box::new(seek) as Box<dyn Fn(f32) -> Msg>) })
            .width(Length::Fixed(inner_w))
            .height(Length::Fixed(WAVE_H));
        column![wave, info].spacing(space::S0_5).into()
    } else {
        let a = md.audio.as_ref();
        let title = match (a.and_then(|a| a.performer.as_deref()), a.and_then(|a| a.title.as_deref())) {
            (Some(p), Some(t)) => format!("{p} — {t}"),
            (None, Some(t)) => t.to_string(),
            _ => md.file_name.clone().unwrap_or_else(|| "Audio".into()),
        };
        let bar = canvas_widget(Bar { progress: v.progress, played: k.played, rest: k.rest, seek: Box::new(seek), seekable })
            .width(Length::Fixed(inner_w))
            .height(Length::Fixed(PROGRESS_H + 4.0));
        column![t1(crate::format::ellipsize(&title, 40), with_weight(ts::UI, 600), k.text), bar, info].spacing(1.0).into()
    };
    row![play, body]
        .spacing(space::S2)
        .align_y(Alignment::Center)
        .width(Length::Fixed(sz.width))
        .height(Length::Fixed(sz.height))
        .into()
}

fn play_button<'a>(v: &TrackView, k: Ink, on: Option<Msg>) -> Element<'a, Msg> {
    let content: Element<'a, Msg> = match &v.state {
        TrackState::Playing => icon_fill(Icon::Pause, 18.0, k.button_fg),
        TrackState::Loading(Some(f)) => t1(format!("{:.0}", f * 100.0), with_weight(ts::CAPTION, 700), k.button_fg).into(),
        TrackState::Loading(None) => icon(Icon::Download, 18.0, k.button_fg),
        TrackState::Failed(_) => icon_fill(Icon::Warning, 20.0, k.button_fg),
        TrackState::Idle | TrackState::Paused => icon_fill(Icon::Play, 18.0, k.button_fg),
    };
    let bg = k.button_bg;
    button(container(content).center(Length::Fill))
        .padding(0)
        .width(Length::Fixed(BUTTON))
        .height(Length::Fixed(BUTTON))
        .style(move |_: &Theme, status| button::Style {
            background: Some(if status == button::Status::Hovered { fade(bg, 0.88) } else { bg }.into()),
            text_color: Color::WHITE,
            border: Border { radius: style::r(radius::PILL), ..Default::default() },
            shadow: Shadow::default(),
            snap: true,
        })
        .on_press_maybe(on)
        .into()
}

fn speed_pill<'a>(app: &App, k: Ink) -> Element<'a, Msg> {
    let label = t1(audio::speed_label(app.audio.speed), with_weight(ts::CAPTION, 700), k.meta);
    let border = k.meta;
    button(label)
        .padding([0, 6])
        .style(move |_: &Theme, status| button::Style {
            background: (status == button::Status::Hovered).then(|| Color { a: 0.12, ..border }.into()),
            text_color: border,
            border: Border { radius: style::r(radius::SM), width: 1.0, color: Color { a: 0.5, ..border } },
            shadow: Shadow::default(),
            snap: true,
        })
        .on_press(Msg::Audio(AudioMsg::CycleSpeed))
        .into()
}

/// Fracción horizontal del cursor en `bounds` si está dentro.
fn fraction_at(bounds: Rectangle, cursor: mouse::Cursor) -> Option<f32> {
    cursor.position_in(bounds).map(|p| (p.x / bounds.width.max(1.0)).clamp(0.0, 1.0))
}

/// Forma de onda con el tramo reproducido en color y salto al pulsar.
struct Wave {
    bars: Vec<u8>,
    progress: f32,
    played: Color,
    rest: Color,
    seek: Option<Box<dyn Fn(f32) -> Msg>>,
}

impl canvas::Program<Msg> for Wave {
    type State = ();

    fn update(&self, _: &mut (), event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let seek = self.seek.as_ref()?;
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => fraction_at(bounds, cursor).map(|f| canvas::Action::publish(seek(f)).and_capture()),
            _ => None,
        }
    }

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let h = bounds.height;
        let n = self.bars.len().max(1) as f32;
        for (i, &v) in self.bars.iter().enumerate() {
            let bh = (f32::from(v) / f32::from(waveform::MAX) * h).max(BAR_W);
            let x = i as f32 * (BAR_W + BAR_GAP);
            let color = if (i as f32 + 0.5) / n <= self.progress { self.played } else { self.rest };
            let path = Path::rounded_rectangle(Point::new(x, h - bh), Size::new(BAR_W, bh), style::r(BAR_W / 2.0));
            frame.fill(&path, color);
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, _: &(), bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if self.seek.is_some() && cursor.is_over(bounds) { mouse::Interaction::Pointer } else { mouse::Interaction::default() }
    }
}

/// Barra de progreso fina de los audios con título.
struct Bar {
    progress: f32,
    played: Color,
    rest: Color,
    seek: Box<dyn Fn(f32) -> Msg>,
    seekable: bool,
}

impl canvas::Program<Msg> for Bar {
    type State = ();

    fn update(&self, _: &mut (), event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if self.seekable => {
                fraction_at(bounds, cursor).map(|f| canvas::Action::publish((self.seek)(f)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let y = (bounds.height - PROGRESS_H) / 2.0;
        let r = style::r(PROGRESS_H / 2.0);
        frame.fill(&Path::rounded_rectangle(Point::new(0.0, y), Size::new(bounds.width, PROGRESS_H), r), self.rest);
        let w = bounds.width * self.progress.clamp(0.0, 1.0);
        if w > 0.5 {
            frame.fill(&Path::rounded_rectangle(Point::new(0.0, y), Size::new(w.max(PROGRESS_H), PROGRESS_H), r), self.played);
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, _: &(), bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if self.seekable && cursor.is_over(bounds) { mouse::Interaction::Pointer } else { mouse::Interaction::default() }
    }
}
