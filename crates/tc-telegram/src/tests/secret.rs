//! Chats secretos de extremo a extremo entre dos `GrammersBackend` (Ana y Beto) unidos por
//! un servidor falso que solo reenvía lo que recibe (como haría Telegram con bytes
//! cifrados). Comprueba la integración TL ↔ `tc-secret` entre dos instancias de este
//! mismo código: NO demuestra interoperabilidad con clientes oficiales.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use futures::stream::BoxStream;
use grammers_client::InvocationError;
use grammers_tl_types as tl;
use tc_core::*;
use tokio::sync::mpsc;

use super::fixtures::*;
use super::*;
use crate::{MemorySecretStateStore, SecretStateStore, UpdateBatch};

const ANA: i64 = ME_ID;
const BETO: i64 = 88;
const CHAT: i32 = 5150;
const HASH: i64 = 4242;

fn name(id: i64) -> &'static str {
    if id == ANA { "Ana" } else { "Beto" }
}

/// Fuente de updates alimentada por el servidor falso.
struct ChanSource(mpsc::UnboundedReceiver<UpdateBatch>);

#[async_trait]
impl UpdateSource for ChanSource {
    async fn next_batch(&mut self) -> Result<UpdateBatch, InvocationError> {
        match self.0.recv().await {
            Some(b) => Ok(b),
            None => futures::future::pending().await,
        }
    }
}

#[derive(Default)]
struct Server {
    date: i32,
    qts: HashMap<i64, i32>,
    g_a: Vec<u8>,
    /// Lo que el servidor ve de cada `sendEncrypted*`: (emisor, bytes).
    seen: Vec<(i64, Vec<u8>)>,
    /// `receivedQueue` recibidos: (cuenta, max_qts).
    acks: Vec<(i64, i32)>,
    /// Pierde el siguiente mensaje de este emisor (para provocar un hueco).
    lose_next_from: Option<i64>,
    /// Canal de updates de cada cuenta (se cambia al «reiniciar» una de ellas).
    tx: HashMap<i64, mpsc::UnboundedSender<UpdateBatch>>,
}

impl Server {
    fn push(&self, to: i64, from: i64, u: tl::enums::Update) {
        let users = vec![user_enum(from, name(from), from * 10, false)];
        let _ = self.tx[&to].send((vec![u], users, vec![]));
    }
    fn next_date(&mut self) -> i32 {
        self.date += 1;
        1_800_000_000 + self.date
    }
}

type Srv = Arc<Mutex<Server>>;

fn input_chat(p: &tl::enums::InputEncryptedChat) -> (i32, i64) {
    let tl::enums::InputEncryptedChat::Chat(c) = p;
    (c.chat_id, c.access_hash)
}

fn handler(me: i64, other: i64, srv: Srv) -> impl FnMut(i32, u32, &[u8]) -> Reply + Send + 'static {
    use tl::functions::{messages as m, users};
    move |_, id, body| {
        let mut s = srv.lock().unwrap();
        match id {
            x if x == cid::<users::GetUsers>() => ok(vec![user_enum(me, name(me), 1, true)]),
            x if x == cid::<m::GetDhConfig>() => {
                let req: m::GetDhConfig = decode(body);
                assert_eq!(req.random_length, 256);
                let random = vec![me as u8; 256];
                if req.version == 1 {
                    ok(tl::enums::messages::DhConfig::from(tl::types::messages::DhConfigNotModified { random }))
                } else {
                    ok(tl::enums::messages::DhConfig::from(tl::types::messages::DhConfig { g: 2, p: hex(RFC3526_2048), version: 1, random }))
                }
            }
            x if x == cid::<m::RequestEncryption>() => {
                let req: m::RequestEncryption = decode(body);
                assert!(matches!(req.user_id, tl::enums::InputUser::User(u) if u.user_id == other && u.access_hash == other * 10));
                s.g_a = req.g_a.clone();
                let date = s.next_date();
                let r = tl::types::EncryptedChatRequested { folder_id: None, id: CHAT, access_hash: HASH, date, admin_id: me, participant_id: other, g_a: req.g_a };
                s.push(other, me, tl::types::UpdateEncryption { chat: r.into(), date }.into());
                ok(tl::enums::EncryptedChat::from(tl::types::EncryptedChatWaiting { id: CHAT, access_hash: HASH, date, admin_id: me, participant_id: other }))
            }
            x if x == cid::<m::AcceptEncryption>() => {
                let req: m::AcceptEncryption = decode(body);
                assert_eq!(input_chat(&req.peer), (CHAT, HASH));
                let date = s.next_date();
                let c = |g: Vec<u8>| tl::types::EncryptedChat {
                    id: CHAT,
                    access_hash: HASH,
                    date,
                    admin_id: other,
                    participant_id: me,
                    g_a_or_b: g,
                    key_fingerprint: req.key_fingerprint,
                };
                s.push(other, me, tl::types::UpdateEncryption { chat: c(req.g_b.clone()).into(), date }.into());
                ok(tl::enums::EncryptedChat::from(c(s.g_a.clone())))
            }
            x if x == cid::<m::SendEncrypted>() || x == cid::<m::SendEncryptedService>() => {
                let (peer, random_id, data, service) = if x == cid::<m::SendEncrypted>() {
                    let r: m::SendEncrypted = decode(body);
                    (r.peer, r.random_id, r.data, false)
                } else {
                    let r: m::SendEncryptedService = decode(body);
                    (r.peer, r.random_id, r.data, true)
                };
                assert_eq!(input_chat(&peer), (CHAT, HASH));
                s.seen.push((me, data.clone()));
                let date = s.next_date();
                if s.lose_next_from == Some(me) {
                    s.lose_next_from = None;
                } else {
                    let message: tl::enums::EncryptedMessage = if service {
                        tl::types::EncryptedMessageService { random_id, chat_id: CHAT, date, bytes: data }.into()
                    } else {
                        tl::types::EncryptedMessage { random_id, chat_id: CHAT, date, bytes: data, file: tl::enums::EncryptedFile::Empty }.into()
                    };
                    let q = s.qts.entry(other).or_default();
                    *q += 1;
                    let qts = *q;
                    s.push(other, me, tl::types::UpdateNewEncryptedMessage { message, qts }.into());
                }
                ok(tl::enums::messages::SentEncryptedMessage::from(tl::types::messages::SentEncryptedMessage { date }))
            }
            x if x == cid::<m::ReceivedQueue>() => {
                let req: m::ReceivedQueue = decode(body);
                s.acks.push((me, req.max_qts));
                ok(Vec::<i64>::new())
            }
            x if x == cid::<m::SetEncryptedTyping>() => {
                let req: m::SetEncryptedTyping = decode(body);
                if req.typing {
                    s.push(other, me, tl::types::UpdateEncryptedChatTyping { chat_id: CHAT }.into());
                }
                ok(true)
            }
            x if x == cid::<m::ReadEncryptedHistory>() => {
                let req: m::ReadEncryptedHistory = decode(body);
                let date = s.next_date();
                s.push(other, me, tl::types::UpdateEncryptedMessagesRead { chat_id: CHAT, max_date: req.max_date, date }.into());
                ok(true)
            }
            x if x == cid::<m::DiscardEncryption>() => {
                let req: m::DiscardEncryption = decode(body);
                let date = s.next_date();
                let d = tl::types::EncryptedChatDiscarded { history_deleted: req.delete_history, id: req.chat_id };
                s.push(other, me, tl::types::UpdateEncryption { chat: d.into(), date }.into());
                ok(true)
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        }
    }
}

struct Side {
    b: GrammersBackend,
    fake: Arc<Fake>,
    ev: BoxStream<'static, BackendEvent>,
}

/// Arranca la cuenta `me` (ya con sesión) contra el servidor compartido.
async fn side(me: i64, srv: &Srv, store: Arc<MemorySecretStateStore>) -> Side {
    let other = if me == ANA { BETO } else { ANA };
    let (tx, rx) = mpsc::unbounded_channel();
    srv.lock().unwrap().tx.insert(me, tx);
    let fake = Fake::new(handler(me, other, Arc::clone(srv)));
    let b = GrammersBackend::with_transport(cfg(), Arc::new(MemorySessionStore::default()), logged_session_as(me), fake.clone(), Some(Box::new(ChanSource(rx))));
    b.set_secret_store(store).await;
    wait_auth(&b, |s| matches!(s, AuthState::LoggedIn(_))).await;
    b.inner.ingest(&[user_enum(other, name(other), other * 10, false)], &[]);
    let ev = b.events();
    Side { b, fake, ev }
}

async fn until(ev: &mut BoxStream<'static, BackendEvent>, mut f: impl FnMut(&BackendEvent) -> bool) -> BackendEvent {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let e = ev.next().await.expect("flujo de eventos abierto");
            if f(&e) {
                return e;
            }
        }
    })
    .await
    .expect("evento no recibido a tiempo")
}

fn text(t: &str, rid: i64) -> OutgoingText {
    OutgoingText { text: t.into(), entities: vec![], reply_to: None, random_id: RandomId(rid) }
}

async fn received(s: &mut Side, t: &str) -> Message {
    match until(&mut s.ev, |e| matches!(e, BackendEvent::MessageNew(m) if m.text == t)).await {
        BackendEvent::MessageNew(m) => m,
        _ => unreachable!(),
    }
}

const PEER: PeerRef = PeerRef::Secret(SecretChatId(CHAT));

/// Solicitud de Ana, aceptación de Beto y espera a que ambos tengan la clave.
async fn handshake(ana: &mut Side, beto: &mut Side) {
    let c = ana.b.request_secret_chat(UserId(BETO)).await.unwrap();
    assert_eq!((c.id, c.user, c.state, c.outgoing), (SecretChatId(CHAT), UserId(BETO), SecretChatState::Waiting, true));
    let BackendEvent::SecretChat(inc) = until(&mut beto.ev, |e| matches!(e, BackendEvent::SecretChat(_))).await else { unreachable!() };
    assert_eq!((inc.user, inc.state, inc.outgoing), (UserId(ANA), SecretChatState::Waiting, false));
    assert_eq!(beto.b.accept_secret_chat(inc.id).await.unwrap().state, SecretChatState::Accepted);
    until(&mut ana.ev, |e| matches!(e, BackendEvent::SecretChat(c) if c.state == SecretChatState::Accepted)).await;
}

async fn fingerprint(s: &Side) -> Option<i64> {
    s.b.inner.secret.state.lock().await.chats.get(&CHAT).and_then(|c| c.session.as_ref()).and_then(|x| x.key_fingerprint())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_backends_talk_through_a_relay_server() {
    let srv = Srv::default();
    let ana_store = Arc::new(MemorySecretStateStore::default());
    let mut ana = side(ANA, &srv, ana_store.clone()).await;
    let mut beto = side(BETO, &srv, Arc::new(MemorySecretStateStore::default())).await;
    handshake(&mut ana, &mut beto).await;

    // Misma huella visual en ambos extremos.
    let (va, vb) = (ana.b.secret_key_visual(SecretChatId(CHAT)).await.unwrap(), beto.b.secret_key_visual(SecretChatId(CHAT)).await.unwrap());
    assert_eq!(va, vb);
    assert_eq!(va.emoji.len(), 4);
    assert_eq!(fingerprint(&ana).await, fingerprint(&beto).await);

    // Texto con formato, en ambos sentidos y con respuesta.
    let bold = vec![TextEntity { kind: EntityKind::Bold, offset: 5, length: 2 }];
    let out = OutgoingText { text: "hola 🦊 secreto".into(), entities: bold.clone(), reply_to: None, random_id: RandomId(1) };
    let sent = ana.b.send_text(PEER, out.clone()).await.unwrap();
    assert_eq!((sent.state, sent.outgoing, sent.sender, sent.peer), (SendState::Sent, true, Some(UserId(ANA)), PEER));
    let got = received(&mut beto, "hola 🦊 secreto").await;
    assert_eq!((got.entities.clone(), got.sender, got.random_id, got.outgoing), (bold, Some(UserId(ANA)), Some(RandomId(1)), false));
    let reply = OutgoingText { reply_to: Some(got.id), ..text("¿qué tal?", 2) };
    beto.b.send_text(PEER, reply).await.unwrap();
    let back = received(&mut ana, "¿qué tal?").await;
    assert_eq!(back.reply_to, Some(sent.id), "la respuesta apunta al mensaje original en el otro extremo");
    // Reintento con el mismo random_id: no sale otra vez.
    let n = ana.fake.count("messages.sendEncrypted");
    assert_eq!(ana.b.send_text(PEER, out).await.unwrap().id, sent.id);
    assert_eq!(ana.fake.count("messages.sendEncrypted"), n);
    assert_eq!(beto.b.history(PEER, Anchor::Latest, 10).await.unwrap().iter().map(|m| m.text.as_str()).collect::<Vec<_>>(), ["hola 🦊 secreto", "¿qué tal?"]);

    // El servidor solo vio bytes cifrados.
    for (_, data) in srv.lock().unwrap().seen.iter() {
        assert!(!data.windows(4).any(|w| w == b"hola"), "texto en claro en el cable");
    }
    // Confirmación de la cola (qts) tras procesar.
    assert!(srv.lock().unwrap().acks.iter().any(|(who, q)| *who == BETO && *q >= 2));

    // Escribiendo… y leídos.
    ana.b.set_typing(PEER, true).await.unwrap();
    until(&mut beto.ev, |e| matches!(e, BackendEvent::Typing { peer: PEER, user: UserId(ANA), typing: true })).await;
    beto.b.mark_read(PEER, got.id).await.unwrap();
    until(&mut ana.ev, |e| matches!(e, BackendEvent::ReadOutbox { peer: PEER, max_id } if *max_id == sent.id)).await;

    // Autodestrucción: TTL de 1 s; Beto lo lee, ambos lo borran al vencer.
    ana.b.set_secret_ttl(SecretChatId(CHAT), Some(1)).await.unwrap();
    until(&mut beto.ev, |e| matches!(e, BackendEvent::SecretChat(c) if c.ttl_secs == Some(1))).await;
    let eph = ana.b.send_text(PEER, text("efímero", 3)).await.unwrap();
    assert_eq!(eph.ttl_secs, Some(1));
    let eph_b = received(&mut beto, "efímero").await;
    assert_eq!(eph_b.ttl_secs, Some(1));
    beto.b.mark_read(PEER, eph_b.id).await.unwrap();
    until(&mut beto.ev, |e| matches!(e, BackendEvent::MessagesDeleted { ids, .. } if ids == &vec![eph_b.id])).await;
    until(&mut ana.ev, |e| matches!(e, BackendEvent::MessagesDeleted { ids, .. } if ids == &vec![eph.id])).await;
    ana.b.set_secret_ttl(SecretChatId(CHAT), None).await.unwrap();

    // Borrado para ambos.
    let del = ana.b.send_text(PEER, text("bórrame", 4)).await.unwrap();
    let del_b = received(&mut beto, "bórrame").await;
    ana.b.delete_messages(PEER, vec![del.id], true).await.unwrap();
    until(&mut beto.ev, |e| matches!(e, BackendEvent::MessagesDeleted { ids, .. } if ids == &vec![del_b.id])).await;

    // Estado persistido (con instantánea) en el almacén inyectado.
    let rec = ana_store.get(SecretChatId(CHAT)).expect("guardado");
    assert_eq!((rec.state, rec.outgoing, rec.user), (SecretChatState::Accepted, true, UserId(BETO)));
    assert!(rec.session.is_some());

    // Cierre remoto.
    beto.b.discard_secret_chat(SecretChatId(CHAT)).await.unwrap();
    until(&mut ana.ev, |e| matches!(e, BackendEvent::SecretChat(c) if c.state == SecretChatState::Discarded)).await;
    assert!(matches!(ana.b.send_text(PEER, text("tarde", 5)).await, Err(BackendError::Forbidden(_))));
    assert_eq!(ana.b.secret_chats().await.unwrap()[0].state, SecretChatState::Discarded);
    assert!(ana_store.get(SecretChatId(CHAT)).unwrap().session.is_none(), "sin claves tras cerrar");
    // Lo que no es texto no está soportado en chats secretos.
    assert!(matches!(ana.b.send_reaction(PEER, sent.id, Some("👍".into())).await, Err(BackendError::Forbidden(_))));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_restores_state_and_rekey_and_resend_work_over_the_network() {
    let srv = Srv::default();
    let ana_store = Arc::new(MemorySecretStateStore::default());
    let mut ana = side(ANA, &srv, ana_store.clone()).await;
    let mut beto = side(BETO, &srv, Arc::new(MemorySecretStateStore::default())).await;
    handshake(&mut ana, &mut beto).await;
    ana.b.send_text(PEER, text("antes", 10)).await.unwrap();
    received(&mut beto, "antes").await;

    // «Reinicio» de Ana: backend nuevo con el mismo almacén.
    drop(ana);
    let mut ana = side(ANA, &srv, ana_store.clone()).await;
    let chats = ana.b.secret_chats().await.unwrap();
    assert_eq!((chats.len(), chats[0].state, chats[0].outgoing), (1, SecretChatState::Accepted, true));
    let m = ana.b.send_text(PEER, text("después", 11)).await.unwrap();
    assert!(m.id > MsgId(1), "los ids locales continúan tras reiniciar");
    received(&mut beto, "después").await;
    beto.b.send_text(PEER, text("recibido", 12)).await.unwrap();
    received(&mut ana, "recibido").await;

    // Un mensaje se pierde en el servidor: el siguiente abre un hueco y se pide Resend.
    srv.lock().unwrap().lose_next_from = Some(ANA);
    ana.b.send_text(PEER, text("perdido", 13)).await.unwrap();
    ana.b.send_text(PEER, text("siguiente", 14)).await.unwrap();
    received(&mut beto, "perdido").await;
    received(&mut beto, "siguiente").await;

    // Re-keying (PFS) automático pasados 100 mensajes con la misma clave.
    let old = fingerprint(&ana).await.unwrap();
    for i in 0..55 {
        ana.b.send_text(PEER, text(&format!("a{i}"), 100 + i)).await.unwrap();
        received(&mut beto, &format!("a{i}")).await;
        beto.b.send_text(PEER, text(&format!("b{i}"), 1000 + i)).await.unwrap();
        received(&mut ana, &format!("b{i}")).await;
    }
    let (fa, fb) = (fingerprint(&ana).await.unwrap(), fingerprint(&beto).await.unwrap());
    assert_ne!(fa, old, "la clave cambió");
    assert_eq!(fa, fb, "ambos usan la misma clave nueva");
    ana.b.send_text(PEER, text("con clave nueva", 9000)).await.unwrap();
    received(&mut beto, "con clave nueva").await;
    beto.b.send_text(PEER, text("ok", 9001)).await.unwrap();
    received(&mut ana, "ok").await;
    // La huella visual no cambia con el re-keying.
    assert_eq!(ana.b.secret_key_visual(SecretChatId(CHAT)).await.unwrap(), beto.b.secret_key_visual(SecretChatId(CHAT)).await.unwrap());
    let _ = ana.fake.names();
    assert!(ana_store.load_all().unwrap()[0].session.is_some());
}
