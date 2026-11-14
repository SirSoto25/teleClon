//! Pruebas de contrato de las transferencias (ADR-0011) contra el mock.
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use futures::stream::{BoxStream, StreamExt};
use tc_core::*;
use tokio::time::Instant;

use crate::{MockBackend, MockConfig, TransferSim};

const NOW: i64 = 1_800_000_000;
const PEER: PeerRef = PeerRef::User(UserId(1001));

fn mock() -> MockBackend {
    MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() })
}

fn sim(chunk: u64, delay_ms: u64) -> TransferSim {
    TransferSim { chunk, delay: Duration::from_millis(delay_ms), fail_at: None }
}

/// Directorio temporal propio que se borra al terminar.
struct TmpDir(PathBuf);
impl TmpDir {
    fn new(tag: &str) -> Self {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("tc-mock-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn item(name: &str, size: u64) -> OutgoingMediaItem {
    OutgoingMediaItem { kind: MediaKind::File, path: format!("/tmp/{name}"), file_name: name.into(), size_bytes: size, width: 0, height: 0 }
}

fn media(items: Vec<OutgoingMediaItem>, rid: i64) -> OutgoingMedia {
    let random_ids = (0..items.len() as i64).map(|i| RandomId(rid + i)).collect();
    OutgoingMedia { items, caption: "pie".into(), reply_to: None, random_ids }
}

/// Envía un documento de `size` bytes y devuelve su mensaje y archivo.
async fn doc(b: &MockBackend, size: u64) -> (MsgId, FileRef) {
    let m = b.send_media(PEER, media(vec![item("informe.pdf", size)], 900)).await.unwrap().remove(0);
    (m.id, m.media.unwrap().file.unwrap())
}

/// Siguiente evento de la transferencia `id` (con tiempo virtual o real, una hora de margen).
async fn next_of(ev: &mut BoxStream<'static, BackendEvent>, id: TransferId) -> Transfer {
    tokio::time::timeout(Duration::from_secs(3600), async {
        loop {
            if let Some(BackendEvent::TransferProgress(t)) = ev.next().await
                && t.id == id
            {
                return t;
            }
        }
    })
    .await
    .expect("evento de transferencia")
}

/// Todos los eventos de `id` hasta el terminal (incluido).
async fn until_end(ev: &mut BoxStream<'static, BackendEvent>, id: TransferId) -> Vec<Transfer> {
    let mut out = Vec::new();
    loop {
        let t = next_of(ev, id).await;
        let end = t.state.is_finished();
        out.push(t);
        if end {
            return out;
        }
    }
}

fn assert_well_formed(list: &[Transfer]) {
    let (last, running) = list.split_last().unwrap();
    assert!(running.iter().all(|t| t.state == TransferState::Running));
    assert!(list.windows(2).all(|w| w[0].done <= w[1].done), "progreso no monótono");
    assert!(list.iter().all(|t| t.done <= t.total && t.total == list[0].total));
    assert!(last.state.is_finished());
}

#[tokio::test]
async fn descarga_a_disco_por_trozos_y_renombra_al_final() {
    let b = mock();
    b.set_transfer_sim(sim(100, 0));
    let (msg, file) = doc(&b, 1000).await;
    let tmp = TmpDir::new("dl");
    let dest = tmp.0.join("informe.pdf");
    let mut ev = b.events();
    let id = b.download_media_to(PEER, msg, file.clone(), MediaVariant::Full, dest.clone()).await.unwrap();
    assert!(matches!(id, TransferId::Download(_)));
    let list = until_end(&mut ev, id).await;
    assert_well_formed(&list);
    assert_eq!(list.len(), 12, "Running(0) + 10 trozos + Done");
    let last = list.last().unwrap();
    assert_eq!(last.state, TransferState::Done);
    assert_eq!((last.done, last.total), (1000, 1000));
    assert_eq!((last.kind, last.peer, last.msg, last.name.as_str()), (TransferKind::Download, PEER, Some(msg), "informe.pdf"));
    // Con `Done` el archivo ya está en su sitio, entero y sin parcial.
    let want = b.download_media(PEER, msg, file, MediaVariant::Full).await.unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), want);
    assert!(!partial_path(&dest).exists());
    assert_eq!(b.active_transfers(), 0);
}

#[tokio::test(start_paused = true)]
async fn el_ritmo_lo_marca_el_reloj_de_tokio() {
    let b = mock();
    let (msg, file) = doc(&b, 1000).await;
    b.set_transfer_sim(sim(100, 1000));
    let tmp = TmpDir::new("clock");
    let mut ev = b.events();
    let t0 = Instant::now();
    let id = b.download_media_to(PEER, msg, file, MediaVariant::Full, tmp.0.join("a.bin")).await.unwrap();
    assert_eq!(next_of(&mut ev, id).await.done, 0);
    for k in 1..=3u64 {
        let t = next_of(&mut ev, id).await;
        assert_eq!(t.done, k * 100);
        assert_eq!(t0.elapsed(), Duration::from_secs(k), "un trozo por segundo virtual");
    }
    let list = until_end(&mut ev, id).await;
    assert_eq!(list.last().unwrap().state, TransferState::Done);
    assert_eq!(t0.elapsed(), Duration::from_secs(10));
}

#[tokio::test(start_paused = true)]
async fn cancelar_una_descarga_a_mitad_borra_el_parcial() {
    let b = mock();
    let (msg, file) = doc(&b, 1000).await;
    b.set_transfer_sim(sim(100, 1000));
    let tmp = TmpDir::new("cancel");
    let dest = tmp.0.join("a.bin");
    let mut ev = b.events();
    let id = b.download_media_to(PEER, msg, file, MediaVariant::Full, dest.clone()).await.unwrap();
    while next_of(&mut ev, id).await.done < 300 {}
    assert!(partial_path(&dest).exists(), "mientras dura, escribe en el parcial");
    b.cancel_transfer(id).await.unwrap();
    let list = until_end(&mut ev, id).await;
    let last = list.last().unwrap();
    assert_eq!(last.state, TransferState::Cancelled);
    assert_eq!(last.done, 300, "no avanza tras cancelar");
    assert!(!dest.exists() && !partial_path(&dest).exists());
    // Idempotente: otra vez, o un id desconocido, no es un error.
    b.cancel_transfer(id).await.unwrap();
    b.cancel_transfer(TransferId::Download(9999)).await.unwrap();
    assert_eq!(b.active_transfers(), 0);
}

#[tokio::test]
async fn un_fallo_inyectado_no_toca_el_destino_y_reintentar_lo_reemplaza() {
    let b = mock();
    let (msg, file) = doc(&b, 1000).await;
    let tmp = TmpDir::new("fail");
    let dest = tmp.0.join("a.bin");
    std::fs::write(&dest, b"viejo").unwrap();
    b.set_transfer_sim(TransferSim { fail_at: Some((500, BackendError::Network)), ..sim(100, 0) });
    let mut ev = b.events();
    let id = b.download_media_to(PEER, msg, file.clone(), MediaVariant::Full, dest.clone()).await.unwrap();
    let list = until_end(&mut ev, id).await;
    assert_well_formed(&list);
    let last = list.last().unwrap();
    assert_eq!(last.state, TransferState::Failed(BackendError::Network));
    assert_eq!(last.done, 500);
    assert_eq!(std::fs::read(&dest).unwrap(), b"viejo");
    assert!(!partial_path(&dest).exists());
    // Reintento sin fallo: id nuevo y el destino se reemplaza entero.
    b.set_transfer_sim(sim(100, 0));
    let id2 = b.download_media_to(PEER, msg, file, MediaVariant::Full, dest.clone()).await.unwrap();
    assert_ne!(id, id2);
    assert_eq!(until_end(&mut ev, id2).await.last().unwrap().state, TransferState::Done);
    assert_eq!(std::fs::read(&dest).unwrap().len(), 1000);
}

#[tokio::test]
async fn errores_inmediatos_sin_eventos() {
    let b = mock();
    let (msg, file) = doc(&b, 100).await;
    let tmp = TmpDir::new("bad");
    let mut ev = b.events();
    let r = b.download_media_to(PEER, msg, file.clone(), MediaVariant::Full, tmp.0.join("no-existe/a.bin")).await;
    assert_eq!(r, Err(BackendError::InvalidInput("dest")));
    let r = b.download_media_to(PEER, msg, file.clone(), MediaVariant::Full, tmp.0.clone()).await;
    assert_eq!(r, Err(BackendError::InvalidInput("dest")));
    let r = b.download_media_to(PEER, MsgId(99_999), file.clone(), MediaVariant::Full, tmp.0.join("a.bin")).await;
    assert_eq!(r, Err(BackendError::NotFound));
    b.set_offline(true);
    let r = b.download_media_to(PEER, msg, file, MediaVariant::Full, tmp.0.join("a.bin")).await;
    assert_eq!(r, Err(BackendError::Network));
    // Ningún `TransferProgress` (solo el cambio de conexión).
    while let Some(Some(e)) = ev.next().now_or_never() {
        assert!(!matches!(e, BackendEvent::TransferProgress(_)), "{e:?}");
    }
    assert_eq!(b.active_transfers(), 0);
}

#[tokio::test]
async fn subida_con_progreso_y_mensajes_antes_de_done() {
    let b = mock();
    b.set_transfer_sim(sim(100, 0));
    let mut ev = b.events();
    let m = media(vec![item("a.pdf", 300), item("b.zip", 700)], 500);
    let id = TransferId::upload_of(&m).unwrap();
    let sent = b.send_media(PEER, m).await.unwrap();
    assert_eq!(sent.len(), 2);
    // Recorre el flujo en orden: los `MessageSent` llegan antes del `Done`.
    let mut seen_sent = 0;
    let mut list = Vec::new();
    while let Some(e) = ev.next().now_or_never().flatten() {
        match e {
            BackendEvent::MessageSent { .. } => seen_sent += 1,
            BackendEvent::TransferProgress(t) if t.id == id => {
                if t.state == TransferState::Done {
                    assert_eq!(seen_sent, 2, "Done después de los MessageSent");
                }
                list.push(t);
            }
            _ => {}
        }
    }
    assert_well_formed(&list);
    let last = list.last().unwrap();
    assert_eq!(last.state, TransferState::Done);
    assert_eq!((last.kind, last.msg, last.name.as_str(), last.done, last.total), (TransferKind::Upload, None, "a.pdf", 1000, 1000));
    assert_eq!(list.len(), 12);
}

#[tokio::test(start_paused = true)]
async fn cancelar_una_subida_no_envia_nada() {
    let b = Arc::new(mock());
    b.set_transfer_sim(sim(100, 1000));
    let mut ev = b.events();
    let m = media(vec![item("grande.iso", 1000)], 700);
    let id = TransferId::upload_of(&m).unwrap();
    let task = tokio::spawn({
        let b = Arc::clone(&b);
        async move { b.send_media(PEER, m).await }
    });
    while next_of(&mut ev, id).await.done < 200 {}
    b.cancel_transfer(id).await.unwrap();
    assert_eq!(task.await.unwrap(), Err(BackendError::Cancelled));
    assert_eq!(until_end(&mut ev, id).await.last().unwrap().state, TransferState::Cancelled);
    let hist = b.history(PEER, Anchor::Latest, 5).await.unwrap();
    assert!(hist.iter().all(|m| m.random_id != Some(RandomId(700))));
    assert_eq!(b.active_transfers(), 0);
}

#[tokio::test]
async fn subida_fallida_devuelve_el_error_y_lo_anuncia() {
    let b = mock();
    b.set_transfer_sim(TransferSim { fail_at: Some((0, BackendError::FloodWait(3))), ..sim(100, 0) });
    let mut ev = b.events();
    let m = media(vec![item("a.pdf", 300)], 800);
    let id = TransferId::upload_of(&m).unwrap();
    assert_eq!(b.send_media(PEER, m).await, Err(BackendError::FloodWait(3)));
    let list = until_end(&mut ev, id).await;
    assert_eq!(list.last().unwrap().state, TransferState::Failed(BackendError::FloodWait(3)));
    // Un archivo vacío también puede fallar.
    let m = media(vec![item("vacio.txt", 0)], 810);
    assert_eq!(b.send_media(PEER, m).await, Err(BackendError::FloodWait(3)));
}
