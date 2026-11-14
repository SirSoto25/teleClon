//! Búsqueda en un chat o tema (`messages.search`) contra el servidor falso: filtros, `top_msg_id`,
//! `offset_id`, ingesta de usuarios y paginación.
use std::sync::{Arc, Mutex};

use grammers_tl_types as tl;
use tc_core::*;

use super::fixtures::*;
use super::flows::restored;
use super::*;

const CH: i64 = 20;

fn slice(count: i32, messages: Vec<tl::enums::Message>, users: Vec<tl::enums::User>) -> Reply {
    ok(tl::enums::messages::Messages::from(tl::types::messages::ChannelMessages {
        inexact: false,
        pts: 1,
        count,
        offset_id_offset: None,
        messages,
        topics: vec![],
        chats: vec![channel(CH, "Huertos", false, 200)],
        users,
    }))
}

fn hit(id: i32, text: &str) -> tl::enums::Message {
    message(id, peer_channel(CH), text, false, Some(peer_user(11)), 1_000 + id).into()
}

fn peer() -> PeerRef {
    PeerRef::Group(crate::ids::channel_group(CH))
}

type Seen = Arc<Mutex<Vec<(String, u32, Option<i32>, i32, i32)>>>;

async fn setup(reply: impl Fn(&tl::functions::messages::Search) -> Reply + Send + 'static) -> (GrammersBackend, Arc<Fake>, Seen) {
    let seen: Seen = Arc::default();
    let s2 = seen.clone();
    let (b, fake, _) = restored(
        move |_, id, body| {
            assert_eq!(id, cid::<tl::functions::messages::Search>(), "llamada inesperada {}", tl::name_for_id(id));
            let req: tl::functions::messages::Search = decode(body);
            assert!(matches!(&req.peer, tl::enums::InputPeer::Channel(c) if c.channel_id == CH && c.access_hash == 200));
            s2.lock().unwrap().push((req.q.clone(), filter_id(&req.filter), req.top_msg_id, req.offset_id, req.limit));
            reply(&req)
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[channel(CH, "Huertos", false, 200)]);
    (b, fake, seen)
}

fn filter_id(f: &tl::enums::MessagesFilter) -> u32 {
    tl::Serializable::to_bytes(f)[..4].iter().rev().fold(0u32, |a, b| (a << 8) | u32::from(*b))
}

#[tokio::test]
async fn busca_en_el_chat_con_texto_offset_y_limite_e_ingiere_usuarios() {
    let (b, fake, seen) = setup(|_| slice(5, vec![hit(30, "riego de zanahorias"), hit(20, "zanahoria"), hit(30, "repetido")], vec![user_enum(11, "Beto", 110, false)])).await;
    let page = b.search_in_chat(peer(), SearchQuery::new("zanahoria", 3).after(MsgId(40))).await.unwrap();
    assert_eq!(page.hits.iter().map(|m| m.id.0).collect::<Vec<_>>(), [30, 20], "más reciente primero y sin repetir ids");
    assert_eq!((page.total, page.next), (Some(5), Some(MsgId(20))), "página llena (3 de 3): hay más");
    assert_eq!(page.hits[0].sender, Some(UserId(11)));
    assert!(b.inner.cache().peers.users.contains_key(&UserId(11)), "usuarios de la respuesta ingeridos");
    assert_eq!(seen.lock().unwrap()[0], ("zanahoria".into(), filter_id(&tl::types::InputMessagesFilterEmpty {}.into()), None, 40, 3));
    assert_eq!(fake.count("messages.search"), 1);
}

#[tokio::test]
async fn pagina_llena_da_next_y_la_ultima_no() {
    let (b, _, _) = setup(|req| {
        if req.offset_id == 0 {
            slice(3, vec![hit(30, "a"), hit(20, "a")], vec![])
        } else {
            slice(3, vec![hit(10, "a")], vec![])
        }
    })
    .await;
    let p1 = b.search_in_chat(peer(), SearchQuery::new("a", 2)).await.unwrap();
    assert_eq!((p1.hits.len(), p1.next, p1.total), (2, Some(MsgId(20)), Some(3)));
    let p2 = b.search_in_chat(peer(), SearchQuery::new("a", 2).after(p1.next.unwrap())).await.unwrap();
    assert_eq!((p2.hits.iter().map(|m| m.id.0).collect::<Vec<_>>(), p2.next), (vec![10], None));
}

#[tokio::test]
async fn cada_filtro_usa_su_input_messages_filter() {
    let (b, _, seen) = setup(|_| slice(0, vec![], vec![])).await;
    for f in [SearchFilter::All, SearchFilter::Photos, SearchFilter::Videos, SearchFilter::Files, SearchFilter::Links, SearchFilter::Voice] {
        let p = b.search_in_chat(peer(), SearchQuery::new("", 10).with_filter(f)).await;
        if f == SearchFilter::All {
            assert_eq!(p.unwrap(), SearchPage::default(), "sin texto ni filtro no hay nada que buscar");
        } else {
            assert_eq!(p.unwrap().total, Some(0));
        }
    }
    let ids: Vec<u32> = seen.lock().unwrap().iter().map(|s| s.1).collect();
    let want = [
        cid::<tl::types::InputMessagesFilterPhotos>(),
        cid::<tl::types::InputMessagesFilterVideo>(),
        cid::<tl::types::InputMessagesFilterDocument>(),
        cid::<tl::types::InputMessagesFilterUrl>(),
        cid::<tl::types::InputMessagesFilterVoice>(),
    ];
    assert_eq!(ids, want);
}

#[tokio::test]
async fn en_un_tema_va_top_msg_id_y_el_scope_lo_aplica() {
    let (b, _, seen) = setup(|_| slice(1, vec![hit(33, "tema")], vec![])).await;
    let th = ThreadRef { group: crate::ids::channel_group(CH), topic: MsgId(10) };
    let p = b.search_in_topic(th, SearchQuery::new("tema", 20)).await.unwrap();
    assert_eq!(p.hits.len(), 1);
    let scope = ThreadScope::new(Arc::new(b), th, Arc::new(std::sync::atomic::AtomicBool::new(true)));
    scope.search_in_chat(peer(), SearchQuery::new("tema", 20).with_filter(SearchFilter::Links)).await.unwrap();
    let s = seen.lock().unwrap();
    assert_eq!((s[0].2, s[1].2), (Some(10), Some(10)));
    assert_eq!(s[1].1, cid::<tl::types::InputMessagesFilterUrl>());
}

#[tokio::test]
async fn chat_secreto_no_soportado_y_error_de_red_se_propaga() {
    let (b, _, _) = setup(|_| rpc_err(500, "INTERDC_CALL_ERROR", None)).await;
    assert_eq!(b.search_in_chat(PeerRef::Secret(SecretChatId(1)), SearchQuery::new("x", 5)).await, Err(unsupported()));
    assert!(b.search_in_chat(peer(), SearchQuery::new("x", 5)).await.is_err());
}
