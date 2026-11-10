//! Notas de voz y audios en la conversación (ADR-0007): descarga perezosa al pulsar reproducir,
//! un único audio sonando a la vez, pausa, salto pulsando en la forma de onda, velocidad 1x/1.5x/2x
//! y error con reintento. El sonido lo pone `tc_audio::Player` en su propio hilo: aquí solo se
//! mandan órdenes y se lee su estado (nunca se bloquea la UI).
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use iced::{Size, Task};
use tc_audio::{AudioError, PlaybackState, Player, SPEEDS};
use tc_core::*;

use crate::app::{App, Msg, error_text};

/// Alto de la burbuja de audio (botón de 40 px y dos líneas).
pub const AUDIO_H: f32 = 44.0;
/// Ancho de una nota de voz (la forma de onda se ajusta a él).
pub const VOICE_W: f32 = 260.0;
/// Ancho de un audio con título.
pub const MUSIC_W: f32 = 280.0;
/// Lado de un vídeo redondo.
pub const ROUND_SIDE: f32 = 200.0;
/// Bytes de audio que se guardan en memoria (los más recientes) para no volver a descargarlos.
const BYTES_CACHE: usize = 48 * 1024 * 1024;

/// Bytes de un audio. `Debug` no los vuelca.
#[derive(Clone)]
pub struct AudioBytes(pub Arc<[u8]>);

impl std::fmt::Debug for AudioBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AudioBytes({} B)", self.0.len())
    }
}

#[derive(Debug, Clone)]
pub enum AudioMsg {
    /// Reproducir / pausar / reintentar el audio de un mensaje.
    Toggle { peer: PeerRef, msg: MsgId },
    /// Terminó la descarga del archivo `file`.
    Loaded { file: i64, result: Result<AudioBytes, String> },
    /// Pulsación en la forma de onda o la barra (0..=1).
    Seek { peer: PeerRef, msg: MsgId, fraction: f32 },
    /// 1x → 1.5x → 2x → 1x.
    CycleSpeed,
    /// Reloj mientras suena (redibuja el progreso).
    Tick,
}

/// Lo que dibuja una burbuja de audio.
#[derive(Debug, Clone, PartialEq)]
pub enum TrackState {
    Idle,
    /// Descargando; fracción si se conoce.
    Loading(Option<f32>),
    Playing,
    Paused,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackView {
    pub state: TrackState,
    /// Es la pista activa (se muestra la velocidad y la posición).
    pub active: bool,
    pub position: Duration,
    /// 0..=1.
    pub progress: f32,
}

#[derive(Debug, Clone, PartialEq)]
enum Fetch {
    Loading,
    Failed(String),
}

/// Estado del audio de la app.
pub struct AudioState {
    pub player: Player,
    pub speed: f32,
    /// Pista activa: chat, mensaje y archivo.
    pub current: Option<(PeerRef, MsgId, i64)>,
    /// Archivo que se reproducirá en cuanto acabe de descargarse.
    pending: Option<i64>,
    fetch: HashMap<i64, Fetch>,
    bytes: HashMap<i64, Arc<[u8]>>,
    order: VecDeque<i64>,
    cached: usize,
}

impl Default for AudioState {
    fn default() -> Self {
        Self::with_player(Player::new())
    }
}

impl AudioState {
    /// Con otro reproductor (las pruebas usan uno mudo).
    pub fn with_player(player: Player) -> Self {
        Self { player, speed: 1.0, current: None, pending: None, fetch: HashMap::new(), bytes: HashMap::new(), order: VecDeque::new(), cached: 0 }
    }

    /// ¿Hay que redibujar periódicamente (suena o arranca)?
    pub fn needs_tick(&self) -> bool {
        self.player.status().is_some_and(|s| matches!(s.state, PlaybackState::Starting | PlaybackState::Playing))
    }

    pub fn is_cached(&self, file: i64) -> bool {
        self.bytes.contains_key(&file)
    }

    fn remember(&mut self, file: i64, bytes: Arc<[u8]>) {
        if self.bytes.contains_key(&file) || bytes.len() > BYTES_CACHE {
            return;
        }
        self.cached += bytes.len();
        self.bytes.insert(file, bytes);
        self.order.push_back(file);
        while self.cached > BYTES_CACHE {
            let Some(old) = self.order.pop_front() else { break };
            if let Some(b) = self.bytes.remove(&old) {
                self.cached -= b.len();
            }
        }
    }

    /// Estado para dibujar el audio `file` (`downloads`: progreso de las descargas largas).
    pub fn view(&self, file: i64, duration: Option<Duration>, downloads: &HashMap<i64, (u64, u64)>) -> TrackView {
        let idle = TrackView { state: TrackState::Idle, active: false, position: Duration::ZERO, progress: 0.0 };
        let active = self.current.is_some_and(|c| c.2 == file);
        if let Some(Fetch::Failed(e)) = self.fetch.get(&file) {
            return TrackView { state: TrackState::Failed(e.clone()), ..idle };
        }
        if matches!(self.fetch.get(&file), Some(Fetch::Loading)) {
            let frac = downloads.get(&file).filter(|(_, t)| *t > 0).map(|(d, t)| *d as f32 / *t as f32);
            return TrackView { state: TrackState::Loading(frac), active, ..idle };
        }
        let Some(st) = self.player.status().filter(|s| active && s.key == file as u64) else { return idle };
        let total = st.duration.or(duration).filter(|d| !d.is_zero());
        let progress = total.map_or(0.0, |t| (st.position.as_secs_f32() / t.as_secs_f32()).clamp(0.0, 1.0));
        let state = match st.state {
            PlaybackState::Starting => TrackState::Loading(None),
            PlaybackState::Playing => TrackState::Playing,
            PlaybackState::Paused => TrackState::Paused,
            // Al terminar vuelve al principio, como al empezar.
            PlaybackState::Ended => return TrackView { active: true, ..idle },
            PlaybackState::Failed => TrackState::Failed(st.error.as_ref().map_or_else(|| "No se puede reproducir".into(), short_error)),
        };
        TrackView { state, active: true, position: st.position, progress }
    }
}

/// Texto corto de un error de audio para la burbuja.
pub fn short_error(e: &AudioError) -> String {
    match e {
        AudioError::NoDevice(_) => "Sin salida de audio".into(),
        AudioError::Unsupported => "Formato no compatible".into(),
        AudioError::Corrupt(_) => "Audio dañado".into(),
    }
}

/// ¿Se dibuja como audio (nota de voz o música)?
pub fn is_audio(md: &Media) -> bool {
    matches!(md.kind, MediaKind::Voice | MediaKind::Audio)
}

/// Tamaño del medio en la burbuja para audios y vídeos redondos (`None`: el de siempre).
pub fn layout_size(md: &Media, inner_max: f32) -> Option<Size> {
    match md.kind {
        MediaKind::Voice => Some(Size::new(VOICE_W.min(inner_max), AUDIO_H)),
        MediaKind::Audio => Some(Size::new(MUSIC_W.min(inner_max), AUDIO_H)),
        MediaKind::VideoNote => {
            let s = ROUND_SIDE.min(inner_max);
            Some(Size::new(s, s))
        }
        _ => None,
    }
}

/// `m:ss` (o `h:mm:ss`).
pub fn clock(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }
}

/// Etiqueta de una velocidad («1x», «1.5x», «2x»).
pub fn speed_label(s: f32) -> String {
    if s.fract() == 0.0 { format!("{}x", s as u32) } else { format!("{s:.1}x") }
}

fn audio_of(app: &App, peer: PeerRef, msg: MsgId) -> Option<(&Media, &FileRef)> {
    let cv = app.conv.as_ref().filter(|c| c.peer == peer)?;
    let m = cv.find(msg).filter(|m| !crate::conv::is_local(m))?;
    let md = m.media.as_ref().filter(|md| is_audio(md))?;
    Some((md, md.file.as_ref()?))
}

impl App {
    pub(crate) fn on_audio(&mut self, msg: AudioMsg) -> Task<Msg> {
        match msg {
            AudioMsg::Tick => Task::none(),
            AudioMsg::CycleSpeed => {
                let i = SPEEDS.iter().position(|s| (*s - self.audio.speed).abs() < 0.01).map_or(0, |i| (i + 1) % SPEEDS.len());
                self.audio.speed = SPEEDS[i];
                self.audio.player.set_speed(self.audio.speed);
                Task::none()
            }
            AudioMsg::Seek { peer, msg, fraction } => {
                let Some((md, file)) = audio_of(self, peer, msg) else { return Task::none() };
                let (id, mime) = (file.id, md.audio.as_ref().and_then(|a| a.mime.clone()));
                let a = &mut self.audio;
                let st = a.player.status().filter(|s| s.key == id as u64 && a.current.is_some_and(|c| c.2 == id));
                match st.map(|s| (s.state, s.duration)) {
                    Some((PlaybackState::Playing | PlaybackState::Paused | PlaybackState::Starting, _)) => {
                        a.player.seek_fraction(fraction);
                        Task::none()
                    }
                    Some((PlaybackState::Ended, Some(d))) if a.bytes.contains_key(&id) => {
                        let bytes = a.bytes[&id].clone();
                        a.player.play(id as u64, bytes, mime, d.mul_f32(fraction.clamp(0.0, 1.0)), a.speed);
                        Task::none()
                    }
                    _ => self.on_audio(AudioMsg::Toggle { peer, msg }),
                }
            }
            AudioMsg::Toggle { peer, msg } => {
                let Some((md, file)) = audio_of(self, peer, msg) else { return Task::none() };
                let (fref, id, mime) = (file.clone(), file.id, md.audio.as_ref().and_then(|a| a.mime.clone()));
                let a = &mut self.audio;
                let mine = a.current.is_some_and(|c| c.2 == id);
                if let Some(st) = a.player.status().filter(|s| mine && s.key == id as u64) {
                    match st.state {
                        PlaybackState::Playing | PlaybackState::Starting => {
                            a.player.set_paused(true);
                            return Task::none();
                        }
                        PlaybackState::Paused => {
                            a.player.set_paused(false);
                            return Task::none();
                        }
                        // Terminada o con error: se vuelve a empezar.
                        PlaybackState::Ended | PlaybackState::Failed => {}
                    }
                }
                if mine && a.pending == Some(id) && matches!(a.fetch.get(&id), Some(Fetch::Loading)) {
                    // Pulsar durante la descarga: ya no se reproducirá al terminar.
                    a.pending = None;
                    a.current = None;
                    return Task::none();
                }
                // Un solo audio a la vez: lo que sonara se para.
                a.player.stop();
                a.current = Some((peer, msg, id));
                if let Some(bytes) = a.bytes.get(&id).cloned() {
                    a.pending = None;
                    a.player.play(id as u64, bytes, mime, Duration::ZERO, a.speed);
                    return Task::none();
                }
                a.pending = Some(id);
                if matches!(a.fetch.get(&id), Some(Fetch::Loading)) {
                    return Task::none();
                }
                let Some(backend) = self.chat_backend() else { return Task::none() };
                self.audio.fetch.insert(id, Fetch::Loading);
                Task::perform(
                    async move {
                        backend.download_media(peer, msg, fref, MediaVariant::Full).await.map(|b| AudioBytes(Arc::from(b))).map_err(|e| error_text(&e))
                    },
                    move |result| Msg::Audio(AudioMsg::Loaded { file: id, result }),
                )
            }
            AudioMsg::Loaded { file, result } => {
                self.downloads.remove(&file);
                let a = &mut self.audio;
                match result {
                    Ok(AudioBytes(bytes)) => {
                        a.fetch.remove(&file);
                        a.remember(file, bytes.clone());
                        if a.pending == Some(file) && a.current.is_some_and(|c| c.2 == file) {
                            a.pending = None;
                            let (peer, msg, _) = a.current.unwrap_or((PeerRef::User(UserId(0)), MsgId(0), file));
                            let mime = audio_of(self, peer, msg).and_then(|(md, _)| md.audio.as_ref().and_then(|x| x.mime.clone()));
                            let a = &mut self.audio;
                            a.player.play(file as u64, bytes, mime, Duration::ZERO, a.speed);
                        }
                    }
                    Err(e) => {
                        tracing::debug!(file, error = %e, "no se pudo descargar el audio");
                        a.fetch.insert(file, Fetch::Failed(e));
                        if a.pending == Some(file) {
                            a.pending = None;
                        }
                    }
                }
                Task::none()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_sizes() {
        assert_eq!(clock(Duration::from_secs(7)), "0:07");
        assert_eq!(clock(Duration::from_secs(615)), "10:15");
        assert_eq!(clock(Duration::from_secs(3725)), "1:02:05");
        assert_eq!(speed_label(1.0), "1x");
        assert_eq!(speed_label(1.5), "1.5x");
        assert_eq!(speed_label(2.0), "2x");
        let md = Media { id: 1, kind: MediaKind::Voice, file_name: None, size_bytes: 1, width: 0, height: 0, duration_secs: Some(3), local_path: None, file: None, audio: None };
        assert_eq!(layout_size(&md, 400.0), Some(Size::new(VOICE_W, AUDIO_H)));
        assert_eq!(layout_size(&Media { kind: MediaKind::VideoNote, ..md.clone() }, 150.0), Some(Size::new(150.0, 150.0)));
        assert_eq!(layout_size(&Media { kind: MediaKind::Photo, ..md }, 400.0), None);
    }

    #[test]
    fn byte_cache_keeps_the_most_recent() {
        let mut a = AudioState::with_player(Player::with_output(|| Box::new(tc_audio::NullOutput::default())));
        let big: Arc<[u8]> = vec![0u8; BYTES_CACHE / 2 + 1].into();
        a.remember(1, big.clone());
        a.remember(2, big.clone());
        assert!(!a.is_cached(1) && a.is_cached(2));
        a.remember(3, vec![0u8; BYTES_CACHE + 1].into());
        assert!(!a.is_cached(3), "lo que no cabe no se guarda");
    }
}
