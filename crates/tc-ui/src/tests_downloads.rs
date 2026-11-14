//! Descarga de archivos contra el mock (fase A3): clic → descarga con progreso, cancelar a mitad,
//! fallo y «Reintentar», nombre hostil saneado, destino repetido (« (1)»), «Guardar como…», recuerdo
//! por sesión, temas de foro y menús. Como en el resto de pruebas, las `Task` no se ejecutan: la
//! prueba hace la llamada que haría la tarea (con el destino que eligió la UI) y entrega el
//! resultado y los eventos como lo haría iced. Nunca se lanza `xdg-open` (`Downloads::last_open`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use tc_core::*;
use tc_mock::{BIG_CHAT, FORUM_CHAT, FORUM_GROUP, MockBackend, MockConfig, TransferSim};
use tc_theme::ThemeKind;

use crate::app::downloads::{DlMsg, DlState, download_name, dest_in, open_command};
use crate::app::forum::ForumMsg;
use crate::app::{App, Backend, Flags, Msg};
use crate::conv::PAGE;
use crate::view::context_menu::message_items;

const NOW: i64 = 1_762_430_400;

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("parlo-dl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

async fn boot() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags { theme: ThemeKind::Dark, ..Flags::new(backend.clone()) });
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let _ = app.update(Msg::GroupLoaded(backend.group(FORUM_GROUP).await));
    let _ = app.update(Msg::Open(BIG_CHAT));
    let r = backend.history(BIG_CHAT, Anchor::Latest, PAGE).await;
    let _ = app.update(Msg::History { peer: BIG_CHAT, older: false, result: r });
    (app, mock)
}

fn pump(app: &mut App, ev: &mut futures::stream::BoxStream<'static, BackendEvent>) {
    while let Some(Some(e)) = ev.next().now_or_never() {
        let _ = app.update(Msg::Backend(e));
    }
}

/// Primer mensaje del chat abierto con un medio de ese tipo.
fn find_media(app: &App, kind: MediaKind) -> Message {
    app.conv.as_ref().unwrap().messages.iter().find(|m| m.media.as_ref().is_some_and(|md| md.kind == kind && md.file.is_some())).cloned().expect("hay un mensaje así en los datos de muestra")
}

fn state(app: &App, m: &Message) -> Option<DlState> {
    app.dl.get(m.peer, m.id).map(|d| d.state.clone())
}

/// Lo que haría la tarea de `Start`/`SavePicked`: llama al backend con el destino que anotó la UI y
/// entrega la respuesta. Devuelve el id de la transferencia.
async fn run_task(app: &mut App, m: &Message) -> BackendResult<TransferId> {
    let d = app.dl.get(m.peer, m.id).expect("la UI anotó la descarga");
    let (path, file) = (d.path.clone(), m.media.as_ref().unwrap().file.clone().unwrap());
    let b = app.scoped_backend().unwrap();
    let r = b.download_media_to(m.peer, m.id, file, MediaVariant::Full, path).await;
    let _ = app.update(Msg::Dl(DlMsg::Started { peer: m.peer, msg: m.id, result: r.clone() }));
    r
}

fn hostile(name: &str) -> Media {
    Media { id: 1, kind: MediaKind::File, file_name: Some(name.into()), size_bytes: 10, width: 0, height: 0, duration_secs: None, local_path: None, file: None, audio: None }
}

#[test]
fn el_nombre_hostil_se_sanea_y_nunca_sale_de_la_carpeta() {
    for raw in ["../../etc/passwd", "..\\..\\Windows\\system32\\evil.dll", "/etc/cron.d/x", "CON.txt", "informe\u{202e}fdp.exe", "   ", "a\0b.txt", ".."] {
        let n = download_name(&hostile(raw));
        let p = Path::new(&n);
        assert_eq!(p.components().count(), 1, "{raw:?} → {n:?}");
        assert!(matches!(p.components().next(), Some(std::path::Component::Normal(_))), "{raw:?} → {n:?}");
        assert!(!n.contains(['/', '\\', '\0', '\u{202e}']) && !n.starts_with('.'), "{raw:?} → {n:?}");
        let dest = dest_in(Path::new("/tmp/destino"), &hostile(raw));
        assert_eq!(dest.parent(), Some(Path::new("/tmp/destino")), "{raw:?}");
    }
    // Sin nombre: uno por tipo.
    let mut md = hostile("x");
    md.file_name = None;
    assert_eq!(download_name(&md), "archivo");
    md.kind = MediaKind::Photo;
    assert_eq!(download_name(&md), "foto.jpg");
}

#[test]
fn nombre_repetido_recibe_sufijo() {
    let dir = tmp("sufijo");
    std::fs::write(dir.join("informe.pdf"), b"x").unwrap();
    std::fs::write(dir.join("informe (1).pdf"), b"x").unwrap();
    assert_eq!(dest_in(&dir, &hostile("informe.pdf")), dir.join("informe (2).pdf"));
    assert_eq!(dest_in(&dir, &hostile("otro.pdf")), dir.join("otro.pdf"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn abrir_es_un_solo_argumento_sin_shell() {
    let path = Path::new("/home/ana/Descargas/a b; rm -rf ~ $(x).pdf");
    let (prog, args) = open_command(path, false);
    assert!(["xdg-open", "open", "explorer"].contains(&prog));
    assert_eq!(args.len(), 1);
    assert_eq!(args[0], path.as_os_str());
    let (_, args) = open_command(path, true);
    assert_eq!(args.len(), 1);
    if cfg!(target_os = "linux") {
        assert_eq!(args[0], Path::new("/home/ana/Descargas").as_os_str(), "Linux abre la carpeta");
    }
}

#[tokio::test(start_paused = true)]
async fn clic_descarga_con_progreso_y_termina_en_abrir() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    let dir = tmp("completa");
    app.dl.dir = Some(dir.clone());
    mock.set_transfer_sim(TransferSim { chunk: 8_000, delay: Duration::from_millis(100), fail_at: None });
    let m = find_media(&app, MediaKind::File);
    let name = m.media.as_ref().unwrap().file_name.clone().unwrap();

    assert_eq!(state(&app, &m), None, "antes del clic: tarjeta normal");
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    assert_eq!(state(&app, &m), Some(DlState::Starting));
    let dest = app.dl.get(m.peer, m.id).unwrap().path.clone();
    assert_eq!(dest, dir.join(&name));
    let id = run_task(&mut app, &m).await.expect("arranca");
    assert!(matches!(id, TransferId::Download(_)));

    tokio::time::sleep(Duration::from_millis(250)).await;
    pump(&mut app, &mut ev);
    let Some(DlState::Running { done, total }) = state(&app, &m) else { panic!("{:?}", state(&app, &m)) };
    assert!(done > 0 && done < total, "a medias: {done}/{total}");
    let d = app.dl.get(m.peer, m.id).unwrap();
    assert!(d.fraction().unwrap() > 0.0);
    assert!(crate::view::send_dialog::status_text(d).contains(" % · "));
    assert!(!dest.exists(), "mientras tanto solo hay parcial");

    tokio::time::sleep(Duration::from_secs(2)).await;
    pump(&mut app, &mut ev);
    assert_eq!(state(&app, &m), Some(DlState::Done));
    assert!(dest.exists() && !tc_core::partial_path(&dest).exists());

    // Otro clic: abre (no vuelve a descargar).
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    assert_eq!(app.dl.last_open, Some((dest.clone(), false)));
    let _ = app.update(Msg::Dl(DlMsg::Reveal(m.id)));
    assert_eq!(app.dl.last_open, Some((dest.clone(), true)));
    assert_eq!(state(&app, &m), Some(DlState::Done));

    // Se recuerda por sesión: cerrar y reabrir el chat no lo olvida.
    let _ = app.update(Msg::CloseChat);
    let _ = app.update(Msg::Open(BIG_CHAT));
    let r = app.scoped_backend().unwrap().history(BIG_CHAT, Anchor::Latest, PAGE).await;
    let _ = app.update(Msg::History { peer: BIG_CHAT, older: false, result: r });
    assert_eq!(state(&app, &m), Some(DlState::Done));

    // Si el archivo desaparece, la tarjeta vuelve a ofrecer la descarga.
    std::fs::remove_file(&dest).unwrap();
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    assert_eq!(state(&app, &m), Some(DlState::Starting), "se vuelve a bajar");
    assert!(app.toasts.is_empty() || !app.toasts.iter().any(|t| t.error));
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test(start_paused = true)]
async fn los_eventos_antes_del_id_encuentran_su_tarjeta() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    let dir = tmp("antes-id");
    app.dl.dir = Some(dir.clone());
    mock.set_transfer_sim(TransferSim { chunk: 8_000, delay: Duration::ZERO, fail_at: None });
    let m = find_media(&app, MediaKind::File);
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    // La transferencia corre y sus eventos llegan antes que la respuesta de `download_media_to`.
    let d = app.dl.get(m.peer, m.id).unwrap();
    let (path, file) = (d.path.clone(), m.media.as_ref().unwrap().file.clone().unwrap());
    let id = app.scoped_backend().unwrap().download_media_to(m.peer, m.id, file, MediaVariant::Full, path).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    pump(&mut app, &mut ev);
    assert_eq!(state(&app, &m), Some(DlState::Done), "el estado terminal llegó sin conocer el id");
    let _ = app.update(Msg::Dl(DlMsg::Started { peer: m.peer, msg: m.id, result: Ok(id) }));
    assert_eq!(app.dl.get(m.peer, m.id).unwrap().id, Some(id));
    assert_eq!(state(&app, &m), Some(DlState::Done));
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test(start_paused = true)]
async fn cancelar_a_mitad_deja_la_tarjeta_como_antes_y_sin_parcial() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    let dir = tmp("cancelar");
    app.dl.dir = Some(dir.clone());
    mock.set_transfer_sim(TransferSim { chunk: 4_000, delay: Duration::from_millis(100), fail_at: None });
    let m = find_media(&app, MediaKind::File);
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    run_task(&mut app, &m).await.unwrap();
    tokio::time::sleep(Duration::from_millis(350)).await;
    pump(&mut app, &mut ev);
    assert!(matches!(state(&app, &m), Some(DlState::Running { .. })));

    let _ = app.update(Msg::Dl(DlMsg::Cancel(m.id)));
    assert_eq!(state(&app, &m), Some(DlState::Cancelling));
    let id = app.dl.get(m.peer, m.id).unwrap().id.unwrap();
    app.scoped_backend().unwrap().cancel_transfer(id).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    pump(&mut app, &mut ev);
    assert_eq!(state(&app, &m), None, "cancelada: la tarjeta vuelve a su estado inicial");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0, "ni archivo ni .part");
    assert!(!app.toasts.iter().any(|t| t.error), "cancelar no es un fallo");
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test(start_paused = true)]
async fn cancelar_antes_de_conocer_el_id_se_cancela_al_llegar() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    let dir = tmp("cancelar-pronto");
    app.dl.dir = Some(dir.clone());
    mock.set_transfer_sim(TransferSim { chunk: 4_000, delay: Duration::from_millis(100), fail_at: None });
    let m = find_media(&app, MediaKind::File);
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    let _ = app.update(Msg::Dl(DlMsg::Cancel(m.id)));
    assert_eq!(state(&app, &m), Some(DlState::Cancelling));
    let id = run_task(&mut app, &m).await.unwrap();
    // La UI pidió cancelar al recibir el id (la tarea `cancel_transfer` la hace iced).
    mock.cancel_transfer(id).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    pump(&mut app, &mut ev);
    assert_eq!(state(&app, &m), None);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test(start_paused = true)]
async fn fallo_muestra_el_error_y_reintentar_termina() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    let dir = tmp("fallo");
    app.dl.dir = Some(dir.clone());
    mock.set_transfer_sim(TransferSim { chunk: 4_000, delay: Duration::from_millis(10), fail_at: Some((8_000, BackendError::Network)) });
    let m = find_media(&app, MediaKind::File);
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    run_task(&mut app, &m).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    pump(&mut app, &mut ev);
    let Some(DlState::Failed(text)) = state(&app, &m) else { panic!("{:?}", state(&app, &m)) };
    assert!(text.contains("Sin conexión"), "{text}");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0, "sin parcial");

    // «Reintentar» = otro clic; esta vez la red va bien.
    mock.set_transfer_sim(TransferSim::default());
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    assert_eq!(state(&app, &m), Some(DlState::Starting));
    run_task(&mut app, &m).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    pump(&mut app, &mut ev);
    assert_eq!(state(&app, &m), Some(DlState::Done));
    let dest = app.dl.get(m.peer, m.id).unwrap().path.clone();
    assert!(dest.exists());
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test(start_paused = true)]
async fn el_fallo_inmediato_de_la_llamada_tambien_es_reintentable() {
    let (mut app, _mock) = boot().await;
    let m = find_media(&app, MediaKind::File);
    app.dl.dir = Some(tmp("inmediato"));
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    let _ = app.update(Msg::Dl(DlMsg::Started { peer: m.peer, msg: m.id, result: Err(BackendError::InvalidInput("dest")) }));
    let Some(DlState::Failed(t)) = state(&app, &m) else { panic!() };
    assert!(t.contains("carpeta de destino"));
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    assert_eq!(state(&app, &m), Some(DlState::Starting));
}

#[tokio::test(start_paused = true)]
async fn guardar_como_respeta_el_destino_elegido() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    let dir = tmp("guardar-como");
    mock.set_transfer_sim(TransferSim::default());
    let m = find_media(&app, MediaKind::File);
    let dest = dir.join("mi copia.bin");
    let _ = app.update(Msg::Dl(DlMsg::SavePicked { peer: m.peer, msg: m.id, dest: Some(dest.clone()) }));
    assert_eq!(app.dl.get(m.peer, m.id).unwrap().path, dest);
    run_task(&mut app, &m).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    pump(&mut app, &mut ev);
    assert_eq!(state(&app, &m), Some(DlState::Done));
    assert!(dest.exists());
    // Cerrar el selector sin elegir no toca nada.
    let n = find_media(&app, MediaKind::Photo);
    let _ = app.update(Msg::Dl(DlMsg::SavePicked { peer: n.peer, msg: n.id, dest: None }));
    assert_eq!(state(&app, &n), None);
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test(start_paused = true)]
async fn guardar_una_foto_avisa_al_terminar() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    let dir = tmp("foto");
    app.dl.dir = Some(dir.clone());
    mock.set_transfer_sim(TransferSim::default());
    let m = find_media(&app, MediaKind::Photo);
    // Visor abierto sobre esa foto: «Guardar».
    let _ = app.update(Msg::OpenPhoto { peer: m.peer, msg: m.id });
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    assert_eq!(app.dl.get(m.peer, m.id).unwrap().path, dir.join("foto.jpg"));
    run_task(&mut app, &m).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    pump(&mut app, &mut ev);
    assert_eq!(state(&app, &m), Some(DlState::Done));
    assert!(app.toasts.iter().any(|t| !t.error && t.text.starts_with("Guardado en ")), "{:?}", app.toasts.iter().map(|t| &t.text).collect::<Vec<_>>());
    std::fs::remove_dir_all(dir).ok();
}

fn labels(app: &App, id: MsgId) -> Vec<String> {
    message_items(app, id).iter().filter_map(|i| i.label().map(str::to_string)).collect()
}

#[tokio::test]
async fn los_menus_ofrecen_guardar_segun_el_medio() {
    let (app, _mock) = boot().await;
    let doc = labels(&app, find_media(&app, MediaKind::File).id);
    assert!(doc.contains(&"Guardar como…".to_string()) && !doc.contains(&"Guardar".to_string()), "{doc:?}");
    let photo = labels(&app, find_media(&app, MediaKind::Photo).id);
    assert!(photo.contains(&"Guardar".to_string()) && photo.contains(&"Guardar como…".to_string()), "{photo:?}");
    let text = app.conv.as_ref().unwrap().messages.iter().find(|m| m.media.is_none() && m.sticker.is_none() && !m.text.is_empty()).unwrap();
    let l = labels(&app, text.id);
    assert!(!l.iter().any(|x| x.starts_with("Guardar")), "{l:?}");
}

#[tokio::test(start_paused = true)]
async fn funciona_dentro_de_un_tema_del_foro() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    let dir = tmp("foro");
    app.dl.dir = Some(dir.clone());
    mock.set_transfer_sim(TransferSim::default());
    let _ = app.update(Msg::Open(FORUM_CHAT));
    let result = mock.forum_topics(FORUM_GROUP, TopicsQuery::first(crate::app::forum::TOPICS_PAGE)).await;
    let _ = app.update(Msg::Forum(ForumMsg::Loaded { group: FORUM_GROUP, append: false, query: None, result }));
    let t = app.forum.topics[&FORUM_GROUP].items.iter().find(|t| !t.closed).cloned().expect("hay un tema abierto");
    let th = ThreadRef { group: FORUM_GROUP, topic: t.id };
    let _ = app.update(Msg::Forum(ForumMsg::OpenTopic(th)));
    let anchor = crate::reading::initial_anchor(app.reading_dialog(th.peer()).as_deref());
    let b = app.scoped_backend().unwrap();
    let result = b.history(th.peer(), anchor, PAGE).await;
    let _ = app.update(Msg::History { peer: th.peer(), older: false, result });
    assert_eq!(app.open_thread(), Some(th));
    // Los temas de muestra no traen documentos: se sube uno al tema (el mock le da su `FileRef`).
    let item = OutgoingMediaItem { kind: MediaKind::File, path: "/tmp/plano.pdf".into(), file_name: "plano.pdf".into(), size_bytes: 30_000, width: 0, height: 0 };
    let media = OutgoingMedia { items: vec![item], caption: String::new(), reply_to: None, random_ids: vec![RandomId(777)] };
    let sent = b.send_media(th.peer(), media).await.unwrap().remove(0);
    app.conv.as_mut().unwrap().upsert(sent.clone());
    pump(&mut app, &mut ev);
    assert!(app.can_save(&sent));
    let _ = app.update(Msg::Dl(DlMsg::Start(sent.id)));
    run_task(&mut app, &sent).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    pump(&mut app, &mut ev);
    assert_eq!(state(&app, &sent), Some(DlState::Done), "en el tema {:?}", t.id);
    assert!(dir.join("plano.pdf").exists());
    std::fs::remove_dir_all(dir).ok();
}

// --- Carpeta de descargas: predeterminada, elegida y persistencia -------------------------------

use crate::app::downloads::{DIR_KEY, Prefs, default_dir_in, is_writable_dir};

#[derive(Default)]
struct MemPrefs(std::sync::Mutex<std::collections::HashMap<String, String>>);
impl Prefs for MemPrefs {
    fn get(&self, key: &str) -> Option<String> {
        self.0.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, value: Option<&str>) {
        let mut m = self.0.lock().unwrap();
        match value {
            Some(v) => m.insert(key.into(), v.into()),
            None => m.remove(key),
        };
    }
}

#[test]
fn predeterminada_usa_telegram_desktop_si_existe() {
    let home = tmp("pd-home");
    let down = home.join("Descargas");
    std::fs::create_dir_all(down.join("Telegram Desktop")).unwrap();
    assert_eq!(default_dir_in(Some(down.clone()), Some(home.clone())), Some(down.join("Telegram Desktop")));
    std::fs::remove_dir_all(home).ok();
}

#[test]
fn predeterminada_sin_telegram_desktop_es_telegram_y_no_se_crea_al_resolver() {
    let home = tmp("pd-solo");
    let down = home.join("Descargas");
    std::fs::create_dir_all(&down).unwrap();
    assert_eq!(default_dir_in(Some(down.clone()), Some(home.clone())), Some(down.join("Telegram")));
    assert!(!down.join("Telegram").exists(), "se crea al primer uso, no al resolver");
    std::fs::remove_dir_all(home).ok();
}

#[test]
fn predeterminada_sin_descargas_cae_a_la_personal() {
    let home = tmp("pd-sin");
    assert_eq!(default_dir_in(None, Some(home.clone())), Some(home.clone()));
    // Una carpeta Descargas que no existe cuenta como ausente.
    assert_eq!(default_dir_in(Some(home.join("no-hay")), Some(home.clone())), Some(home.clone()));
    assert_eq!(default_dir_in(None, None), None);
    std::fs::remove_dir_all(home).ok();
}

#[tokio::test(start_paused = true)]
async fn la_primera_descarga_crea_la_carpeta_telegram() {
    let (mut app, _) = boot().await;
    let base = tmp("crea");
    let target = base.join("Telegram");
    app.dl.default_override = Some(target.clone());
    let m = find_media(&app, MediaKind::File);
    let _ = app.update(Msg::Dl(DlMsg::Start(m.id)));
    assert!(!target.exists(), "no se crea al decidir el destino");
    let dest = app.dl.get(m.peer, m.id).unwrap().path.clone();
    assert_eq!(dest.parent(), Some(target.as_path()));
    // La tarea de la descarga hace esto mismo antes de pedir el archivo (las `Task` no se ejecutan aquí).
    crate::app::downloads::ensure_parent(&dest).unwrap();
    assert!(target.is_dir(), "se crea al primer uso");
    run_task(&mut app, &m).await.expect("arranca");
    std::fs::remove_dir_all(base).ok();
}

#[tokio::test(start_paused = true)]
async fn carpeta_elegida_se_usa_se_persiste_y_se_restablece() {
    let prefs = Arc::new(MemPrefs::default());
    let chosen = tmp("elegida");
    let (mut app, _) = {
        let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
        let backend: Backend = mock.clone();
        App::new(Flags { prefs: Some(prefs.clone()), ..Flags::new(backend) })
    };
    assert_eq!(app.dl.dir, None);

    let _ = app.update(Msg::Dl(DlMsg::DirPicked(Some(chosen.clone()))));
    assert_eq!(app.dl.dir.as_deref(), Some(chosen.as_path()));
    assert_eq!(prefs.get(DIR_KEY).as_deref(), chosen.to_str(), "queda guardada");
    assert_eq!(app.dl_target_dir(), Some(chosen.clone()));

    // «Guardar como…» propone la elegida: es la misma resolución que usa el selector.
    assert_eq!(app.dl.resolve(None).dir, Some(chosen.clone()));

    // Otra sesión (otra `App` con los mismos ajustes) la recupera.
    let (again, _) = {
        let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
        let backend: Backend = mock.clone();
        App::new(Flags { prefs: Some(prefs.clone()), ..Flags::new(backend) })
    };
    assert_eq!(again.dl.dir.as_deref(), Some(chosen.as_path()));

    let _ = app.update(Msg::Dl(DlMsg::ResetDir));
    assert_eq!(app.dl.dir, None);
    assert_eq!(prefs.get(DIR_KEY), None, "restablecer borra el ajuste");
    std::fs::remove_dir_all(chosen).ok();
}

#[tokio::test(start_paused = true)]
async fn carpeta_elegida_borrada_avisa_y_vuelve_a_la_predeterminada() {
    let prefs = Arc::new(MemPrefs::default());
    let gone = tmp("borrada");
    prefs.set(DIR_KEY, gone.to_str());
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags { prefs: Some(prefs.clone()), ..Flags::new(backend) });
    assert_eq!(app.dl.dir.as_deref(), Some(gone.as_path()), "al arrancar no se valida");
    std::fs::remove_dir_all(&gone).unwrap();

    let r = app.dl.resolve(Some(PathBuf::from("/por/defecto")));
    assert_eq!(r.dir, Some(PathBuf::from("/por/defecto")));
    assert_eq!(r.invalid.as_deref(), Some(gone.as_path()));

    let dir = app.dl_target_dir();
    assert_eq!(dir, default_dir_in(dirs::download_dir(), dirs::home_dir()));
    assert_eq!(app.dl.dir, None, "se olvida");
    assert_eq!(prefs.get(DIR_KEY), None);
    assert!(app.toasts.iter().any(|t| t.error && t.text.contains("predeterminada")));
}

#[test]
fn una_carpeta_elegida_que_no_se_puede_escribir_se_rechaza() {
    let base = tmp("noesc");
    let file = base.join("archivo");
    std::fs::write(&file, b"x").unwrap();
    assert!(!is_writable_dir(&file), "un archivo no es carpeta");
    assert!(!is_writable_dir(&base.join("no-existe")));
    assert!(is_writable_dir(&base));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let ro = base.join("ro");
        std::fs::create_dir(&ro).unwrap();
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
        // Como root los permisos no impiden escribir: solo se comprueba si de verdad no deja.
        let probe = std::fs::File::create(ro.join("p")).is_ok();
        assert_eq!(is_writable_dir(&ro), probe);
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::remove_dir_all(base).ok();
}

#[tokio::test(start_paused = true)]
async fn elegir_una_carpeta_invalida_avisa_y_no_la_guarda() {
    let (mut app, _) = boot().await;
    let _ = app.update(Msg::Dl(DlMsg::DirPicked(Some(PathBuf::from("/no/existe/de/verdad")))));
    assert_eq!(app.dl.dir, None);
    assert!(app.toasts.iter().any(|t| t.error));
}

#[tokio::test(start_paused = true)]
async fn el_raíl_abre_el_dialogo_de_la_carpeta() {
    let (mut app, _) = boot().await;
    let _ = app.update(Msg::Dl(DlMsg::Settings));
    assert!(matches!(app.overlay, Some(crate::app::Overlay::DownloadDir)));
    let _ = app.view();
    let _ = app.update(Msg::CloseOverlay);
    assert!(app.overlay.is_none());
}
