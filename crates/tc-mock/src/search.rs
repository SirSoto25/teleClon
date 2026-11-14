//! Búsqueda dentro de un chat o de un tema (ADR-0012) sobre los mensajes del mock.
use tc_core::*;

/// Aplica texto, filtro y paginación a `list` (mensajes ya limitados al chat o tema).
pub(crate) fn search<'a>(list: impl Iterator<Item = &'a Message>, q: &SearchQuery) -> SearchPage {
    if q.is_empty() {
        return SearchPage::default();
    }
    let has_text = q.text.split_whitespace().next().is_some();
    let mut all: Vec<&Message> = list
        .filter(|m| m.id.0 > 0 && q.filter.accepts(m) && (!has_text || text_matches(&q.text, &m.text)))
        .collect();
    all.sort_by_key(|m| std::cmp::Reverse(m.id));
    let total = all.len() as u32;
    let rest: Vec<&Message> = all.into_iter().filter(|m| q.offset.is_none_or(|o| m.id < o)).collect();
    let hits: Vec<Message> = rest.iter().take(q.limit).map(|m| (*m).clone()).collect();
    let next = (rest.len() > hits.len()).then(|| hits.last().map(|m| m.id)).flatten();
    SearchPage { hits, total: Some(total), next }
}

#[cfg(test)]
mod tests {
    use crate::{FORUM_GROUP, MockBackend, MockConfig};
    use tc_core::*;

    fn mock() -> MockBackend {
        MockBackend::new(MockConfig { now: Some(1_800_000_000), ..Default::default() })
    }

    fn peer() -> PeerRef {
        PeerRef::User(UserId(1012))
    }

    async fn say(b: &MockBackend, t: &str, rid: i64) -> Message {
        b.send_text(peer(), OutgoingText { text: t.into(), entities: vec![], reply_to: None, random_id: RandomId(rid) }).await.unwrap()
    }

    fn q(t: &str) -> SearchQuery {
        SearchQuery::new(t, 50)
    }

    #[tokio::test]
    async fn texto_acentos_mayusculas_prefijo_y_vacio() {
        let b = mock();
        let a = say(&b, "Quedamos en la CAFETERÍA zanahoria", 1).await;
        say(&b, "nos vemos luego", 2).await;
        for t in ["cafeteria", "CAFETERÍA", "cafe", "zanahoria cafe"] {
            let p = b.search_in_chat(peer(), q(t)).await.unwrap();
            assert_eq!(p.hits.iter().filter(|m| m.id == a.id).count(), 1, "{t}");
        }
        assert!(b.search_in_chat(peer(), q("xyzqw")).await.unwrap().hits.is_empty());
        assert_eq!(b.search_in_chat(peer(), q("xyzqw")).await.unwrap().total, Some(0));
        assert_eq!(b.search_in_chat(peer(), q("  ")).await.unwrap(), SearchPage::default());
    }

    #[tokio::test]
    async fn filtros_por_tipo() {
        let b = mock();
        let item = |kind, name: &str| OutgoingMediaItem { kind, path: "/tmp/x".into(), file_name: name.into(), size_bytes: 10, width: 4, height: 4 };
        let sent = b
            .send_media(
                peer(),
                OutgoingMedia {
                    items: vec![item(MediaKind::Photo, "a.jpg"), item(MediaKind::File, "b.pdf")],
                    caption: "adjunto zanahoria".into(),
                    reply_to: None,
                    random_ids: vec![RandomId(5), RandomId(6)],
                },
            )
            .await
            .unwrap();
        let link = say(&b, "mira https://example.org/zanahoria", 7).await;
        let ids = |p: SearchPage| p.hits.iter().map(|m| m.id).collect::<Vec<_>>();
        let photos = b.search_in_chat(peer(), SearchQuery::new("", 100).with_filter(SearchFilter::Photos)).await.unwrap();
        assert!(ids(photos.clone()).contains(&sent[0].id) && !ids(photos).contains(&sent[1].id));
        let files = b.search_in_chat(peer(), SearchQuery::new("", 100).with_filter(SearchFilter::Files)).await.unwrap();
        assert!(ids(files.clone()).contains(&sent[1].id) && !ids(files).contains(&sent[0].id));
        let links = b.search_in_chat(peer(), SearchQuery::new("zanahoria", 100).with_filter(SearchFilter::Links)).await.unwrap();
        assert_eq!(ids(links), [link.id]);
        let all = b.search_in_chat(peer(), q("zanahoria")).await.unwrap();
        assert_eq!(ids(all), [link.id, sent[0].id], "más reciente primero (la leyenda va en el primero)");
    }

    #[tokio::test]
    async fn paginacion_sin_repetir_y_borrado() {
        let b = mock();
        let mut sent = Vec::new();
        for i in 0..7 {
            sent.push(say(&b, &format!("riego zanahoria {i}"), 100 + i).await);
        }
        let mut seen = Vec::new();
        let mut query = SearchQuery::new("zanahoria", 3);
        loop {
            let p = b.search_in_chat(peer(), query.clone()).await.unwrap();
            assert_eq!(p.total, Some(7));
            seen.extend(p.hits.iter().map(|m| m.id));
            match p.next {
                Some(n) => query = query.after(n),
                None => break,
            }
        }
        let want: Vec<_> = sent.iter().rev().map(|m| m.id).collect();
        assert_eq!(seen, want);
        b.delete_messages(peer(), vec![sent[6].id], true).await.unwrap();
        let p = b.search_in_chat(peer(), SearchQuery::new("zanahoria", 50)).await.unwrap();
        assert_eq!((p.hits.len(), p.total), (6, Some(6)));
        assert!(p.hits.iter().all(|m| m.id != sent[6].id), "el borrado desaparece");
    }

    #[tokio::test]
    async fn tema_de_foro_y_scope() {
        let b = std::sync::Arc::new(mock());
        let topics = b.forum_topics(FORUM_GROUP, TopicsQuery::first(50)).await.unwrap().topics;
        let open: Vec<_> = topics.iter().filter(|t| !t.closed).collect();
        let (t1, t2) = (open[0].thread(), open[1].thread());
        let m1 = b.send_topic_text(t1, OutgoingText { text: "pelota roja".into(), entities: vec![], reply_to: None, random_id: RandomId(1) }).await.unwrap();
        let m2 = b.send_topic_text(t2, OutgoingText { text: "pelota azul".into(), entities: vec![], reply_to: None, random_id: RandomId(2) }).await.unwrap();
        let group = b.search_in_chat(PeerRef::Group(FORUM_GROUP), q("pelota")).await.unwrap();
        assert_eq!(group.hits.len(), 2);
        let in1 = b.search_in_topic(t1, q("pelota")).await.unwrap();
        assert_eq!(in1.hits.iter().map(|m| m.id).collect::<Vec<_>>(), [m1.id]);
        let scope = ThreadScope::new(b.clone(), t2, std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)));
        let via = scope.search_in_chat(PeerRef::Group(FORUM_GROUP), q("pelota")).await.unwrap();
        assert_eq!(via.hits.iter().map(|m| m.id).collect::<Vec<_>>(), [m2.id], "ThreadScope limita al tema");
        let other = scope.search_in_chat(peer(), q("zanahoria")).await.unwrap();
        assert!(other.hits.is_empty(), "otro chat pasa tal cual");
    }

    #[tokio::test]
    async fn sin_red_falla() {
        let b = mock();
        b.set_offline(true);
        assert_eq!(b.search_in_chat(peer(), q("hola")).await, Err(BackendError::Network));
    }
}
