//! Notas de voz contra el `MockBackend` (WAV sintéticos): antes se dibujaban como una caja de foto
//! «0 × 0» sin forma de reproducirlas. Ahora: burbuja de audio, descarga perezosa al pulsar, un solo
//! audio a la vez, pausa, velocidad, error con reintento y sin dispositivo de salida.
//! El reproductor usa salidas falsas (nada suena) y las `Task` se resuelven a mano.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tc_audio::{AudioError, NullOutput, Output, Player};
use tc_core::*;
use tc_mock::{MockBackend, MockConfig};
use tc_theme::ThemeKind;

use crate::app::{App, Backend, Flags, Msg};
use crate::audio::{self, AudioBytes, AudioMsg, AudioState, TrackState};
use crate::conv::PAGE;

const NOW: i64 = 1_762_430_400;

async fn boot(player: Player) -> (App, Backend) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock;
    let (mut app, _) = App::new(Flags { theme: ThemeKind::Dark, ..Flags::new(backend.clone()) });
    app.audio = AudioState::with_player(player);
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    (app, backend)
}

/// Abre un chat con al menos dos notas de voz recientes y las devuelve.
async fn open_with_voices(app: &mut App, backend: &Backend) -> (PeerRef, Vec<Message>) {
    for d in app.dialogs.clone() {
        if matches!(d.peer, PeerRef::Secret(_)) {
            continue;
        }
        let h = backend.history(d.peer, Anchor::Latest, PAGE).await.unwrap();
        let voices: Vec<Message> = h.iter().filter(|m| m.media.as_ref().is_some_and(|md| md.kind == MediaKind::Voice)).cloned().collect();
        if voices.len() >= 2 {
            let _ = app.update(Msg::Open(d.peer));
            let _ = app.update(Msg::History { peer: d.peer, older: false, result: Ok(h) });
            return (d.peer, voices);
        }
    }
    panic!("el mock no tiene un chat con dos notas de voz");
}

fn file_of(m: &Message) -> i64 {
    m.media.as_ref().and_then(|md| md.file.as_ref()).map(|f| f.id).unwrap()
}

fn state(app: &App, m: &Message) -> TrackState {
    app.audio.view(file_of(m), None, &app.downloads).state
}

/// Descarga como lo haría la tarea de iced y entrega el resultado.
async fn deliver(app: &mut App, backend: &Backend, peer: PeerRef, m: &Message) {
    let f = m.media.as_ref().and_then(|md| md.file.clone()).unwrap();
    let id = f.id;
    let r = backend.download_media(peer, m.id, f, MediaVariant::Full).await.map(|b| AudioBytes(b.into())).map_err(|e| e.to_string());
    let _ = app.update(Msg::Audio(AudioMsg::Loaded { file: id, result: r }));
}

fn wait(app: &App, m: &Message, want: impl Fn(&TrackState) -> bool) -> TrackState {
    let t0 = Instant::now();
    loop {
        let s = state(app, m);
        if want(&s) {
            return s;
        }
        assert!(t0.elapsed() < Duration::from_secs(10), "tiempo agotado: {s:?}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[tokio::test]
async fn nota_de_voz_se_descarga_al_pulsar_y_se_reproduce_de_una_en_una() {
    let (mut app, backend) = boot(Player::with_output(|| Box::new(NullOutput::default()))).await;
    let (peer, voices) = open_with_voices(&mut app, &backend).await;
    let (a, b) = (&voices[0], &voices[1]);
    let md = a.media.as_ref().unwrap();
    // Burbuja de audio compacta (antes: caja de foto de 270 px de alto con «0 × 0»).
    assert_eq!(audio::layout_size(md, 500.0).map(|s| s.height), Some(audio::AUDIO_H));
    assert_eq!(md.audio.as_ref().map(|x| (x.voice, x.waveform.len())), Some((true, 100)));
    assert_eq!(state(&app, a), TrackState::Idle);
    // Dibujar la burbuja no falla en ningún estado.
    let _ = crate::view::audio::bubble(&app, peer, a, md, iced::Size::new(260.0, audio::AUDIO_H), 1.0);

    // Pulsar: descarga perezosa (nada se pidió antes).
    assert!(!app.audio.is_cached(file_of(a)));
    let _ = app.update(Msg::Audio(AudioMsg::Toggle { peer, msg: a.id }));
    assert_eq!(state(&app, a), TrackState::Loading(None));
    // Progreso de la red mientras descarga.
    let _ = app.update(Msg::Backend(BackendEvent::Download { file_id: file_of(a), done: 25, total: 100 }));
    assert_eq!(state(&app, a), TrackState::Loading(Some(0.25)));
    let _ = crate::view::audio::bubble(&app, peer, a, md, iced::Size::new(260.0, audio::AUDIO_H), 1.0);
    deliver(&mut app, &backend, peer, a).await;
    assert!(app.audio.is_cached(file_of(a)));
    assert!(app.downloads.is_empty());
    wait(&app, a, |s| *s == TrackState::Playing);
    assert!(app.audio.needs_tick());
    let _ = crate::view::audio::bubble(&app, peer, a, md, iced::Size::new(260.0, audio::AUDIO_H), 1.0);

    // Pausa, reanuda y velocidad.
    let _ = app.update(Msg::Audio(AudioMsg::Toggle { peer, msg: a.id }));
    wait(&app, a, |s| *s == TrackState::Paused);
    let _ = app.update(Msg::Audio(AudioMsg::Toggle { peer, msg: a.id }));
    wait(&app, a, |s| *s == TrackState::Playing);
    let _ = app.update(Msg::Audio(AudioMsg::CycleSpeed));
    assert_eq!(app.audio.player.status().map(|s| s.speed), Some(1.5));
    let _ = app.update(Msg::Audio(AudioMsg::CycleSpeed));
    let _ = app.update(Msg::Audio(AudioMsg::CycleSpeed));
    assert_eq!(app.audio.speed, 1.0);

    // Otra nota: la primera deja de sonar (un solo audio a la vez).
    let _ = app.update(Msg::Audio(AudioMsg::Toggle { peer, msg: b.id }));
    deliver(&mut app, &backend, peer, b).await;
    wait(&app, b, |s| *s == TrackState::Playing);
    assert_eq!(state(&app, a), TrackState::Idle);
    // La primera ya está en memoria: volver a ella no descarga.
    let _ = app.update(Msg::Audio(AudioMsg::Toggle { peer, msg: a.id }));
    wait(&app, a, |s| *s == TrackState::Playing);
    // Pulsar en la forma de onda salta.
    let _ = app.update(Msg::Audio(AudioMsg::Seek { peer, msg: a.id, fraction: 0.5 }));
    assert_eq!(state(&app, b), TrackState::Idle);
}

#[tokio::test]
async fn descarga_fallida_muestra_error_y_se_reintenta() {
    let (mut app, backend) = boot(Player::with_output(|| Box::new(NullOutput::default()))).await;
    let (peer, voices) = open_with_voices(&mut app, &backend).await;
    let a = &voices[0];
    let _ = app.update(Msg::Audio(AudioMsg::Toggle { peer, msg: a.id }));
    let _ = app.update(Msg::Audio(AudioMsg::Loaded { file: file_of(a), result: Err("Sin conexión".into()) }));
    assert_eq!(state(&app, a), TrackState::Failed("Sin conexión".into()));
    let md = a.media.as_ref().unwrap();
    let _ = crate::view::audio::bubble(&app, peer, a, md, iced::Size::new(260.0, audio::AUDIO_H), 1.0);
    // Reintentar vuelve a descargar.
    let _ = app.update(Msg::Audio(AudioMsg::Toggle { peer, msg: a.id }));
    assert_eq!(state(&app, a), TrackState::Loading(None));
    deliver(&mut app, &backend, peer, a).await;
    wait(&app, a, |s| *s == TrackState::Playing);
}

/// Salida que no existe (equipo sin tarjeta de sonido).
struct NoDevice;

impl Output for NoDevice {
    fn start(&mut self, _: tc_audio::player::Track, _: f32, _: bool) -> Result<(), AudioError> {
        Err(AudioError::NoDevice("no hay dispositivos".into()))
    }
    fn set_paused(&mut self, _: bool) {}
    fn set_speed(&mut self, _: f32) {}
    fn stop(&mut self) {}
}

#[tokio::test]
async fn sin_dispositivo_de_salida_hay_error_claro_sin_panico() {
    let (mut app, backend) = boot(Player::with_output(|| Box::new(NoDevice))).await;
    let (peer, voices) = open_with_voices(&mut app, &backend).await;
    let a = &voices[0];
    let _ = app.update(Msg::Audio(AudioMsg::Toggle { peer, msg: a.id }));
    deliver(&mut app, &backend, peer, a).await;
    assert_eq!(wait(&app, a, |s| matches!(s, TrackState::Failed(_))), TrackState::Failed("Sin salida de audio".into()));
    assert!(!app.audio.needs_tick());
}
