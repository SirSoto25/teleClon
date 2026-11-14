//! Notificaciones de escritorio contra el mock con `RecordingNotifier` (fase C2): cada regla
//! (foco y chat abierto, propio, silenciado y mención, archivado, por tipo, secreto sin texto, «se
//! unió», «No molestar», ráfaga, carga inicial), temas de foro, clic que abre el chat, «Marcar como
//! leído», retirada al leer y ajustes que persisten. Las `Task` no se ejecutan.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use futures::{FutureExt, StreamExt};
use tc_core::*;
use tc_mock::{BIG_CHAT, FORUM_CHAT, FORUM_GROUP, MockBackend, MockConfig, SECRET_ID, UNREAD_CHAT};
use tc_notify::rules::{ACTION_MARK_READ, DoNotDisturb, Preview};
use tc_notify::{ChatKey, Icon, NotifyEvent, NotifyEvents, RecordingNotifier};

use crate::app::downloads::Prefs;
use crate::app::forum::ForumMsg;
use crate::app::notify::{DndChoice, NotifyHandle, NotifyMsg, Toggle, chat_key, topic_key};
use crate::app::{App, Backend, Flags, Msg, Overlay};
use crate::sticker_anim::AnimMsg;

const NOW: i64 = 1_762_430_400;

#[derive(Default)]
struct MemPrefs(Mutex<HashMap<String, String>>);
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

struct T {
    app: App,
    mock: Arc<MockBackend>,
    rec: RecordingNotifier,
    clicks: NotifyEvents,
    ev: futures::stream::BoxStream<'static, BackendEvent>,
}

impl T {
    /// Entrega a la UI los eventos pendientes del backend.
    fn pump(&mut self) {
        while let Some(Some(e)) = self.ev.next().now_or_never() {
            let _ = self.app.update(Msg::Backend(e));
        }
    }

    fn incoming(&mut self, peer: PeerRef, text: &str) -> Message {
        let m = self.mock.inject_incoming(peer, text).unwrap();
        self.pump();
        m
    }

    fn focus(&mut self, f: bool) {
        let _ = self.app.update(Msg::StickerAnim(AnimMsg::Focus(f)));
    }

    async fn mute(&mut self, peer: PeerRef, on: bool) {
        self.mock.set_muted(peer, on).await.unwrap();
        self.pump();
    }

    async fn archive(&mut self, peer: PeerRef, on: bool) {
        self.mock.set_archived(peer, on).await.unwrap();
        self.pump();
    }

    fn dialog_mut(&mut self, peer: PeerRef) -> &mut Dialog {
        self.app.dialogs.iter_mut().find(|d| d.peer == peer).expect("el chat está en la lista")
    }

    fn set(&mut self, t: Toggle, v: bool) {
        let _ = self.app.update(Msg::Notify(NotifyMsg::Set(t, v)));
    }

    /// Entrega a la app los eventos que el notificador simulado mandó (como la suscripción).
    fn deliver_clicks(&mut self) {
        while let Some(Some(e)) = self.clicks.next().now_or_never() {
            let _ = self.app.update(Msg::Notify(NotifyMsg::Event(e)));
        }
    }

    fn channel(&self) -> PeerRef {
        self.app.dialogs.iter().map(|d| d.peer).find(|p| matches!(p, PeerRef::Group(g) if self.app.groups.get(g).is_some_and(|g| g.kind == GroupKind::Channel))).expect("hay un canal")
    }

    fn group(&self) -> PeerRef {
        self.app.dialogs.iter().map(|d| d.peer).find(|p| matches!(p, PeerRef::Group(g) if self.app.groups.get(g).is_some_and(|g| g.kind != GroupKind::Channel && !g.is_forum))).expect("hay un grupo")
    }
}

async fn boot_with(prefs: Option<Arc<dyn Prefs>>, load: bool) -> T {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (rec, clicks) = RecordingNotifier::new();
    // El canal de la app no se usa en las pruebas: los clics se leen de `clicks` y se entregan a mano.
    let (_tx, unused) = tc_notify::event_channel();
    let flags = Flags { notify: Some(NotifyHandle::new(Arc::new(rec.clone()), unused)), prefs, ..Flags::new(backend.clone()) };
    let (mut app, _) = App::new(flags);
    app.notifs.clock = Some(NOW);
    let ev = backend.events();
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::Contacts(backend.contacts().await));
    let _ = app.update(Msg::Secrets(backend.secret_chats().await));
    let mut t = T { app, mock, rec, clicks, ev };
    if load {
        let _ = t.app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
        let groups: Vec<GroupId> = t.app.dialogs.iter().filter_map(|d| if let PeerRef::Group(g) = d.peer { Some(g) } else { None }).collect();
        for g in groups {
            let _ = t.app.update(Msg::GroupLoaded(backend.group(g).await));
        }
        // Ningún chat silenciado de partida (en el mock los canales lo están): cada prueba pone lo
        // que necesita, en el backend, para que el `DialogUpserted` de cada mensaje no lo deshaga.
        let muted: Vec<PeerRef> = t.app.dialogs.iter().filter(|d| d.muted).map(|d| d.peer).collect();
        for p in muted {
            t.mock.set_muted(p, false).await.unwrap();
        }
        t.pump();
    }
    t
}

async fn boot() -> T {
    boot_with(None, true).await
}

#[tokio::test]
async fn privado_sin_foco_avisa_con_texto_y_boton() {
    let mut t = boot().await;
    t.focus(false);
    t.incoming(BIG_CHAT, "¿Quedamos a las seis?");
    let n = t.rec.last().expect("aviso");
    assert_eq!(n.chat_key, chat_key(BIG_CHAT));
    assert_eq!(n.title, t.app.dialog(BIG_CHAT).unwrap().title);
    assert_eq!(n.body, "¿Quedamos a las seis?");
    assert_eq!(n.actions[0].id, ACTION_MARK_READ);
    assert!(!n.silent, "el primero suena");
}

#[tokio::test]
async fn chat_abierto_con_foco_no_avisa_y_sin_foco_si() {
    let mut t = boot().await;
    let _ = t.app.update(Msg::Open(BIG_CHAT));
    t.incoming(BIG_CHAT, "uno");
    assert!(t.rec.sent().is_empty(), "lo está viendo");
    // Con foco pero otro chat abierto: avisa.
    t.incoming(UNREAD_CHAT, "dos");
    assert_eq!(t.rec.sent().len(), 1);
    t.focus(false);
    t.incoming(BIG_CHAT, "tres");
    assert_eq!(t.rec.last().unwrap().chat_key, chat_key(BIG_CHAT));
}

#[tokio::test]
async fn mensaje_propio_no_avisa() {
    let mut t = boot().await;
    t.focus(false);
    let me = t.app.me_id().unwrap();
    let mut m = t.mock.inject_incoming(BIG_CHAT, "eco").unwrap();
    t.rec.clear();
    m.id = MsgId(m.id.0 + 1000);
    m.outgoing = true;
    m.sender = Some(me);
    let _ = t.app.update(Msg::Backend(BackendEvent::MessageNew(m.clone())));
    // Mismo autor que yo aunque no venga marcado como saliente (otro dispositivo).
    m.id = MsgId(m.id.0 + 1);
    m.outgoing = false;
    let _ = t.app.update(Msg::Backend(BackendEvent::MessageNew(m)));
    assert!(t.rec.sent().is_empty());
}

#[tokio::test]
async fn silenciado_calla_salvo_mencion_y_el_plazo_vencido_vuelve_a_sonar() {
    let mut t = boot().await;
    t.focus(false);
    let g = t.group();
    t.mute(g, true).await;
    t.incoming(g, "nada importante");
    assert!(t.rec.sent().is_empty(), "silenciado");
    t.incoming(g, "@yo_mismo ¿lo miras?");
    assert_eq!(t.rec.sent().len(), 1, "la mención atraviesa el silencio");

    // Silencio con plazo desde el menú: dentro del plazo calla, vencido avisa aunque la lista siga «muted».
    t.rec.clear();
    let _ = t.app.update(Msg::Chat(crate::app::chat_actions::ChatMsg::Mute(BIG_CHAT, Some(crate::app::chat_actions::MuteFor::Hour))));
    // Lo que haría la tarea del menú contra el backend (la lista sigue «muted» tras cada mensaje).
    t.mock.mute_until(BIG_CHAT, Some(Timestamp(t.app.notifs.now() + 3600))).await.unwrap();
    t.pump();
    assert!(t.app.dialog(BIG_CHAT).unwrap().muted);
    t.incoming(BIG_CHAT, "a");
    assert!(t.rec.sent().is_empty());
    t.app.notifs.clock = Some(NOW + 3601);
    t.incoming(BIG_CHAT, "b");
    assert!(t.app.dialog(BIG_CHAT).unwrap().muted, "la lista no se ha enterado");
    assert_eq!(t.rec.sent().len(), 1, "el plazo ya pasó");
}

#[tokio::test]
async fn archivados_siguen_el_ajuste() {
    let mut t = boot().await;
    t.focus(false);
    t.archive(BIG_CHAT, true).await;
    assert!(t.app.dialog(BIG_CHAT).unwrap().archived);
    t.incoming(BIG_CHAT, "desde archivados");
    assert_eq!(t.rec.sent().len(), 1, "por defecto avisa (como Telegram)");
    t.set(Toggle::Archived, false);
    t.incoming(BIG_CHAT, "otra vez");
    assert_eq!(t.rec.sent().len(), 1);
    // Archivado y silenciado: nunca.
    t.set(Toggle::Archived, true);
    t.mute(BIG_CHAT, true).await;
    t.incoming(BIG_CHAT, "silenciado");
    assert_eq!(t.rec.sent().len(), 1);
}

#[tokio::test]
async fn ajustes_por_tipo() {
    let mut t = boot().await;
    t.focus(false);
    let (g, ch) = (t.group(), t.channel());
    t.set(Toggle::Groups, false);
    t.set(Toggle::Channels, false);
    t.incoming(g, "grupo");
    t.incoming(ch, "canal");
    assert!(t.rec.sent().is_empty());
    t.incoming(BIG_CHAT, "privado");
    assert_eq!(t.rec.sent().len(), 1);
    t.set(Toggle::Private, false);
    t.incoming(BIG_CHAT, "privado 2");
    assert_eq!(t.rec.sent().len(), 1);
    t.set(Toggle::Channels, true);
    t.incoming(ch, "canal 2");
    assert_eq!(t.rec.last().unwrap().chat_key, chat_key(ch));
    // Interruptor general.
    t.set(Toggle::Enabled, false);
    t.incoming(ch, "canal 3");
    assert_eq!(t.rec.sent().len(), 2);
}

#[tokio::test]
async fn grupo_antepone_el_autor() {
    let mut t = boot().await;
    t.focus(false);
    let g = t.group();
    let m = t.incoming(g, "hola a todos");
    let n = t.rec.last().unwrap();
    let who = t.app.users.get(&m.sender.unwrap()).map(User::display_name).unwrap();
    assert_eq!(n.body, format!("{who}: hola a todos"));
}

#[tokio::test]
async fn secreto_nunca_muestra_el_texto_por_defecto() {
    let mut t = boot().await;
    t.focus(false);
    let peer = PeerRef::Secret(SECRET_ID);
    t.incoming(peer, "la clave es 1234");
    let n = t.rec.last().expect("aviso del chat secreto");
    assert_eq!(n.chat_key, chat_key(peer));
    assert!(!n.body.contains("1234") && !n.title.contains("1234"));
    assert_eq!(n.body, "Mensaje nuevo");
    // Vista previa general «Nada»: ni el nombre.
    let _ = t.app.update(Msg::Notify(NotifyMsg::Preview(Preview::Hidden)));
    t.incoming(BIG_CHAT, "secreto también");
    let n = t.rec.last().unwrap();
    assert_eq!((n.title.as_str(), n.body.contains("secreto")), ("Parlo", false));
    // Si se pide, el secreto enseña el texto (con la vista previa completa).
    let _ = t.app.update(Msg::Notify(NotifyMsg::Preview(Preview::Full)));
    t.set(Toggle::SecretText, true);
    t.incoming(peer, "ahora sí");
    assert!(t.rec.last().unwrap().body.contains("ahora sí"));
}

#[tokio::test]
async fn se_unio_a_telegram_se_ignora_por_defecto() {
    let mut t = boot().await;
    t.focus(false);
    t.incoming(BIG_CHAT, crate::app::notify::CONTACT_JOINED_TEXT);
    assert!(t.rec.sent().is_empty());
    t.set(Toggle::IgnoreJoined, false);
    t.incoming(BIG_CHAT, crate::app::notify::CONTACT_JOINED_TEXT);
    assert_eq!(t.rec.sent().len(), 1);
}

#[tokio::test]
async fn no_molestar_apagado_encendido_y_hasta_una_hora() {
    let mut t = boot().await;
    t.focus(false);
    let _ = t.app.update(Msg::Notify(NotifyMsg::Dnd(DndChoice::On)));
    t.incoming(BIG_CHAT, "a");
    assert!(t.rec.sent().is_empty());
    let _ = t.app.update(Msg::Notify(NotifyMsg::Dnd(DndChoice::Hour)));
    assert_eq!(t.app.notifs.settings().dnd, DoNotDisturb::Until(NOW + 3600));
    t.incoming(BIG_CHAT, "b");
    assert!(t.rec.sent().is_empty());
    t.app.notifs.clock = Some(NOW + 3600);
    t.incoming(BIG_CHAT, "c");
    assert_eq!(t.rec.sent().len(), 1, "terminó el plazo");
    let _ = t.app.update(Msg::Notify(NotifyMsg::Dnd(DndChoice::On)));
    let _ = t.app.update(Msg::Notify(NotifyMsg::Dnd(DndChoice::Off)));
    t.incoming(BIG_CHAT, "d");
    assert_eq!(t.rec.sent().len(), 2);
}

#[tokio::test]
async fn rafaga_al_reconectar_limitada_y_con_un_solo_sonido() {
    let mut t = boot().await;
    t.focus(false);
    let peers: Vec<PeerRef> = t.app.dialogs.iter().map(|d| d.peer).filter(|p| !matches!(p, PeerRef::Group(g) if *g == FORUM_GROUP)).take(10).collect();
    let _ = t.app.update(Msg::Backend(BackendEvent::Connection(ConnectionState::Connecting)));
    t.incoming(BIG_CHAT, "mientras conecta");
    assert!(t.rec.sent().is_empty(), "sin conexión al día no se avisa");
    let _ = t.app.update(Msg::Backend(BackendEvent::Connection(ConnectionState::Online)));
    for p in &peers {
        t.incoming(*p, "puesta al día");
    }
    let sent = t.rec.sent();
    assert_eq!(sent.len(), 5, "límite del Engine: 5 en 10 s");
    assert_eq!(sent.iter().filter(|n| !n.silent).count(), 1, "un solo sonido");
    // Pasada la ventana, el siguiente aviso de un chat contenido suma lo que no se avisó.
    t.app.notifs.clock = Some(NOW + 11);
    t.incoming(peers[7], "otro");
    let n = t.rec.last().unwrap();
    assert!(n.title.contains("(2)") || n.body.starts_with("2 "), "agrupado: {n:?}");
}

#[tokio::test]
async fn nunca_durante_la_carga_inicial() {
    let mut t = boot_with(None, false).await;
    t.focus(false);
    t.incoming(BIG_CHAT, "antes de la lista");
    assert!(t.rec.sent().is_empty());
    let _ = t.app.update(Msg::DialogsLoaded(t.mock.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    // Mensaje antiguo que llega en la puesta al día: no se anuncia.
    let mut old = t.mock.inject_incoming(BIG_CHAT, "de ayer").unwrap();
    t.rec.clear();
    t.app.dialogs.iter_mut().for_each(|d| d.muted = false);
    old.id = MsgId(old.id.0 + 500);
    old.date = Timestamp(NOW - 86_400);
    let _ = t.app.update(Msg::Backend(BackendEvent::MessageNew(old)));
    assert!(t.rec.sent().is_empty());
    t.ev.next().now_or_never();
    t.incoming(BIG_CHAT, "ahora");
    assert_eq!(t.rec.sent().len(), 1);
}

#[tokio::test]
async fn clic_enfoca_y_abre_el_chat_y_lo_da_por_leido() {
    let mut t = boot().await;
    t.focus(false);
    t.incoming(UNREAD_CHAT, "mira esto");
    let key = t.rec.last().unwrap().chat_key;
    assert!(t.app.conv.is_none());
    t.app.overlay = Some(Overlay::DownloadDir);
    t.rec.click(key.clone());
    t.deliver_clicks();
    assert_eq!(t.app.conv.as_ref().map(|c| c.peer), Some(UNREAD_CHAT));
    assert!(t.app.overlay.is_none(), "nada tapa el chat");
    assert_eq!(t.rec.withdrawn(), vec![key]);
}

#[tokio::test]
async fn accion_marcar_como_leido() {
    let mut t = boot().await;
    t.focus(false);
    t.incoming(BIG_CHAT, "léeme");
    assert!(t.app.dialog(BIG_CHAT).unwrap().unread_count > 0);
    t.rec.press(chat_key(BIG_CHAT), ACTION_MARK_READ);
    t.deliver_clicks();
    let d = t.app.dialog(BIG_CHAT).unwrap();
    assert_eq!((d.unread_count, d.unread_mark), (0, false));
    assert_eq!(t.rec.withdrawn(), vec![chat_key(BIG_CHAT)]);
    assert!(t.app.conv.is_none(), "no abre el chat");
    // Acción desconocida: nada.
    t.rec.press(chat_key(BIG_CHAT), "otra");
    t.deliver_clicks();
    assert_eq!(t.rec.withdrawn().len(), 1);
}

#[tokio::test]
async fn leer_retira_el_aviso() {
    let mut t = boot().await;
    t.focus(false);
    // Abrirlo.
    t.incoming(BIG_CHAT, "uno");
    let _ = t.app.update(Msg::Open(BIG_CHAT));
    assert_eq!(t.rec.withdrawn(), vec![chat_key(BIG_CHAT)]);
    // Abierto sin foco: avisa; al volver el foco con ese chat abierto, se retira.
    t.incoming(BIG_CHAT, "dos");
    assert_eq!(t.rec.sent().len(), 2);
    t.focus(true);
    assert_eq!(t.rec.withdrawn().len(), 2);
    // ReadInbox desde otro dispositivo: solo si llega hasta el último avisado.
    t.focus(false);
    let m = t.incoming(UNREAD_CHAT, "tres");
    let _ = t.app.update(Msg::Backend(BackendEvent::ReadInbox { peer: UNREAD_CHAT, max_id: MsgId(m.id.0 - 1) }));
    assert_eq!(t.rec.withdrawn().len(), 2, "lectura más antigua: sigue");
    let _ = t.app.update(Msg::Backend(BackendEvent::ReadInbox { peer: UNREAD_CHAT, max_id: m.id }));
    assert_eq!(t.rec.withdrawn().last(), Some(&chat_key(UNREAD_CHAT)));
    // El grupo empieza de cero tras leer: el siguiente no dice «(2)».
    t.incoming(UNREAD_CHAT, "cuatro");
    assert!(!t.rec.last().unwrap().title.contains('('));
}

#[tokio::test]
async fn foro_usa_el_titulo_del_tema_y_el_clic_abre_el_tema() {
    let mut t = boot().await;
    let _ = t.app.update(Msg::Open(FORUM_CHAT));
    let result = t.mock.forum_topics(FORUM_GROUP, TopicsQuery::first(crate::app::forum::TOPICS_PAGE)).await;
    let _ = t.app.update(Msg::Forum(ForumMsg::Loaded { group: FORUM_GROUP, append: false, query: None, result }));
    let topic = t.app.forum.topics[&FORUM_GROUP].items[1].clone();
    let th = ThreadRef { group: FORUM_GROUP, topic: topic.id };
    let _ = t.app.update(Msg::Open(BIG_CHAT));
    t.mock.inject_topic_incoming(th, "novedades del tema").unwrap();
    t.pump();
    let n = t.rec.last().expect("aviso del tema");
    assert_eq!(n.chat_key, topic_key(th));
    assert!(n.title.starts_with(&topic.title), "{}", n.title);
    assert!(n.body.ends_with("novedades del tema"));
    t.rec.click(n.chat_key.clone());
    t.deliver_clicks();
    assert_eq!(t.app.open_thread(), Some(th));
    assert_eq!(t.rec.withdrawn(), vec![topic_key(th)]);
    // Con el tema abierto y foco: ese tema no avisa.
    t.rec.clear();
    t.mock.inject_topic_incoming(th, "otra").unwrap();
    t.pump();
    assert!(t.rec.sent().is_empty());
}

#[tokio::test]
async fn icono_del_chat_si_el_avatar_esta_en_cache() {
    let mut t = boot().await;
    t.focus(false);
    let photo = ProfilePhoto { id: 77, dc_id: 2 };
    t.dialog_mut(BIG_CHAT).photo = Some(photo);
    let key = crate::media::ImageKey::Avatar { peer: BIG_CHAT, photo: 77 };
    let px = vec![200u8; 4 * 4 * 4];
    t.app.images.start(key);
    t.app.images.finish(
        key,
        Some(crate::media::Decoded { handle: iced::widget::image::Handle::from_rgba(4, 4, px), width: 4, height: 4 }),
        Instant::now(),
    );
    t.incoming(BIG_CHAT, "con foto");
    match t.rec.last().unwrap().icon {
        Some(Icon::Bytes(b)) => assert_eq!(&b[1..4], b"PNG"),
        other => panic!("sin icono: {other:?}"),
    }
    // Sin foto en caché: sin icono.
    t.incoming(UNREAD_CHAT, "sin foto");
    assert!(t.rec.last().unwrap().icon.is_none());
}

#[tokio::test]
async fn ajustes_persisten_y_se_cargan_al_arrancar() {
    let prefs: Arc<MemPrefs> = Arc::default();
    let mut t = boot_with(Some(prefs.clone()), true).await;
    let _ = t.app.update(Msg::Notify(NotifyMsg::Settings));
    assert!(matches!(t.app.overlay, Some(Overlay::Notifications)));
    let _ = t.app.view();
    t.set(Toggle::Groups, false);
    t.set(Toggle::Sound, false);
    let _ = t.app.update(Msg::Notify(NotifyMsg::Preview(Preview::NameOnly)));
    let _ = t.app.update(Msg::Notify(NotifyMsg::Dnd(DndChoice::EightHours)));
    {
        let m = prefs.0.lock().unwrap();
        assert_eq!(m.get("notify.groups").map(String::as_str), Some("0"));
        assert_eq!(m.get("notify.preview").map(String::as_str), Some("name"));
        assert_eq!(m.get("notify.dnd"), Some(&format!("until:{}", NOW + 8 * 3600)));
        assert!(!m.contains_key("notify.channels"), "solo se guarda lo que cambia");
    }
    let again = boot_with(Some(prefs), true).await;
    let s = again.app.notifs.settings();
    assert!(!s.groups && !s.sound && s.channels);
    assert_eq!(s.preview, Preview::NameOnly);
    assert_eq!(s.dnd, DoNotDisturb::Until(NOW + 8 * 3600));
}

#[tokio::test]
async fn el_rail_abre_el_dialogo_y_se_cierra() {
    let mut t = boot().await;
    let _ = t.app.update(Msg::Notify(NotifyMsg::Settings));
    let _ = t.app.view();
    t.set(Toggle::Enabled, false);
    let _ = t.app.view();
    let _ = t.app.update(Msg::CloseOverlay);
    assert!(t.app.overlay.is_none());
}

#[tokio::test]
async fn clic_con_clave_ajena_no_hace_nada() {
    let mut t = boot().await;
    t.rec.click(ChatKey::from("zz:1"));
    t.rec.press(ChatKey::from("zz:1"), ACTION_MARK_READ);
    t.deliver_clicks();
    assert!(t.app.conv.is_none() && t.rec.withdrawn().is_empty());
    let _ = NotifyEvent::Clicked(ChatKey::from("zz"));
}
