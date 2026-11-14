//! Reglas puras de notificación: cuándo avisar y qué enseñar.
//!
//! No conocen `tc-core`: la app traduce su modelo a [`ChatInfo`] e [`IncomingMessage`]. Dos piezas:
//!
//! * [`check`] y [`render`]: funciones sin estado (filtro y texto del aviso).
//! * [`Engine`]: añade el estado que hace falta para agrupar por chat (contador de no avisados),
//!   limitar ráfagas (p. ej. al reconectar con huecos) y no repetir el sonido en ráfaga.
//!
//! Todas las horas son segundos Unix (`now`), igual que `mute_until` de Telegram.

use std::collections::{HashMap, VecDeque};

use crate::{Action, ChatKey, Notification};

/// Nombre que se muestra cuando la vista previa oculta hasta el chat.
pub const APP_NAME: &str = "Parlo";
/// Id del botón «Marcar como leído» (vuelve en [`crate::NotifyEvent::Action`]).
pub const ACTION_MARK_READ: &str = "mark-read";
/// Longitud máxima del cuerpo en caracteres (los servidores cortan mal o nada).
pub const MAX_BODY_CHARS: usize = 200;

/// Tipo de chat a efectos de ajustes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatKind {
    /// Privado (también bots).
    Private,
    /// Grupo o supergrupo (también temas de foro).
    Group,
    Channel,
    /// Chat secreto: cuenta como privado para los ajustes por tipo, pero su vista previa se limita
    /// con [`NotifySettings::secret_preview`].
    Secret,
}

/// Lo que las reglas necesitan saber del chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatInfo {
    pub key: ChatKey,
    pub title: String,
    pub kind: ChatKind,
    /// Silenciado sin plazo (o con el plazo de `mute_until`).
    pub muted: bool,
    /// Fin del silencio (segundos Unix). Si está, manda sobre `muted`: pasado el plazo, el chat
    /// vuelve a sonar aunque `muted` siga a `true`.
    pub mute_until: Option<i64>,
    pub archived: bool,
}

impl ChatInfo {
    pub fn new(key: impl Into<ChatKey>, title: impl Into<String>, kind: ChatKind) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            kind,
            muted: false,
            mute_until: None,
            archived: false,
        }
    }

    /// ¿Está silenciado en `now`?
    pub fn is_muted(&self, now: i64) -> bool {
        match self.mute_until {
            Some(until) => until > now,
            None => self.muted,
        }
    }
}

/// Mensaje de servicio que puede filtrarse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceKind {
    /// «X se unió a Telegram».
    ContactJoined,
    /// Otro (fijado, alguien entró al grupo…): sigue las reglas normales.
    Other,
}

/// Un mensaje recién llegado.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IncomingMessage {
    /// Autor (en grupos se antepone al texto).
    pub sender: Option<String>,
    /// Texto ya resumido por la app (p. ej. «📷 Foto» para medios). Vacío = «Mensaje nuevo».
    pub text: String,
    /// Mío (enviado desde otro dispositivo o eco del envío).
    pub outgoing: bool,
    pub service: Option<ServiceKind>,
    /// Me menciona o responde a un mensaje mío.
    pub mentions_me: bool,
    /// El remitente lo mandó sin sonido.
    pub silent: bool,
}

/// Estado de la ventana.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowState {
    pub focused: bool,
    /// Chat (o tema) abierto en el panel de mensajes.
    pub open_chat: Option<ChatKey>,
}

/// Cuánto enseñar del mensaje. Ordenado de menos a más.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Preview {
    /// Ni chat ni texto: «Parlo · Mensaje nuevo».
    Hidden,
    /// Nombre del chat, sin texto.
    NameOnly,
    /// Nombre y texto.
    Full,
}

/// «No molestar».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DoNotDisturb {
    #[default]
    Off,
    /// Hasta que se quite.
    On,
    /// Hasta ese instante (segundos Unix).
    Until(i64),
}

impl DoNotDisturb {
    pub fn is_active(self, now: i64) -> bool {
        match self {
            Self::Off => false,
            Self::On => true,
            Self::Until(t) => t > now,
        }
    }
}

/// Límite de ráfagas: como mucho `max` avisos en `window_secs` segundos (todos los chats). El
/// sonido suena como mucho una vez por ventana.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurstLimit {
    pub window_secs: i64,
    pub max: u32,
}

impl Default for BurstLimit {
    fn default() -> Self {
        Self {
            window_secs: 10,
            max: 5,
        }
    }
}

/// Ajustes de notificaciones (S28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifySettings {
    /// Interruptor general.
    pub enabled: bool,
    /// Privados y secretos.
    pub private: bool,
    pub groups: bool,
    pub channels: bool,
    /// Avisar de chats archivados no silenciados (los silenciados nunca avisan).
    pub archived: bool,
    /// Las menciones y respuestas avisan aunque el chat esté silenciado.
    pub mentions_bypass_mute: bool,
    /// Ignorar «X se unió a Telegram».
    pub ignore_contact_joined: bool,
    pub preview: Preview,
    /// Tope de vista previa en chats secretos (por defecto solo el nombre: nunca texto).
    pub secret_preview: Preview,
    pub sound: bool,
    pub dnd: DoNotDisturb,
    pub burst: BurstLimit,
}

impl Default for NotifySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            private: true,
            groups: true,
            channels: true,
            archived: true,
            mentions_bypass_mute: true,
            ignore_contact_joined: true,
            preview: Preview::Full,
            secret_preview: Preview::NameOnly,
            sound: true,
            dnd: DoNotDisturb::Off,
            burst: BurstLimit::default(),
        }
    }
}

/// Por qué no se avisa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suppress {
    /// Mensaje propio.
    Outgoing,
    /// Notificaciones desactivadas.
    Disabled,
    DoNotDisturb,
    /// «Se unió a Telegram».
    ContactJoined,
    /// La ventana tiene foco y ese chat está abierto.
    ChatOpen,
    /// Desactivadas para ese tipo de chat.
    KindDisabled(ChatKind),
    /// Chat archivado y los archivados no avisan.
    Archived,
    Muted,
    /// Límite de ráfaga superado.
    RateLimited,
}

/// Filtro sin estado: `Ok(())` si este mensaje merece aviso.
pub fn check(
    msg: &IncomingMessage,
    chat: &ChatInfo,
    window: &WindowState,
    settings: &NotifySettings,
    now: i64,
) -> Result<(), Suppress> {
    if msg.outgoing {
        return Err(Suppress::Outgoing);
    }
    if !settings.enabled {
        return Err(Suppress::Disabled);
    }
    if settings.dnd.is_active(now) {
        return Err(Suppress::DoNotDisturb);
    }
    if settings.ignore_contact_joined && msg.service == Some(ServiceKind::ContactJoined) {
        return Err(Suppress::ContactJoined);
    }
    if window.focused && window.open_chat.as_ref() == Some(&chat.key) {
        return Err(Suppress::ChatOpen);
    }
    let kind_on = match chat.kind {
        ChatKind::Private | ChatKind::Secret => settings.private,
        ChatKind::Group => settings.groups,
        ChatKind::Channel => settings.channels,
    };
    if !kind_on {
        return Err(Suppress::KindDisabled(chat.kind));
    }
    let muted = chat.is_muted(now);
    if chat.archived && !muted && !settings.archived {
        return Err(Suppress::Archived);
    }
    if muted && !(msg.mentions_me && settings.mentions_bypass_mute) {
        return Err(Suppress::Muted);
    }
    Ok(())
}

/// Vista previa efectiva para ese chat.
pub fn effective_preview(chat: &ChatInfo, settings: &NotifySettings) -> Preview {
    match chat.kind {
        ChatKind::Secret => settings.preview.min(settings.secret_preview),
        _ => settings.preview,
    }
}

/// Título y cuerpo de un aviso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub title: String,
    pub body: String,
}

/// Texto del aviso. `count` = mensajes sin avisar de ese chat, incluido este (agrupar por chat).
pub fn render(
    msg: &IncomingMessage,
    chat: &ChatInfo,
    settings: &NotifySettings,
    count: u32,
) -> Rendered {
    let count = count.max(1);
    let generic = if count > 1 {
        format!("{count} mensajes nuevos")
    } else {
        "Mensaje nuevo".to_owned()
    };
    match effective_preview(chat, settings) {
        Preview::Hidden => Rendered {
            title: APP_NAME.to_owned(),
            body: generic,
        },
        Preview::NameOnly => Rendered {
            title: chat.title.clone(),
            body: generic,
        },
        Preview::Full => {
            let title = if count > 1 {
                format!("{} ({count})", chat.title)
            } else {
                chat.title.clone()
            };
            let text = msg.text.trim();
            let text = if text.is_empty() {
                "Mensaje nuevo"
            } else {
                text
            };
            let body = match (&msg.sender, chat.kind) {
                (Some(s), ChatKind::Group) if !s.is_empty() => format!("{s}: {text}"),
                _ => text.to_owned(),
            };
            Rendered {
                title,
                body: truncate(&body, MAX_BODY_CHARS),
            }
        }
    }
}

/// Corta a `max` caracteres (no bytes) con «…».
pub fn truncate(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        None => s.to_owned(),
        Some(_) => {
            let cut: String = s.chars().take(max.saturating_sub(1)).collect();
            format!("{}…", cut.trim_end())
        }
    }
}

/// Resultado de [`Engine::on_message`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Mostrar (sustituye al aviso anterior de ese chat). `icon` va vacío: lo pone la app.
    Show(Notification),
    Suppressed(Suppress),
}

/// Reglas con estado: agrupar por chat, limitar ráfagas y no repetir el sonido en ráfaga.
#[derive(Debug, Clone, Default)]
pub struct Engine {
    settings: NotifySettings,
    /// Instantes de los últimos avisos mostrados (para la ráfaga).
    shown: VecDeque<i64>,
    /// Mensajes que han pasado el filtro desde la última lectura, por chat.
    pending: HashMap<ChatKey, u32>,
    last_sound: Option<i64>,
}

impl Engine {
    pub fn new(settings: NotifySettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    pub fn settings(&self) -> &NotifySettings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: NotifySettings) {
        self.settings = settings;
    }

    /// Mensajes pendientes (sin leer y avisables) de `chat`.
    pub fn pending(&self, chat: &ChatKey) -> u32 {
        self.pending.get(chat).copied().unwrap_or(0)
    }

    /// Decide sobre un mensaje nuevo.
    pub fn on_message(
        &mut self,
        msg: &IncomingMessage,
        chat: &ChatInfo,
        window: &WindowState,
        now: i64,
    ) -> Outcome {
        if let Err(why) = check(msg, chat, window, &self.settings, now) {
            if why == Suppress::ChatOpen {
                // Lo está viendo: el grupo de ese chat empieza de cero.
                self.pending.remove(&chat.key);
            }
            return Outcome::Suppressed(why);
        }
        let count = {
            let c = self.pending.entry(chat.key.clone()).or_insert(0);
            *c = c.saturating_add(1);
            *c
        };
        let BurstLimit { window_secs, max } = self.settings.burst;
        while self.shown.front().is_some_and(|&t| t <= now - window_secs) {
            self.shown.pop_front();
        }
        if self.shown.len() >= max as usize {
            // Se cuenta igualmente: el próximo aviso de ese chat dirá «N mensajes».
            return Outcome::Suppressed(Suppress::RateLimited);
        }
        self.shown.push_back(now);

        let sound_ok = self.settings.sound
            && !msg.silent
            && self.last_sound.is_none_or(|t| t <= now - window_secs);
        if sound_ok {
            self.last_sound = Some(now);
        }
        let Rendered { title, body } = render(msg, chat, &self.settings, count);
        Outcome::Show(Notification {
            chat_key: chat.key.clone(),
            title,
            body,
            icon: None,
            actions: vec![Action::new(ACTION_MARK_READ, "Marcar como leído")],
            silent: !sound_ok,
        })
    }

    /// El usuario leyó `chat` (lo abrió con foco, o «Marcar como leído»). Devuelve `true` si había
    /// algo avisado: la app debería llamar a [`crate::Notifier::withdraw`].
    pub fn on_chat_read(&mut self, chat: &ChatKey) -> bool {
        self.pending.remove(chat).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn chat(kind: ChatKind) -> ChatInfo {
        let (key, title) = match kind {
            ChatKind::Private => ("u:1", "Ana"),
            ChatKind::Group => ("g:2", "Familia"),
            ChatKind::Channel => ("c:3", "Noticias"),
            ChatKind::Secret => ("s:4", "Ana 🔒"),
        };
        ChatInfo::new(key, title, kind)
    }

    fn msg(text: &str) -> IncomingMessage {
        IncomingMessage {
            sender: Some("Luis".into()),
            text: text.into(),
            ..Default::default()
        }
    }

    fn away() -> WindowState {
        WindowState {
            focused: false,
            open_chat: None,
        }
    }

    fn ok(
        m: &IncomingMessage,
        c: &ChatInfo,
        w: &WindowState,
        s: &NotifySettings,
    ) -> Result<(), Suppress> {
        check(m, c, w, s, NOW)
    }

    // --- check: foco y chat abierto ---

    #[test]
    fn avisa_sin_foco_aunque_el_chat_este_abierto() {
        let c = chat(ChatKind::Private);
        let w = WindowState {
            focused: false,
            open_chat: Some(c.key.clone()),
        };
        assert_eq!(ok(&msg("hola"), &c, &w, &NotifySettings::default()), Ok(()));
    }

    #[test]
    fn avisa_con_foco_si_el_chat_no_es_el_abierto() {
        let c = chat(ChatKind::Private);
        let w = WindowState {
            focused: true,
            open_chat: Some("u:999".into()),
        };
        assert_eq!(ok(&msg("hola"), &c, &w, &NotifySettings::default()), Ok(()));
        let w = WindowState {
            focused: true,
            open_chat: None,
        };
        assert_eq!(ok(&msg("hola"), &c, &w, &NotifySettings::default()), Ok(()));
    }

    #[test]
    fn no_avisa_con_foco_y_chat_abierto() {
        let c = chat(ChatKind::Group);
        let w = WindowState {
            focused: true,
            open_chat: Some(c.key.clone()),
        };
        assert_eq!(
            ok(&msg("hola"), &c, &w, &NotifySettings::default()),
            Err(Suppress::ChatOpen)
        );
    }

    // --- check: propio, general, No molestar, servicio ---

    #[test]
    fn nunca_avisa_mensajes_propios() {
        let mut m = msg("yo");
        m.outgoing = true;
        m.mentions_me = true;
        for k in [
            ChatKind::Private,
            ChatKind::Group,
            ChatKind::Channel,
            ChatKind::Secret,
        ] {
            assert_eq!(
                ok(&m, &chat(k), &away(), &NotifySettings::default()),
                Err(Suppress::Outgoing)
            );
        }
    }

    #[test]
    fn interruptor_general() {
        let s = NotifySettings {
            enabled: false,
            ..Default::default()
        };
        assert_eq!(
            ok(&msg("x"), &chat(ChatKind::Private), &away(), &s),
            Err(Suppress::Disabled)
        );
    }

    #[test]
    fn no_molestar_indefinido_y_con_plazo() {
        let c = chat(ChatKind::Private);
        let mut s = NotifySettings {
            dnd: DoNotDisturb::On,
            ..Default::default()
        };
        assert_eq!(ok(&msg("x"), &c, &away(), &s), Err(Suppress::DoNotDisturb));
        s.dnd = DoNotDisturb::Until(NOW + 60);
        assert_eq!(ok(&msg("x"), &c, &away(), &s), Err(Suppress::DoNotDisturb));
        s.dnd = DoNotDisturb::Until(NOW);
        assert_eq!(ok(&msg("x"), &c, &away(), &s), Ok(()), "el plazo ya venció");
        s.dnd = DoNotDisturb::Off;
        assert_eq!(ok(&msg("x"), &c, &away(), &s), Ok(()));
    }

    #[test]
    fn no_molestar_gana_a_las_menciones() {
        let mut m = msg("@yo");
        m.mentions_me = true;
        let s = NotifySettings {
            dnd: DoNotDisturb::On,
            ..Default::default()
        };
        assert_eq!(
            ok(&m, &chat(ChatKind::Group), &away(), &s),
            Err(Suppress::DoNotDisturb)
        );
    }

    #[test]
    fn ignora_se_unio_a_telegram_salvo_que_se_pida() {
        let c = chat(ChatKind::Private);
        let m = IncomingMessage {
            service: Some(ServiceKind::ContactJoined),
            text: "Ana se unió".into(),
            ..Default::default()
        };
        assert_eq!(
            ok(&m, &c, &away(), &NotifySettings::default()),
            Err(Suppress::ContactJoined)
        );
        let s = NotifySettings {
            ignore_contact_joined: false,
            ..Default::default()
        };
        assert_eq!(ok(&m, &c, &away(), &s), Ok(()));
    }

    #[test]
    fn otros_servicios_siguen_las_reglas_normales() {
        let m = IncomingMessage {
            service: Some(ServiceKind::Other),
            text: "fijó un mensaje".into(),
            ..Default::default()
        };
        assert_eq!(
            ok(
                &m,
                &chat(ChatKind::Group),
                &away(),
                &NotifySettings::default()
            ),
            Ok(())
        );
    }

    // --- check: tipo de chat ---

    #[test]
    fn ajustes_por_tipo() {
        let base = NotifySettings::default();
        let cases = [
            (
                NotifySettings {
                    private: false,
                    ..base.clone()
                },
                ChatKind::Private,
                true,
            ),
            (
                NotifySettings {
                    private: false,
                    ..base.clone()
                },
                ChatKind::Secret,
                true,
            ),
            (
                NotifySettings {
                    private: false,
                    ..base.clone()
                },
                ChatKind::Group,
                false,
            ),
            (
                NotifySettings {
                    groups: false,
                    ..base.clone()
                },
                ChatKind::Group,
                true,
            ),
            (
                NotifySettings {
                    groups: false,
                    ..base.clone()
                },
                ChatKind::Channel,
                false,
            ),
            (
                NotifySettings {
                    channels: false,
                    ..base.clone()
                },
                ChatKind::Channel,
                true,
            ),
            (
                NotifySettings {
                    channels: false,
                    ..base.clone()
                },
                ChatKind::Private,
                false,
            ),
        ];
        for (s, kind, blocked) in cases {
            let r = ok(&msg("x"), &chat(kind), &away(), &s);
            if blocked {
                assert_eq!(r, Err(Suppress::KindDisabled(kind)), "{kind:?}");
            } else {
                assert_eq!(r, Ok(()), "{kind:?}");
            }
        }
    }

    #[test]
    fn tipo_desactivado_gana_a_las_menciones() {
        let mut m = msg("@yo");
        m.mentions_me = true;
        let s = NotifySettings {
            groups: false,
            ..Default::default()
        };
        assert_eq!(
            ok(&m, &chat(ChatKind::Group), &away(), &s),
            Err(Suppress::KindDisabled(ChatKind::Group))
        );
    }

    // --- check: silenciado ---

    #[test]
    fn silenciado_sin_plazo() {
        let mut c = chat(ChatKind::Group);
        c.muted = true;
        assert_eq!(
            ok(&msg("x"), &c, &away(), &NotifySettings::default()),
            Err(Suppress::Muted)
        );
    }

    #[test]
    fn silenciado_con_plazo_vigente_y_vencido() {
        let mut c = chat(ChatKind::Private);
        c.mute_until = Some(NOW + 3600);
        assert!(c.is_muted(NOW));
        assert_eq!(
            ok(&msg("x"), &c, &away(), &NotifySettings::default()),
            Err(Suppress::Muted)
        );
        // Plazo vencido: vuelve a sonar aunque `muted` siga a true (dato desfasado).
        c.mute_until = Some(NOW - 1);
        c.muted = true;
        assert!(!c.is_muted(NOW));
        assert_eq!(
            ok(&msg("x"), &c, &away(), &NotifySettings::default()),
            Ok(())
        );
        // Justo en el instante del fin ya no está silenciado.
        c.mute_until = Some(NOW);
        assert!(!c.is_muted(NOW));
        // Plazo futuro con muted=false: manda el plazo.
        c.muted = false;
        c.mute_until = Some(NOW + 1);
        assert!(c.is_muted(NOW));
    }

    #[test]
    fn mencion_atraviesa_el_silencio_si_el_ajuste_lo_permite() {
        let mut c = chat(ChatKind::Group);
        c.muted = true;
        let mut m = msg("@yo mira");
        m.mentions_me = true;
        assert_eq!(ok(&m, &c, &away(), &NotifySettings::default()), Ok(()));
        let s = NotifySettings {
            mentions_bypass_mute: false,
            ..Default::default()
        };
        assert_eq!(ok(&m, &c, &away(), &s), Err(Suppress::Muted));
    }

    // --- check: archivados ---

    #[test]
    fn archivado_no_silenciado_depende_del_ajuste() {
        let mut c = chat(ChatKind::Private);
        c.archived = true;
        assert_eq!(
            ok(&msg("x"), &c, &away(), &NotifySettings::default()),
            Ok(())
        );
        let s = NotifySettings {
            archived: false,
            ..Default::default()
        };
        assert_eq!(ok(&msg("x"), &c, &away(), &s), Err(Suppress::Archived));
    }

    #[test]
    fn archivado_y_silenciado_nunca_avisa() {
        let mut c = chat(ChatKind::Group);
        c.archived = true;
        c.muted = true;
        assert_eq!(
            ok(&msg("x"), &c, &away(), &NotifySettings::default()),
            Err(Suppress::Muted)
        );
    }

    // --- render: vista previa ---

    #[test]
    fn vista_previa_completa_privado_grupo_canal() {
        let s = NotifySettings::default();
        let r = render(&msg("hola"), &chat(ChatKind::Private), &s, 1);
        assert_eq!(
            r,
            Rendered {
                title: "Ana".into(),
                body: "hola".into()
            }
        );
        let r = render(&msg("hola"), &chat(ChatKind::Group), &s, 1);
        assert_eq!(
            r,
            Rendered {
                title: "Familia".into(),
                body: "Luis: hola".into()
            }
        );
        let r = render(&msg("titular"), &chat(ChatKind::Channel), &s, 1);
        assert_eq!(r.body, "titular", "en canales no se antepone autor");
        let mut sin_autor = msg("hola");
        sin_autor.sender = None;
        assert_eq!(
            render(&sin_autor, &chat(ChatKind::Group), &s, 1).body,
            "hola"
        );
        sin_autor.sender = Some(String::new());
        assert_eq!(
            render(&sin_autor, &chat(ChatKind::Group), &s, 1).body,
            "hola"
        );
    }

    #[test]
    fn vista_previa_solo_nombre_y_oculta() {
        let s = NotifySettings {
            preview: Preview::NameOnly,
            ..Default::default()
        };
        let r = render(&msg("secreto bancario"), &chat(ChatKind::Group), &s, 1);
        assert_eq!(
            r,
            Rendered {
                title: "Familia".into(),
                body: "Mensaje nuevo".into()
            }
        );
        let s = NotifySettings {
            preview: Preview::Hidden,
            ..Default::default()
        };
        let r = render(&msg("secreto bancario"), &chat(ChatKind::Group), &s, 3);
        assert_eq!(
            r,
            Rendered {
                title: APP_NAME.into(),
                body: "3 mensajes nuevos".into()
            }
        );
    }

    #[test]
    fn chats_secretos_nunca_muestran_texto_por_defecto() {
        let s = NotifySettings::default();
        assert_eq!(s.preview, Preview::Full);
        let c = chat(ChatKind::Secret);
        let r = render(&msg("clave 1234"), &c, &s, 1);
        assert!(!r.body.contains("1234") && !r.title.contains("1234"));
        assert_eq!(
            r,
            Rendered {
                title: "Ana 🔒".into(),
                body: "Mensaje nuevo".into()
            }
        );
        // El tope de secretos solo baja, nunca sube por encima del general.
        let s = NotifySettings {
            preview: Preview::Hidden,
            secret_preview: Preview::Full,
            ..Default::default()
        };
        assert_eq!(effective_preview(&c, &s), Preview::Hidden);
        // Se puede pedir texto en secretos explícitamente.
        let s = NotifySettings {
            secret_preview: Preview::Full,
            ..Default::default()
        };
        assert_eq!(render(&msg("clave 1234"), &c, &s, 1).body, "clave 1234");
        // El tope de secretos no afecta a otros tipos.
        let s = NotifySettings {
            secret_preview: Preview::Hidden,
            ..Default::default()
        };
        assert_eq!(
            effective_preview(&chat(ChatKind::Private), &s),
            Preview::Full
        );
    }

    #[test]
    fn cuenta_agrupada_en_el_titulo_o_el_cuerpo() {
        let s = NotifySettings::default();
        let r = render(&msg("tercero"), &chat(ChatKind::Private), &s, 3);
        assert_eq!(
            r,
            Rendered {
                title: "Ana (3)".into(),
                body: "tercero".into()
            }
        );
        let s = NotifySettings {
            preview: Preview::NameOnly,
            ..Default::default()
        };
        assert_eq!(
            render(&msg("x"), &chat(ChatKind::Private), &s, 2).body,
            "2 mensajes nuevos"
        );
        // count 0 se trata como 1.
        assert_eq!(
            render(&msg("x"), &chat(ChatKind::Private), &s, 0).body,
            "Mensaje nuevo"
        );
    }

    #[test]
    fn texto_vacio_y_largo() {
        let s = NotifySettings::default();
        let r = render(&msg("   "), &chat(ChatKind::Private), &s, 1);
        assert_eq!(r.body, "Mensaje nuevo");
        let largo = "á".repeat(500);
        let r = render(&msg(&largo), &chat(ChatKind::Private), &s, 1);
        assert_eq!(r.body.chars().count(), MAX_BODY_CHARS);
        assert!(r.body.ends_with('…'));
    }

    #[test]
    fn truncar_por_caracteres() {
        assert_eq!(truncate("hola", 4), "hola");
        assert_eq!(truncate("hola!", 4), "hol…");
        assert_eq!(
            truncate("ab cdef", 4),
            "ab…",
            "sin espacio antes de los puntos"
        );
        assert_eq!(truncate("🙂🙂🙂", 2), "🙂…");
        assert_eq!(truncate("", 3), "");
    }

    // --- Engine: agrupar, ráfagas, sonido ---

    fn engine() -> Engine {
        Engine::new(NotifySettings::default())
    }

    fn shown(o: Outcome) -> Notification {
        match o {
            Outcome::Show(n) => n,
            Outcome::Suppressed(why) => panic!("suprimido: {why:?}"),
        }
    }

    #[test]
    fn engine_produce_aviso_con_accion_y_clave() {
        let mut e = engine();
        let c = chat(ChatKind::Private);
        let n = shown(e.on_message(&msg("hola"), &c, &away(), NOW));
        assert_eq!(n.chat_key, c.key);
        assert_eq!(n.title, "Ana");
        assert_eq!(n.body, "hola");
        assert_eq!(n.icon, None);
        assert_eq!(
            n.actions,
            vec![Action::new(ACTION_MARK_READ, "Marcar como leído")]
        );
        assert!(!n.silent);
    }

    #[test]
    fn engine_agrupa_por_chat_y_reinicia_al_leer() {
        let mut e = engine();
        let a = chat(ChatKind::Private);
        let g = chat(ChatKind::Group);
        shown(e.on_message(&msg("1"), &a, &away(), NOW));
        shown(e.on_message(&msg("x"), &g, &away(), NOW + 20));
        let n = shown(e.on_message(&msg("2"), &a, &away(), NOW + 40));
        assert_eq!(n.title, "Ana (2)");
        assert_eq!(e.pending(&a.key), 2);
        assert_eq!(e.pending(&g.key), 1);
        assert!(e.on_chat_read(&a.key));
        assert!(!e.on_chat_read(&a.key), "segunda lectura: nada que retirar");
        assert_eq!(e.pending(&a.key), 0);
        let n = shown(e.on_message(&msg("3"), &a, &away(), NOW + 60));
        assert_eq!(n.title, "Ana");
    }

    #[test]
    fn engine_abrir_el_chat_con_foco_reinicia_el_grupo() {
        let mut e = engine();
        let a = chat(ChatKind::Private);
        shown(e.on_message(&msg("1"), &a, &away(), NOW));
        let viendo = WindowState {
            focused: true,
            open_chat: Some(a.key.clone()),
        };
        assert_eq!(
            e.on_message(&msg("2"), &a, &viendo, NOW + 20),
            Outcome::Suppressed(Suppress::ChatOpen)
        );
        assert_eq!(e.pending(&a.key), 0);
        assert_eq!(
            shown(e.on_message(&msg("3"), &a, &away(), NOW + 40)).title,
            "Ana"
        );
    }

    #[test]
    fn engine_suprimidos_no_cuentan_para_el_grupo() {
        let mut e = engine();
        let mut c = chat(ChatKind::Group);
        c.muted = true;
        assert_eq!(
            e.on_message(&msg("1"), &c, &away(), NOW),
            Outcome::Suppressed(Suppress::Muted)
        );
        assert_eq!(e.pending(&c.key), 0);
        assert!(!e.on_chat_read(&c.key));
    }

    #[test]
    fn engine_limita_rafagas_al_reconectar() {
        let mut e = Engine::new(NotifySettings {
            burst: BurstLimit {
                window_secs: 10,
                max: 3,
            },
            ..Default::default()
        });
        // 20 mensajes de 20 chats en el mismo segundo (reconexión con hueco).
        let mut shown_n = 0;
        let mut limited = 0;
        for i in 0..20 {
            let c = ChatInfo::new(format!("x:{i}"), format!("C{i}"), ChatKind::Private);
            match e.on_message(&msg("x"), &c, &away(), NOW) {
                Outcome::Show(_) => shown_n += 1,
                Outcome::Suppressed(Suppress::RateLimited) => limited += 1,
                other => panic!("{other:?}"),
            }
        }
        assert_eq!((shown_n, limited), (3, 17));
        // Dentro de la ventana sigue limitado; justo al cumplirse se libera.
        let c = chat(ChatKind::Private);
        assert_eq!(
            e.on_message(&msg("y"), &c, &away(), NOW + 9),
            Outcome::Suppressed(Suppress::RateLimited)
        );
        let n = shown(e.on_message(&msg("z"), &c, &away(), NOW + 10));
        assert_eq!(
            n.title, "Ana (2)",
            "el mensaje limitado cuenta para el grupo"
        );
    }

    #[test]
    fn engine_ventana_deslizante() {
        let mut e = Engine::new(NotifySettings {
            burst: BurstLimit {
                window_secs: 10,
                max: 2,
            },
            ..Default::default()
        });
        let c = chat(ChatKind::Private);
        shown(e.on_message(&msg("a"), &c, &away(), NOW));
        shown(e.on_message(&msg("b"), &c, &away(), NOW + 5));
        assert!(matches!(
            e.on_message(&msg("c"), &c, &away(), NOW + 9),
            Outcome::Suppressed(Suppress::RateLimited)
        ));
        // A los 10 s sale el primero de la ventana, pero el de NOW+5 sigue.
        shown(e.on_message(&msg("d"), &c, &away(), NOW + 10));
        assert!(matches!(
            e.on_message(&msg("e"), &c, &away(), NOW + 12),
            Outcome::Suppressed(Suppress::RateLimited)
        ));
        shown(e.on_message(&msg("f"), &c, &away(), NOW + 15));
    }

    #[test]
    fn engine_un_solo_sonido_por_ventana() {
        let mut e = engine();
        let a = chat(ChatKind::Private);
        let g = chat(ChatKind::Group);
        assert!(!shown(e.on_message(&msg("1"), &a, &away(), NOW)).silent);
        assert!(shown(e.on_message(&msg("2"), &g, &away(), NOW + 3)).silent);
        assert!(shown(e.on_message(&msg("3"), &a, &away(), NOW + 9)).silent);
        assert!(!shown(e.on_message(&msg("4"), &a, &away(), NOW + 10)).silent);
    }

    #[test]
    fn engine_sin_sonido_por_ajuste_o_por_remitente() {
        let mut e = Engine::new(NotifySettings {
            sound: false,
            ..Default::default()
        });
        assert!(shown(e.on_message(&msg("1"), &chat(ChatKind::Private), &away(), NOW)).silent);
        let mut e = engine();
        let mut m = msg("shh");
        m.silent = true;
        assert!(shown(e.on_message(&m, &chat(ChatKind::Private), &away(), NOW)).silent);
        // Un aviso silencioso no gasta el sonido de la ventana.
        assert!(!shown(e.on_message(&msg("2"), &chat(ChatKind::Group), &away(), NOW + 1)).silent);
    }

    #[test]
    fn engine_cambiar_ajustes_en_caliente() {
        let mut e = engine();
        let c = chat(ChatKind::Private);
        shown(e.on_message(&msg("1"), &c, &away(), NOW));
        e.set_settings(NotifySettings {
            dnd: DoNotDisturb::On,
            ..e.settings().clone()
        });
        assert_eq!(
            e.on_message(&msg("2"), &c, &away(), NOW + 20),
            Outcome::Suppressed(Suppress::DoNotDisturb)
        );
        assert_eq!(e.settings().dnd, DoNotDisturb::On);
    }

    #[test]
    fn engine_secreto_agrupado_sin_texto() {
        let mut e = engine();
        let c = chat(ChatKind::Secret);
        shown(e.on_message(&msg("pin 1"), &c, &away(), NOW));
        let n = shown(e.on_message(&msg("pin 2"), &c, &away(), NOW + 20));
        assert_eq!(
            (n.title.as_str(), n.body.as_str()),
            ("Ana 🔒", "2 mensajes nuevos")
        );
    }

    #[test]
    fn preview_ordenado() {
        assert!(Preview::Hidden < Preview::NameOnly && Preview::NameOnly < Preview::Full);
    }
}
