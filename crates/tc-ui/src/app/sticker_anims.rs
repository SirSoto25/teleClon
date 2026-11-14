//! Stickers TGS animados en la app (ADR-0008): qué está a la vista, reloj de 30 fps, descargas y
//! render por lotes fuera del hilo de la UI. El estado y la política viven en `sticker_anim.rs`.

use super::*;
use crate::sticker_anim::{self, AnimMsg, Job};

/// TGS a la vista: documento y de dónde descargarlo.
struct ActiveTgs {
    peer: PeerRef,
    msg: MsgId,
    file: FileRef,
}

impl App {
    /// Política de reproducción actual (movimiento reducido de la app y foco de la ventana).
    pub fn sticker_playback(&self) -> tc_stickers::anim::Playback {
        self.sticker_anims.playback(self.anim.pref.is_reduced())
    }

    /// Sticker del selector que se anima: el que está bajo el ratón o, si no, el del cursor de teclado.
    pub fn picker_active_sticker(&self) -> Option<StickerId> {
        if !self.picker.open {
            return None;
        }
        let list = self.picker_tab_stickers();
        self.sticker_anims
            .hovered
            .filter(|id| list.iter().any(|s| s.id == *id))
            .or_else(|| self.picker.cursor.and_then(|i| list.get(i)).map(|s| s.id))
    }

    /// TGS animables a la vista: burbujas de la ventana visible y el sticker activo del selector.
    fn active_tgs(&self) -> Vec<ActiveTgs> {
        let mut out: Vec<ActiveTgs> = Vec::new();
        let mut push = |peer, msg, s: &Sticker| {
            if let Some(file) = s.file.as_ref().filter(|_| sticker_anim::plays(s))
                && !out.iter().any(|a| a.file.id == file.id)
            {
                out.push(ActiveTgs { peer, msg, file: file.clone() });
            }
        };
        if let Some(cv) = &self.conv
            && let Some((a, z)) = cv.visible.get()
        {
            for m in cv.messages.get(a..z.min(cv.messages.len())).unwrap_or_default() {
                let Some(s) = m.sticker.as_ref() else { continue };
                let (peer, msg) = if conv::is_local(m) || matches!(cv.peer, PeerRef::Secret(_)) { (self.sticker_peer(), MsgId(0)) } else { (cv.peer, m.id) };
                push(peer, msg, s);
            }
        }
        if let Some(id) = self.picker_active_sticker()
            && let Some(s) = self.picker_tab_stickers().into_iter().find(|s| s.id == id)
        {
            push(self.sticker_peer(), MsgId(0), s);
        }
        out
    }

    /// ¿Hay algún TGS animándose a la vista? (para el reloj de 30 fps).
    pub(crate) fn tgs_visible(&self) -> bool {
        tc_stickers::tgs::lottie_enabled() && !self.active_tgs().is_empty()
    }

    /// Fotograma animado de `s` para dibujarlo a `size` px, o `None` para la imagen estática. En el
    /// selector (tamaño pequeño) solo se anima el sticker activo.
    pub fn sticker_anim_frame(&self, s: &Sticker, size: f32) -> Option<iced::widget::image::Handle> {
        if !sticker_anim::plays(s) || (size <= crate::sticker_img::SMALL_MAX && self.picker_active_sticker() != Some(s.id)) {
            return None;
        }
        self.sticker_anims.frame(s.file.as_ref()?.id, self.sticker_playback()).cloned()
    }

    pub(crate) fn on_sticker_anim(&mut self, m: AnimMsg) -> Task<Msg> {
        let now = Instant::now();
        match m {
            AnimMsg::Tick(at) => {
                self.sticker_ms = at.saturating_duration_since(self.sticker_t0).as_secs_f32() * 1000.0;
                self.plan_sticker_anims(at)
            }
            AnimMsg::Loaded(file, tgs) => {
                self.sticker_anims.loaded(file, tgs, now);
                self.plan_sticker_anims(now)
            }
            AnimMsg::Frames(file, frames) => {
                self.sticker_anims.rendered(file, frames, now);
                Task::none()
            }
            AnimMsg::Focus(f) => {
                self.sticker_anims.focused = f;
                // Volver a la ventana con un chat abierto es leerlo (`app/notify.rs`).
                self.notify_focus(f);
                Task::none()
            }
            AnimMsg::Hover(id) => {
                self.sticker_anims.hovered = Some(id);
                self.plan_sticker_anims(now)
            }
            AnimMsg::Unhover(id) => {
                if self.sticker_anims.hovered == Some(id) {
                    self.sticker_anims.hovered = None;
                }
                Task::none()
            }
        }
    }

    /// Avanza las animaciones al instante `at` y lanza las descargas y lotes de render que falten.
    fn plan_sticker_anims(&mut self, at: Instant) -> Task<Msg> {
        if !tc_stickers::tgs::lottie_enabled() || self.phase != Phase::Main {
            return Task::none();
        }
        let active = self.active_tgs();
        let ids: Vec<i64> = active.iter().map(|a| a.file.id).collect();
        let elapsed = at.saturating_duration_since(self.sticker_t0);
        let playback = self.sticker_playback();
        let jobs = self.sticker_anims.plan(playback, elapsed, &ids, Instant::now());
        let mut tasks = Vec::new();
        for job in jobs {
            match job {
                Job::Load(id) => {
                    let Some(a) = active.iter().find(|a| a.file.id == id) else { continue };
                    let (peer, msg, file) = (a.peer, a.msg, a.file.clone());
                    tasks.push(self.call(
                        move |b| async move {
                            let bytes = b.download_media(peer, msg, file, MediaVariant::Full).await.ok()?;
                            sticker_anim::off_thread(move || tc_stickers::anim::Tgs::load(&bytes, &format!("tg{id}")).ok().map(Arc::new)).await.flatten()
                        },
                        move |tgs| Msg::StickerAnim(AnimMsg::Loaded(id, tgs)),
                    ));
                }
                Job::Render { file, tgs, frames } => {
                    tasks.push(Task::perform(
                        async move { sticker_anim::off_thread(move || sticker_anim::render_handles(&tgs, &frames)).await.unwrap_or_default() },
                        move |out| Msg::StickerAnim(AnimMsg::Frames(file, out)),
                    ));
                }
            }
        }
        Task::batch(tasks)
    }
}
