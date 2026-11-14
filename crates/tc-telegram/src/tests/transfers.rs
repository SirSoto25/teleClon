//! Transferencias con progreso y cancelación (ADR-0011) contra el transporte falso: descarga a disco
//! por partes (`.part` + renombrado), progreso monótono y limitado a ≤ 10 Hz, cancelación a mitad,
//! reconexión, `FILE_REFERENCE_*` y `FILE_MIGRATE` sin volver a empezar, subida por partes con
//! `TransferId::upload_of`, archivo enorme rechazado y nombre hostil saneado.
//!
//! Las pruebas con `start_paused` usan el reloj virtual de tokio: el transporte `Delayed` «tarda»
//! `delay` en cada parte y el tiempo avanza solo cuando todo está parado, así que es exacto.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures::FutureExt;
use futures::stream::{BoxStream, StreamExt};
use grammers_client::InvocationError;
use grammers_tl_types as tl;
use tc_core::*;
use tokio::time::Instant;

use super::fixtures::*;
use super::*;
use crate::media::{BIG_FILE, MAX_FILE, PART_SIZE, PROGRESS_INTERVAL};

const ANA: PeerRef = PeerRef::User(UserId(10));
const DOC: i64 = 900;

/// Directorio temporal propio que se borra al terminar.
struct TmpDir(PathBuf);
impl TmpDir {
    fn new(tag: &str) -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("tc-telegram-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }
}
impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Transporte que tarda `delay` (reloj de tokio) en cada `upload.*` y luego responde con `Fake`.
struct Delayed {
    fake: Arc<Fake>,
    delay: Duration,
}

#[async_trait]
impl Transport for Delayed {
    async fn invoke_raw(&self, dc_id: i32, body: Vec<u8>) -> Result<Vec<u8>, InvocationError> {
        let id = u32::from_le_bytes(body[..4].try_into().unwrap());
        let slow = [cid::<tl::functions::upload::GetFile>(), cid::<tl::functions::upload::SaveFilePart>(), cid::<tl::functions::upload::SaveBigFilePart>()];
        if !self.delay.is_zero() && slow.contains(&id) {
            tokio::time::sleep(self.delay).await;
        }
        self.fake.invoke_raw(dc_id, body).await
    }
}

/// Backend con sesión iniciada sobre `Delayed` (Ana ya conocida).
async fn setup(delay_ms: u64, mut h: impl FnMut(i32, u32, &[u8]) -> Reply + Send + 'static) -> (GrammersBackend, Arc<Fake>) {
    let fake = Fake::new(move |dc, id, body| {
        if id == cid::<tl::functions::users::GetUsers>() {
            return ok(vec![user_enum(ME_ID, "Yo", 1, true)]);
        }
        h(dc, id, body)
    });
    let t = Arc::new(Delayed { fake: fake.clone(), delay: Duration::from_millis(delay_ms) });
    let b = GrammersBackend::with_transport(cfg(), Arc::new(MemorySessionStore::default()), logged_session(), t, None);
    wait_auth(&b, |s| matches!(s, AuthState::LoggedIn(_))).await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    (b, fake)
}

fn content(len: usize) -> Arc<Vec<u8>> {
    Arc::new((0..len).map(|i| (i % 251) as u8).collect())
}

fn doc_ref(reference: u8, dc: i32, size: usize) -> FileRef {
    FileRef { kind: FileKind::Document, id: DOC, access_hash: 3, file_reference: vec![reference], dc_id: dc, size_bytes: size as u64, thumb: None, full: None }
}

/// Responde una parte de `data` según el desplazamiento pedido (comprobando las reglas de la API).
fn serve(data: &[u8], req: &tl::functions::upload::GetFile) -> Reply {
    assert_eq!(req.limit, PART_SIZE as i32);
    assert_eq!(req.offset % PART_SIZE as i64, 0, "desplazamiento alineado");
    let from = (req.offset as usize).min(data.len());
    let to = (from + PART_SIZE).min(data.len());
    ok(tl::enums::upload::File::from(tl::types::upload::File { r#type: tl::enums::storage::FileType::FileUnknown, mtime: 0, bytes: data[from..to].to_vec() }))
}

fn doc_location(req: &tl::functions::upload::GetFile) -> tl::types::InputDocumentFileLocation {
    let tl::enums::InputFileLocation::InputDocumentFileLocation(l) = req.location.clone() else { panic!("ubicación de documento") };
    assert_eq!((l.id, l.access_hash, l.thumb_size.as_str()), (DOC, 3, ""));
    l
}

/// Eventos hasta el estado terminal de `id` (incluido), con la hora (virtual) de llegada.
async fn until_finished(ev: &mut BoxStream<'static, BackendEvent>, id: TransferId) -> Vec<(Instant, BackendEvent)> {
    tokio::time::timeout(Duration::from_secs(3600), async {
        let mut out = vec![];
        while let Some(e) = ev.next().await {
            let end = matches!(&e, BackendEvent::TransferProgress(t) if t.id == id && t.state.is_finished());
            out.push((Instant::now(), e));
            if end {
                return out;
            }
        }
        panic!("se cerró el canal de eventos");
    })
    .await
    .expect("la transferencia termina")
}

fn transfers(evs: &[(Instant, BackendEvent)], id: TransferId) -> Vec<(Instant, Transfer)> {
    evs.iter()
        .filter_map(|(at, e)| match e {
            BackendEvent::TransferProgress(t) if t.id == id => Some((*at, t.clone())),
            _ => None,
        })
        .collect()
}

/// Eventos ya disponibles sin esperar.
fn ready(ev: &mut BoxStream<'static, BackendEvent>) -> Vec<BackendEvent> {
    let mut out = vec![];
    while let Some(Some(e)) = ev.next().now_or_never() {
        out.push(e);
    }
    out
}

fn no_transfer_events(ev: &mut BoxStream<'static, BackendEvent>) {
    let evs = ready(ev);
    assert!(!evs.iter().any(|e| matches!(e, BackendEvent::TransferProgress(_))), "sin eventos de transferencia: {evs:?}");
}

/// `done` nunca decrece, ≤ total, un único estado terminal al final.
fn assert_well_formed(ts: &[(Instant, Transfer)]) {
    assert!(ts.len() >= 2);
    assert_eq!((ts[0].1.done, &ts[0].1.state), (0, &TransferState::Running), "empieza en 0");
    for w in ts.windows(2) {
        assert!(w[1].1.done >= w[0].1.done, "progreso monótono: {} → {}", w[0].1.done, w[1].1.done);
    }
    let (last, running) = ts.split_last().unwrap();
    assert!(last.1.state.is_finished());
    assert!(running.iter().all(|t| t.1.state == TransferState::Running && (t.1.total == 0 || t.1.done <= t.1.total)));
}

// ------------------------------------------------------------------ descargas a disco

#[tokio::test(start_paused = true)]
async fn download_to_disk_reports_monotonic_throttled_progress_and_renames() {
    let data = content(20 * PART_SIZE + 1000);
    let d = data.clone();
    let (b, fake) = setup(30, move |_, id, body| {
        assert_eq!(id, cid::<tl::functions::upload::GetFile>());
        let req: tl::functions::upload::GetFile = decode(body);
        doc_location(&req);
        serve(&d, &req)
    })
    .await;
    let dir = TmpDir::new("dl");
    // Ya existe: se reemplaza al terminar.
    let dest = dir.file("informe.pdf", b"viejo");
    let mut ev = b.events();
    let start = Instant::now();
    let id = b.download_media_to(ANA, MsgId(7), doc_ref(1, 2, data.len()), MediaVariant::Full, dest.clone()).await.unwrap();
    assert!(matches!(id, TransferId::Download(_)));
    let ts = transfers(&until_finished(&mut ev, id).await, id);
    assert_well_formed(&ts);
    let t = &ts.last().unwrap().1;
    assert_eq!((&t.state, t.done, t.total), (&TransferState::Done, data.len() as u64, data.len() as u64));
    assert_eq!((t.kind, t.peer, t.msg, t.name.as_str()), (TransferKind::Download, ANA, Some(MsgId(7)), "informe.pdf"));
    // El archivo ya está en su sitio cuando llega `Done`, sin parcial.
    assert_eq!(std::fs::read(&dest).unwrap(), *data);
    assert!(!partial_path(&dest).exists());
    // ≤ 10 Hz: entre dos `Running` hay al menos 100 ms (el primero, con 0, sale al momento).
    let running: Vec<_> = ts.iter().filter(|t| t.1.state == TransferState::Running).collect();
    for w in running.windows(2) {
        assert!(w[1].0 - w[0].0 >= PROGRESS_INTERVAL, "eventos demasiado seguidos");
    }
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(21 * 30));
    let max = 2 + (elapsed.as_millis() / PROGRESS_INTERVAL.as_millis()) as usize;
    assert!((3..=max).contains(&running.len()), "{} eventos Running en {elapsed:?}", running.len());
    assert_eq!(fake.count("upload.getFile"), 21);
}

#[tokio::test(start_paused = true)]
async fn download_cancelled_midway_removes_partial_and_keeps_dest() {
    let data = content(40 * PART_SIZE);
    let d = data.clone();
    let (b, fake) = setup(30, move |_, _, body| serve(&d, &decode(body))).await;
    let dir = TmpDir::new("cancel");
    let dest = dir.file("video.mp4", b"viejo");
    let mut ev = b.events();
    let id = b.download_media_to(ANA, MsgId(7), doc_ref(1, 2, data.len()), MediaVariant::Full, dest.clone()).await.unwrap();
    // Espera a que vaya por la tercera parte y cancela.
    let mut seen = vec![];
    loop {
        match ev.next().await.unwrap() {
            BackendEvent::TransferProgress(t) if t.id == id => {
                assert_eq!(t.state, TransferState::Running);
                seen.push((Instant::now(), t.clone()));
                if t.done >= 3 * PART_SIZE as u64 {
                    break;
                }
            }
            _ => {}
        }
    }
    assert!(partial_path(&dest).exists(), "escribe en el parcial mientras tanto");
    b.cancel_transfer(id).await.unwrap();
    seen.extend(transfers(&until_finished(&mut ev, id).await, id));
    assert_well_formed(&seen);
    assert_eq!(seen.last().unwrap().1.state, TransferState::Cancelled);
    assert_eq!(std::fs::read(&dest).unwrap(), b"viejo", "el destino queda como estaba");
    assert!(!partial_path(&dest).exists(), "sin parcial tras cancelar");
    assert!(fake.count("upload.getFile") < 10);
    // Idempotente: otra vez, una descarga desconocida o una subida que no existe.
    b.cancel_transfer(id).await.unwrap();
    b.cancel_transfer(TransferId::Download(999)).await.unwrap();
    b.cancel_transfer(TransferId::Upload(RandomId(5))).await.unwrap();
    // Nada más de esa transferencia.
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!ready(&mut ev).iter().any(|e| matches!(e, BackendEvent::TransferProgress(t) if t.id == id)));
}

#[tokio::test(start_paused = true)]
async fn download_survives_reconnection_file_reference_and_file_migrate_without_restarting() {
    let data = content(4 * PART_SIZE + 77);
    let d = data.clone();
    let drops = Arc::new(AtomicUsize::new(0));
    let dr = drops.clone();
    let offsets = Arc::new(std::sync::Mutex::new(vec![]));
    let offs = offsets.clone();
    let (b, fake) = setup(0, move |dc, id, body| match id {
        x if x == cid::<tl::functions::upload::GetFile>() => {
            let req: tl::functions::upload::GetFile = decode(body);
            let loc = doc_location(&req);
            if dc != 4 {
                return rpc_err(303, "FILE_MIGRATE", Some(4));
            }
            offs.lock().unwrap().push((req.offset, loc.file_reference.clone()));
            // Se corta la conexión dos veces en la segunda parte.
            if req.offset == PART_SIZE as i64 && dr.fetch_add(1, Ordering::SeqCst) < 2 {
                return Err(InvocationError::Dropped("conexión perdida"));
            }
            // La referencia caduca a mitad de la descarga.
            if req.offset >= 2 * PART_SIZE as i64 && loc.file_reference == [1] {
                return rpc_err(400, "FILE_REFERENCE_EXPIRED", None);
            }
            serve(&d, &req)
        }
        x if x == cid::<tl::functions::messages::GetMessages>() => {
            let mut m = message(7, peer_user(10), "", false, None, 100);
            m.media = Some(doc_media(2, 2, d.len()));
            ok(tl::enums::messages::Messages::from(tl::types::messages::Messages { messages: vec![m.into()], topics: vec![], chats: vec![], users: vec![] }))
        }
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    })
    .await;
    let dir = TmpDir::new("resume");
    let dest = dir.0.join("datos.bin");
    let mut ev = b.events();
    let id = b.download_media_to(ANA, MsgId(7), doc_ref(1, 2, data.len()), MediaVariant::Full, dest.clone()).await.unwrap();
    let evs = until_finished(&mut ev, id).await;
    let ts = transfers(&evs, id);
    assert_well_formed(&ts);
    assert_eq!(ts.last().unwrap().1.state, TransferState::Done);
    assert_eq!(std::fs::read(&dest).unwrap(), *data);
    assert_eq!(fake.count("messages.getMessages"), 1);
    // Sigue desde donde iba: el desplazamiento 0 se pidió una sola vez y la parte caducada se repite
    // ya con la referencia nueva.
    let p = PART_SIZE as i64;
    let got = offsets.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![(0, vec![1]), (p, vec![1]), (p, vec![1]), (p, vec![1]), (2 * p, vec![1]), (2 * p, vec![2]), (3 * p, vec![2]), (4 * p, vec![2])]
    );
    // La reconexión se anunció.
    assert!(evs.iter().any(|(_, e)| matches!(e, BackendEvent::Connection(ConnectionState::Connecting))));
}

fn doc_media(reference: u8, dc: i32, size: usize) -> tl::enums::MessageMedia {
    let doc = tl::types::Document {
        id: DOC,
        access_hash: 3,
        file_reference: vec![reference],
        date: 0,
        mime_type: "application/octet-stream".into(),
        size: size as i64,
        thumbs: None,
        video_thumbs: None,
        dc_id: dc,
        attributes: vec![tl::types::DocumentAttributeFilename { file_name: "datos.bin".into() }.into()],
    };
    tl::types::MessageMediaDocument {
        nopremium: false,
        spoiler: false,
        video: false,
        round: false,
        voice: false,
        document: Some(doc.into()),
        alt_documents: None,
        video_cover: None,
        video_timestamp: None,
        ttl_seconds: None,
    }
    .into()
}

#[tokio::test(start_paused = true)]
async fn long_outage_fails_the_download_and_cleans_the_partial() {
    let data = content(3 * PART_SIZE);
    let d = data.clone();
    let (b, _) = setup(0, move |_, _, body| {
        let req: tl::functions::upload::GetFile = decode(body);
        if req.offset > 0 { Err(InvocationError::Dropped("sin red")) } else { serve(&d, &req) }
    })
    .await;
    let dir = TmpDir::new("outage");
    let dest = dir.0.join("datos.bin");
    let mut ev = b.events();
    let id = b.download_media_to(ANA, MsgId(7), doc_ref(1, 2, data.len()), MediaVariant::Full, dest.clone()).await.unwrap();
    let ts = transfers(&until_finished(&mut ev, id).await, id);
    assert_well_formed(&ts);
    assert_eq!(ts.last().unwrap().1.state, TransferState::Failed(BackendError::Network));
    assert!(!dest.exists() && !partial_path(&dest).exists());
}

#[tokio::test]
async fn download_validation_fails_fast_without_events() {
    let (b, fake) = setup(0, |_, id, _| panic!("llamada inesperada {}", tl::name_for_id(id))).await;
    let dir = TmpDir::new("val");
    let mut ev = b.events();
    let f = doc_ref(1, 2, 10);
    let get = |dest: PathBuf, variant| b.download_media_to(ANA, MsgId(7), f.clone(), variant, dest);
    assert_eq!(get(dir.0.join("no-existe").join("a.pdf"), MediaVariant::Full).await, Err(BackendError::InvalidInput("dest")));
    assert_eq!(get(dir.0.clone(), MediaVariant::Full).await, Err(BackendError::InvalidInput("dest")));
    assert_eq!(get(dir.0.join(".."), MediaVariant::Full).await, Err(BackendError::InvalidInput("dest")));
    // Un documento sin miniatura no tiene variante `Thumb`.
    assert_eq!(get(dir.0.join("a.jpg"), MediaVariant::Thumb).await, Err(BackendError::NotFound));
    let secret = b.download_media_to(PeerRef::Secret(SecretChatId(1)), MsgId(7), f.clone(), MediaVariant::Full, dir.0.join("a.pdf")).await;
    assert!(matches!(secret, Err(BackendError::Forbidden(_))));
    b.inner.net.conn.set(ConnectionState::Offline);
    let _ = ready(&mut ev); // el propio evento de conexión
    assert_eq!(get(dir.0.join("a.pdf"), MediaVariant::Full).await, Err(BackendError::Network));
    no_transfer_events(&mut ev);
    assert_eq!(fake.count("upload.getFile"), 0);
    assert!(std::fs::read_dir(&dir.0).unwrap().next().is_none(), "no crea nada en el directorio");
}

// ------------------------------------------------------------------ subidas

fn file_item(path: &Path, name: &str) -> OutgoingMediaItem {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    OutgoingMediaItem { kind: MediaKind::File, path: path.to_string_lossy().into_owned(), file_name: name.into(), size_bytes: size, width: 0, height: 0 }
}

fn media(items: Vec<OutgoingMediaItem>, rid: i64) -> OutgoingMedia {
    let random_ids = (0..items.len() as i64).map(|i| RandomId(rid + i)).collect();
    OutgoingMedia { items, caption: "pie".into(), reply_to: None, random_ids }
}

fn sent_short(id: i32) -> Reply {
    ok(tl::enums::Updates::from(tl::types::UpdateShortSentMessage { out: true, id, pts: 1, pts_count: 1, date: 1234, media: None, entities: None, ttl_period: None }))
}

/// Reconstruye los archivos subidos: `file_id` → partes.
type Parts = Arc<std::sync::Mutex<std::collections::BTreeMap<(i64, i32), Vec<u8>>>>;

/// Manejador de subidas: guarda las partes y confirma `sendMedia` con ids 500, 501…
fn uploader(parts: Parts, sends: Arc<std::sync::Mutex<Vec<tl::functions::messages::SendMedia>>>) -> impl FnMut(i32, u32, &[u8]) -> Reply + Send + 'static {
    move |_, id, body| match id {
        x if x == cid::<tl::functions::upload::SaveFilePart>() => {
            let r: tl::functions::upload::SaveFilePart = decode(body);
            parts.lock().unwrap().insert((r.file_id, r.file_part), r.bytes);
            ok(true)
        }
        x if x == cid::<tl::functions::upload::SaveBigFilePart>() => {
            let r: tl::functions::upload::SaveBigFilePart = decode(body);
            assert!(r.file_part < r.file_total_parts);
            parts.lock().unwrap().insert((r.file_id, r.file_part), r.bytes);
            ok(true)
        }
        x if x == cid::<tl::functions::messages::SendMedia>() => {
            let r: tl::functions::messages::SendMedia = decode(body);
            let mut s = sends.lock().unwrap();
            s.push(r);
            sent_short(500 + s.len() as i32 - 1)
        }
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    }
}

fn uploaded(parts: &Parts, file_id: i64) -> Vec<u8> {
    parts.lock().unwrap().iter().filter(|((f, _), _)| *f == file_id).flat_map(|(_, b)| b.clone()).collect()
}

/// `(id, partes, nombre, ¿grande?)` del archivo subido de un `sendMedia`.
fn input_file(r: &tl::functions::messages::SendMedia) -> (i64, i32, String, bool, String) {
    let tl::enums::InputMedia::UploadedDocument(d) = &r.media else { panic!("documento subido") };
    let attr = d
        .attributes
        .iter()
        .find_map(|a| match a {
            tl::enums::DocumentAttribute::Filename(f) => Some(f.file_name.clone()),
            _ => None,
        })
        .unwrap();
    match &d.file {
        tl::enums::InputFile::File(f) => (f.id, f.parts, f.name.clone(), false, attr),
        tl::enums::InputFile::Big(f) => (f.id, f.parts, f.name.clone(), true, attr),
        other => panic!("archivo inesperado {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn upload_reports_progress_and_finishes_after_message_sent() {
    let small = content(3 * PART_SIZE + 10);
    let big = content(BIG_FILE as usize + 1);
    let dir = TmpDir::new("up");
    let (p1, p2) = (dir.file("a.bin", &small), dir.file("b.bin", &big));
    let parts: Parts = Arc::default();
    let sends = Arc::new(std::sync::Mutex::new(vec![]));
    let (b, _) = setup(30, uploader(parts.clone(), sends.clone())).await;
    let mut ev = b.events();
    let m = media(vec![file_item(&p1, "a.bin"), file_item(&p2, "b.bin")], 41);
    let id = TransferId::upload_of(&m).unwrap();
    let total = (small.len() + big.len()) as u64;
    let out = b.send_media(ANA, m).await.unwrap();
    assert_eq!(out.iter().map(|m| (m.id, m.random_id)).collect::<Vec<_>>(), vec![(MsgId(500), Some(RandomId(41))), (MsgId(501), Some(RandomId(42)))]);

    let evs = until_finished(&mut ev, id).await;
    let ts = transfers(&evs, id);
    assert_well_formed(&ts);
    let last = &ts.last().unwrap().1;
    assert_eq!((&last.state, last.done, last.total, last.kind, last.msg, last.name.as_str()), (&TransferState::Done, total, total, TransferKind::Upload, None, "a.bin"));
    assert!(ts.iter().all(|t| t.1.total == total && t.1.peer == ANA));
    assert!(ts.len() > 3, "hay progreso intermedio");
    // `Done` llega después de los dos `MessageSent`.
    let pos = |pred: &dyn Fn(&BackendEvent) -> bool| evs.iter().position(|(_, e)| pred(e)).unwrap();
    let done_at = pos(&|e| matches!(e, BackendEvent::TransferProgress(t) if t.state == TransferState::Done));
    assert!(pos(&|e| matches!(e, BackendEvent::MessageSent { random_id: RandomId(42), .. })) < done_at);
    assert!(pos(&|e| matches!(e, BackendEvent::MessageSent { random_id: RandomId(41), .. })) < done_at);

    // Lo subido es exactamente cada archivo, el pequeño con `saveFilePart` y el grande con `saveBigFilePart`.
    let s = sends.lock().unwrap();
    let (f1, n1, name1, big1, _) = input_file(&s[0]);
    let (f2, n2, name2, big2, _) = input_file(&s[1]);
    assert_eq!((n1, name1.as_str(), big1, s[0].message.as_str()), (4, "a.bin", false, "pie"));
    assert_eq!((n2, name2.as_str(), big2, s[1].message.as_str()), (21, "b.bin", true, ""));
    assert_eq!(uploaded(&parts, f1), *small);
    assert_eq!(uploaded(&parts, f2), *big);
}

#[tokio::test(start_paused = true)]
async fn upload_cancelled_midway_keeps_what_was_already_sent() {
    let data = content(3 * PART_SIZE);
    let dir = TmpDir::new("upcancel");
    let (p1, p2) = (dir.file("a.bin", &data), dir.file("b.bin", &data));
    let parts: Parts = Arc::default();
    let sends = Arc::new(std::sync::Mutex::new(vec![]));
    let (b, fake) = setup(30, uploader(parts, sends.clone())).await;
    let mut ev = b.events();
    let m = media(vec![file_item(&p1, "a.bin"), file_item(&p2, "b.bin")], 51);
    let id = TransferId::upload_of(&m).unwrap();
    let mut evs = vec![];
    let (r, ()) = tokio::join!(b.send_media(ANA, m), async {
        // Cancela en cuanto se confirma el primer elemento.
        loop {
            let e = ev.next().await.unwrap();
            let first = matches!(e, BackendEvent::MessageSent { random_id: RandomId(51), .. });
            evs.push((Instant::now(), e));
            if first {
                b.cancel_transfer(id).await.unwrap();
                return;
            }
        }
    });
    assert_eq!(r, Err(BackendError::Cancelled));
    evs.extend(until_finished(&mut ev, id).await);
    let ts = transfers(&evs, id);
    assert_well_formed(&ts);
    assert_eq!(ts.last().unwrap().1.state, TransferState::Cancelled);
    // El primero quedó enviado; el segundo ni se terminó de subir ni se envió.
    assert_eq!(sends.lock().unwrap().len(), 1);
    assert!(fake.count("upload.saveFilePart") < 6);
    // Tras cancelar, cancelar otra vez no es un error.
    b.cancel_transfer(id).await.unwrap();
}

#[tokio::test]
async fn huge_or_invalid_uploads_are_rejected_without_events() {
    let (b, fake) = setup(0, |_, id, _| panic!("llamada inesperada {}", tl::name_for_id(id))).await;
    let dir = TmpDir::new("upval");
    // Archivo disperso de más del máximo (no ocupa disco).
    let huge = dir.0.join("enorme.iso");
    std::fs::File::create(&huge).unwrap().set_len(MAX_FILE + 1).unwrap();
    let ok_file = dir.file("ok.bin", b"hola");
    let empty = dir.file("vacio.bin", b"");
    let mut ev = b.events();

    let e = b.send_media(ANA, media(vec![file_item(&huge, "enorme.iso")], 1)).await.unwrap_err();
    let BackendError::Forbidden(msg) = e else { panic!("aviso de tamaño: {e:?}") };
    assert!(msg.contains("enorme.iso") && msg.contains("2000 MiB") && msg.contains("Premium"), "{msg}");
    // También si es el segundo de un álbum: no se sube nada.
    let e = b.send_media(ANA, media(vec![file_item(&ok_file, "ok.bin"), file_item(&huge, "enorme.iso")], 2)).await;
    assert!(matches!(e, Err(BackendError::Forbidden(_))));

    let mut no_ids = media(vec![file_item(&ok_file, "ok.bin")], 3);
    no_ids.random_ids.clear();
    assert_eq!(b.send_media(ANA, no_ids).await, Err(BackendError::InvalidInput("items")));
    assert_eq!(b.send_media(ANA, media(vec![], 4)).await, Err(BackendError::InvalidInput("items")));
    assert_eq!(b.send_media(ANA, media(vec![file_item(&dir.0.join("no.bin"), "no.bin")], 5)).await, Err(BackendError::InvalidInput("media")));
    assert_eq!(b.send_media(ANA, media(vec![file_item(&empty, "vacio.bin")], 6)).await, Err(BackendError::InvalidInput("media")));
    assert_eq!(b.send_media(PeerRef::User(UserId(404)), media(vec![file_item(&ok_file, "ok.bin")], 7)).await, Err(BackendError::NotFound));
    no_transfer_events(&mut ev);
    assert!(fake.names().iter().all(|n| !n.starts_with("upload.") && *n != "messages.sendMedia"));
}

#[tokio::test]
async fn hostile_file_name_is_sanitized_before_sending() {
    let dir = TmpDir::new("hostile");
    let p = dir.file("x.bin", b"contenido");
    let parts: Parts = Arc::default();
    let sends = Arc::new(std::sync::Mutex::new(vec![]));
    let (b, _) = setup(0, uploader(parts.clone(), sends.clone())).await;
    let mut ev = b.events();
    let hostile = "../../etc/\u{202E}fdp.exe\0..";
    let clean = sanitize_file_name(hostile);
    assert!(!clean.contains(['/', '\\', '\0', '\u{202E}']) && !clean.starts_with('.'));
    let m = media(vec![file_item(&p, hostile)], 61);
    let id = TransferId::upload_of(&m).unwrap();
    b.send_media(ANA, m).await.unwrap();
    let (fid, _, name, _, attr) = input_file(&sends.lock().unwrap()[0]);
    assert_eq!((name.as_str(), attr.as_str()), (clean.as_str(), clean.as_str()));
    assert_eq!(uploaded(&parts, fid), b"contenido");
    let ts = transfers(&until_finished(&mut ev, id).await, id);
    assert!(ts.iter().all(|t| t.1.name == clean));
}
