//! Reproductor: un único audio a la vez, controlado desde la UI sin bloquearla.
//!
//! [`Player`] es un asa barata (`Clone`). Cada orden se manda por un canal a un hilo propio
//! (`parlo-audio`) que abre la salida al primer uso y la conserva; abrir el dispositivo o
//! decodificar la cabecera nunca ocurre en el hilo de la UI. El estado (posición, fin, error) se lee
//! con [`Player::status`] sin pasar por ese hilo: lo escribe la fuente de audio en atómicos.
//!
//! La posición se cuenta en frames de la fuente (antes de aplicar la velocidad), así que es la
//! posición real dentro del archivo sea cual sea la velocidad.
use std::num::NonZero;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use crate::AudioError;
use crate::decode::{AudioStream, open};

/// Velocidades que ofrece la UI.
pub const SPEEDS: [f32; 3] = [1.0, 1.5, 2.0];

/// Salida de audio. La real es [`RodioOutput`]; las pruebas usan una propia.
pub trait Output: Send + 'static {
    /// Empieza a sonar `track` sustituyendo lo anterior.
    fn start(&mut self, track: Track, speed: f32, paused: bool) -> Result<(), AudioError>;
    fn set_paused(&mut self, paused: bool);
    fn set_speed(&mut self, speed: f32);
    fn stop(&mut self);
}

/// Estado compartido entre la fuente (hilo de audio del sistema) y la UI.
#[derive(Debug, Default)]
struct Shared {
    /// Frames entregados desde el inicio del archivo.
    frame: AtomicU64,
    /// Salto pendiente (en frames), `u64::MAX` si no hay.
    seek: AtomicU64,
    ended: AtomicBool,
    paused: AtomicBool,
    rate: AtomicU64,
    /// Duración en milisegundos (0 si no se conoce).
    total_ms: AtomicU64,
    error: Mutex<Option<AudioError>>,
}

impl Shared {
    fn new() -> Arc<Self> {
        Arc::new(Self { seek: AtomicU64::new(u64::MAX), ..Default::default() })
    }

    fn fail(&self, e: AudioError) {
        *self.error.lock().unwrap_or_else(PoisonError::into_inner) = Some(e);
        self.ended.store(true, Ordering::SeqCst);
    }
}

/// Fuente de audio para rodio: decodifica a trozos y publica la posición.
pub struct Track {
    stream: Box<dyn AudioStream>,
    shared: Arc<Shared>,
    buf: Vec<f32>,
    i: usize,
    channels: NonZero<u16>,
    rate: NonZero<u32>,
    ch: u16,
    done: bool,
}

/// Ejecuta una operación del decodificador convirtiendo un pánico en error: esto corre en el hilo de
/// audio del sistema y un pánico ahí dejaría la app sin sonido.
fn guarded<T>(f: impl FnOnce() -> Result<T, AudioError>) -> Result<T, AudioError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| Err(AudioError::Corrupt("fallo interno del decodificador".into())))
}

impl Track {
    fn new(stream: Box<dyn AudioStream>, shared: Arc<Shared>) -> Result<Self, AudioError> {
        let channels = NonZero::new(stream.channels()).ok_or(AudioError::Unsupported)?;
        let rate = NonZero::new(stream.sample_rate()).ok_or(AudioError::Unsupported)?;
        shared.rate.store(u64::from(rate.get()), Ordering::SeqCst);
        shared.total_ms.store(stream.duration().map_or(0, |d| d.as_millis() as u64), Ordering::SeqCst);
        Ok(Self { stream, shared, buf: Vec::new(), i: 0, channels, rate, ch: 0, done: false })
    }

    /// Siguiente muestra; `None` al terminar o ante un error (que queda en el estado).
    fn next_sample(&mut self) -> Option<f32> {
        if self.done {
            return None;
        }
        if self.ch == 0 {
            let seek = self.shared.seek.swap(u64::MAX, Ordering::SeqCst);
            if seek != u64::MAX {
                let to = Duration::from_secs_f64(seek as f64 / f64::from(self.rate.get()));
                let stream = &mut self.stream;
                if let Err(e) = guarded(|| stream.seek(to)) {
                    self.done = true;
                    self.shared.fail(e);
                    return None;
                }
                self.buf.clear();
                self.i = 0;
                self.shared.frame.store(seek, Ordering::SeqCst);
            }
            if self.i >= self.buf.len() {
                self.i = 0;
                let (stream, buf) = (&mut self.stream, &mut self.buf);
                match guarded(|| stream.next_chunk(buf)) {
                    Ok(true) if !self.buf.is_empty() => {}
                    Ok(_) => {
                        self.done = true;
                        self.shared.ended.store(true, Ordering::SeqCst);
                        return None;
                    }
                    Err(e) => {
                        self.done = true;
                        self.shared.fail(e);
                        return None;
                    }
                }
            }
            self.shared.frame.fetch_add(1, Ordering::Relaxed);
        }
        let s = self.buf.get(self.i).copied();
        self.i += 1;
        self.ch = (self.ch + 1) % self.channels.get();
        s
    }
}

impl Iterator for Track {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_sample()
    }
}

impl rodio::Source for Track {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.stream.duration()
    }
}

/// Salida real con rodio (cpal): ALSA en Linux, WASAPI en Windows, CoreAudio en macOS.
#[derive(Default)]
pub struct RodioOutput {
    sink: Option<rodio::MixerDeviceSink>,
    player: Option<rodio::Player>,
}

impl Output for RodioOutput {
    fn start(&mut self, track: Track, speed: f32, paused: bool) -> Result<(), AudioError> {
        if self.sink.is_none() {
            let mut sink = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| AudioError::NoDevice(e.to_string()))?;
            sink.log_on_drop(false);
            self.sink = Some(sink);
        }
        let Some(sink) = &self.sink else { return Err(AudioError::NoDevice("sin salida".into())) };
        let player = rodio::Player::connect_new(sink.mixer());
        player.set_speed(speed);
        if paused {
            player.pause();
        }
        player.append(track);
        // Soltar el anterior lo detiene.
        self.player = Some(player);
        Ok(())
    }

    fn set_paused(&mut self, paused: bool) {
        if let Some(p) = &self.player {
            if paused { p.pause() } else { p.play() }
        }
    }

    fn set_speed(&mut self, speed: f32) {
        if let Some(p) = &self.player {
            p.set_speed(speed);
        }
    }

    fn stop(&mut self) {
        self.player = None;
    }
}

/// Salida muda: acepta la pista y no la consume (pruebas de la UI sin tocar el sistema de sonido).
#[derive(Debug, Default)]
pub struct NullOutput {
    track: Option<Track>,
}

impl Output for NullOutput {
    fn start(&mut self, track: Track, _speed: f32, _paused: bool) -> Result<(), AudioError> {
        self.track = Some(track);
        Ok(())
    }

    fn set_paused(&mut self, _: bool) {}

    fn set_speed(&mut self, _: f32) {}

    fn stop(&mut self) {
        self.track = None;
    }
}

impl std::fmt::Debug for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Track").field("channels", &self.channels).field("rate", &self.rate).finish_non_exhaustive()
    }
}

enum Cmd {
    Play { bytes: Arc<[u8]>, mime: Option<String>, start: Duration, speed: f32, shared: Arc<Shared> },
    Paused(bool),
    Speed(f32),
    Stop,
}

/// Qué está pasando con la pista actual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    /// Abriendo la salida o decodificando la cabecera.
    Starting,
    Playing,
    Paused,
    Ended,
    Failed,
}

/// Instantánea del reproductor.
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    /// Clave de la pista (la pone quien llama; p. ej. el id del archivo).
    pub key: u64,
    pub state: PlaybackState,
    pub position: Duration,
    /// Duración real del archivo, si ya se conoce.
    pub duration: Option<Duration>,
    pub speed: f32,
    pub error: Option<AudioError>,
}

struct Current {
    key: u64,
    shared: Arc<Shared>,
    speed: f32,
}

/// Asa del reproductor. Crearla no abre nada; el hilo de audio arranca con la primera orden.
#[derive(Clone)]
pub struct Player {
    tx: Arc<Mutex<Option<Sender<Cmd>>>>,
    current: Arc<Mutex<Option<Current>>>,
    make: Arc<dyn Fn() -> Box<dyn Output> + Send + Sync>,
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player").field("status", &self.status()).finish_non_exhaustive()
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Player {
    /// Reproductor con la salida real del sistema.
    pub fn new() -> Self {
        Self::with_output(|| Box::new(RodioOutput::default()))
    }

    /// Reproductor con otra salida (pruebas).
    pub fn with_output(make: impl Fn() -> Box<dyn Output> + Send + Sync + 'static) -> Self {
        Self { tx: Arc::default(), current: Arc::default(), make: Arc::new(make) }
    }

    fn send(&self, cmd: Cmd) {
        let mut tx = lock(&self.tx);
        let cmd = match tx.as_ref() {
            Some(t) => match t.send(cmd) {
                Ok(()) => return,
                // El hilo murió (no debería): se arranca otro con la misma orden.
                Err(e) => e.0,
            },
            None => cmd,
        };
        let (t, rx) = channel();
        let out = (self.make)();
        let spawned = std::thread::Builder::new().name("parlo-audio".into()).spawn(move || run(out, rx));
        if let Err(e) = spawned {
            *tx = None;
            drop(tx);
            tracing::warn!(error = %e, "no se pudo crear el hilo de audio");
            if let Some(c) = lock(&self.current).as_ref() {
                c.shared.fail(AudioError::NoDevice(e.to_string()));
            }
            return;
        }
        let _ = t.send(cmd);
        *tx = Some(t);
    }

    /// Reproduce `bytes` desde `start` (sustituye a lo que sonara). `key` identifica la pista.
    pub fn play(&self, key: u64, bytes: Arc<[u8]>, mime: Option<String>, start: Duration, speed: f32) {
        let shared = Shared::new();
        shared.frame.store(0, Ordering::SeqCst);
        *lock(&self.current) = Some(Current { key, shared: shared.clone(), speed });
        self.send(Cmd::Play { bytes, mime, start, speed, shared });
    }

    /// Pausa o reanuda la pista actual.
    pub fn set_paused(&self, paused: bool) {
        if let Some(c) = lock(&self.current).as_ref() {
            c.shared.paused.store(paused, Ordering::SeqCst);
        } else {
            return;
        }
        self.send(Cmd::Paused(paused));
    }

    pub fn set_speed(&self, speed: f32) {
        if let Some(c) = lock(&self.current).as_mut() {
            c.speed = speed;
        } else {
            return;
        }
        self.send(Cmd::Speed(speed));
    }

    /// Salta a una fracción (0..=1) de la pista actual. Sin duración conocida no hace nada.
    pub fn seek_fraction(&self, f: f32) {
        if let Some(c) = lock(&self.current).as_ref() {
            let (rate, total) = (c.shared.rate.load(Ordering::SeqCst), c.shared.total_ms.load(Ordering::SeqCst));
            if rate > 0 && total > 0 {
                let frame = (f64::from(f.clamp(0.0, 1.0)) * total as f64 / 1000.0 * rate as f64) as u64;
                c.shared.ended.store(false, Ordering::SeqCst);
                c.shared.seek.store(frame, Ordering::SeqCst);
            }
        }
    }

    /// Para y olvida la pista actual.
    pub fn stop(&self) {
        if lock(&self.current).take().is_some() {
            self.send(Cmd::Stop);
        }
    }

    /// Estado de la pista actual (`None` si no hay).
    pub fn status(&self) -> Option<Status> {
        let cur = lock(&self.current);
        let c = cur.as_ref()?;
        let s = &c.shared;
        let error = lock(&s.error).clone();
        let rate = s.rate.load(Ordering::SeqCst);
        let total = s.total_ms.load(Ordering::SeqCst);
        let duration = (total > 0).then(|| Duration::from_millis(total));
        let mut position = if rate > 0 { Duration::from_secs_f64(s.frame.load(Ordering::Relaxed) as f64 / rate as f64) } else { Duration::ZERO };
        if let Some(d) = duration {
            position = position.min(d);
        }
        let state = if error.is_some() {
            PlaybackState::Failed
        } else if s.ended.load(Ordering::SeqCst) {
            PlaybackState::Ended
        } else if rate == 0 {
            PlaybackState::Starting
        } else if s.paused.load(Ordering::SeqCst) {
            PlaybackState::Paused
        } else {
            PlaybackState::Playing
        };
        Some(Status { key: c.key, state, position, duration, speed: c.speed, error })
    }
}

/// Bucle del hilo de audio.
fn run(mut out: Box<dyn Output>, rx: Receiver<Cmd>) {
    let mut current: Option<Arc<Shared>> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Play { bytes, mime, start, speed, shared } => {
                out.stop();
                let opened = guarded(|| open(bytes, mime.as_deref())).and_then(|mut s| {
                    if !start.is_zero() {
                        s.seek(start)?;
                    }
                    Track::new(s, shared.clone())
                });
                match opened {
                    Ok(track) => {
                        if !start.is_zero() {
                            let rate = shared.rate.load(Ordering::SeqCst) as f64;
                            shared.frame.store((start.as_secs_f64() * rate) as u64, Ordering::SeqCst);
                        }
                        let paused = shared.paused.load(Ordering::SeqCst);
                        if let Err(e) = out.start(track, speed, paused) {
                            tracing::warn!(error = %e, "no se pudo abrir la salida de audio");
                            shared.fail(e);
                        }
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "no se pudo decodificar el audio");
                        shared.fail(e);
                    }
                }
                current = Some(shared);
            }
            Cmd::Paused(p) => {
                if current.is_some() {
                    out.set_paused(p);
                }
            }
            Cmd::Speed(s) => out.set_speed(s),
            Cmd::Stop => {
                out.stop();
                current = None;
            }
        }
    }
    out.stop();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Salida falsa: consume la pista en un hilo a velocidad x50 (o falla si `fail`).
    struct FakeOutput {
        fail: bool,
        paused: Arc<AtomicBool>,
        stop: Arc<AtomicBool>,
    }

    impl Output for FakeOutput {
        fn start(&mut self, mut track: Track, _speed: f32, paused: bool) -> Result<(), AudioError> {
            if self.fail {
                return Err(AudioError::NoDevice("sin tarjeta de sonido".into()));
            }
            self.stop.store(true, Ordering::SeqCst);
            let (p, s) = (Arc::new(AtomicBool::new(paused)), Arc::new(AtomicBool::new(false)));
            (self.paused, self.stop) = (p.clone(), s.clone());
            std::thread::spawn(move || {
                let per_ms = usize::from(track.channels.get()) * track.rate.get() as usize / 1000 * 50;
                while !s.load(Ordering::SeqCst) {
                    if p.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    for _ in 0..per_ms {
                        if track.next().is_none() {
                            return;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            });
            Ok(())
        }

        fn set_paused(&mut self, paused: bool) {
            self.paused.store(paused, Ordering::SeqCst);
        }

        fn set_speed(&mut self, _: f32) {}

        fn stop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
        }
    }

    fn fake(fail: bool) -> Player {
        Player::with_output(move || Box::new(FakeOutput { fail, paused: Arc::default(), stop: Arc::default() }))
    }

    fn wait_for(p: &Player, what: impl Fn(&Status) -> bool) -> Status {
        let t0 = Instant::now();
        loop {
            let s = p.status().expect("hay pista");
            if what(&s) {
                return s;
            }
            assert!(t0.elapsed() < Duration::from_secs(10), "tiempo agotado; estado {s:?}");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn voice() -> Arc<[u8]> {
        std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/voice.ogg")).unwrap().into()
    }

    #[test]
    fn plays_to_the_end_and_reports_position() {
        let p = fake(false);
        assert!(p.status().is_none());
        p.play(7, voice(), Some("audio/ogg".into()), Duration::ZERO, 1.5);
        let s = wait_for(&p, |s| s.state == PlaybackState::Ended);
        assert_eq!((s.key, s.speed), (7, 1.5));
        assert_eq!(s.duration, Some(Duration::from_secs(1)));
        assert_eq!(s.position, Duration::from_secs(1));
        assert!(s.error.is_none());
    }

    #[test]
    fn pause_seek_and_replace() {
        let p = fake(false);
        p.play(1, voice(), None, Duration::from_millis(500), 1.0);
        p.set_paused(true);
        let s = wait_for(&p, |s| s.state == PlaybackState::Paused);
        assert!(s.position >= Duration::from_millis(500), "empieza donde se pidió: {s:?}");
        p.seek_fraction(0.9);
        p.set_paused(false);
        wait_for(&p, |s| s.state == PlaybackState::Ended);
        // Otra pista sustituye a la anterior (un solo audio a la vez).
        p.play(2, voice(), None, Duration::ZERO, 2.0);
        assert_eq!(p.status().map(|s| s.key), Some(2));
        p.stop();
        assert!(p.status().is_none());
    }

    #[test]
    fn missing_device_and_bad_audio_are_errors() {
        let p = fake(true);
        p.play(3, voice(), None, Duration::ZERO, 1.0);
        let s = wait_for(&p, |s| s.state == PlaybackState::Failed);
        assert!(matches!(s.error, Some(AudioError::NoDevice(_))));
        let p = fake(false);
        p.play(4, Arc::from(&b"no es audio"[..]), None, Duration::ZERO, 1.0);
        let s = wait_for(&p, |s| s.state == PlaybackState::Failed);
        assert_eq!(s.error, Some(AudioError::Unsupported));
    }
}
