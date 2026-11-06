//! TL, huella visual y temporizador.

use tc_secret::tl::{Action, DecryptedMessage, DecryptedMessageLayer, Entity, EntityKind, Media, PlainMessage, TopLevel, TypingAction};
use tc_secret::testing::hex;
use tc_secret::{KeyVisualization, SecretError, SecretKey, SelfDestructTimer};

fn layer(message: DecryptedMessage) -> TopLevel {
    TopLevel::Layer(DecryptedMessageLayer { random_bytes: vec![0xAB; 300], layer: 144, in_seq_no: 4, out_seq_no: 7, message })
}

#[test]
fn tl_round_trip_all_supported_constructors() {
    let full = PlainMessage {
        no_webpage: true,
        silent: true,
        random_id: -7,
        ttl: 30,
        message: "negrita, código y enlace".into(),
        media: Some(Media::Empty),
        entities: vec![
            Entity { kind: EntityKind::Bold, offset: 0, length: 7 },
            Entity { kind: EntityKind::Pre { language: "rust".into() }, offset: 9, length: 6 },
            Entity { kind: EntityKind::TextUrl { url: "https://example.org".into() }, offset: 18, length: 6 },
            Entity { kind: EntityKind::CustomEmoji { document_id: 99 }, offset: 1, length: 2 },
            Entity { kind: EntityKind::Spoiler, offset: 0, length: 1 },
            Entity { kind: EntityKind::Blockquote, offset: 0, length: 1 },
        ],
        via_bot_name: Some("bot".into()),
        reply_to_random_id: Some(5),
        grouped_id: Some(6),
    };
    let mut cases = vec![layer(DecryptedMessage::Message(full)), layer(DecryptedMessage::Message(PlainMessage::default()))];
    for action in [
        Action::SetMessageTtl { ttl_seconds: 5 },
        Action::ReadMessages { random_ids: vec![1, -2, i64::MAX] },
        Action::DeleteMessages { random_ids: vec![] },
        Action::ScreenshotMessages { random_ids: vec![3] },
        Action::FlushHistory,
        Action::Resend { start_seq_no: 2, end_seq_no: 8 },
        Action::NotifyLayer { layer: 144 },
        Action::Typing(TypingAction::Typing),
        Action::Typing(TypingAction::Cancel),
        Action::RequestKey { exchange_id: -1, g_a: vec![5; 256] },
        Action::AcceptKey { exchange_id: 2, g_b: vec![6; 256], key_fingerprint: -3 },
        Action::AbortKey { exchange_id: 4 },
        Action::CommitKey { exchange_id: 5, key_fingerprint: 6 },
        Action::Noop,
    ] {
        cases.push(layer(DecryptedMessage::Service { random_id: 11, action }));
    }
    cases.push(TopLevel::LegacyService { random_id: 1, random_bytes: vec![1; 15], action: Action::NotifyLayer { layer: 144 } });
    for c in cases {
        let bytes = c.to_bytes();
        assert_eq!(bytes.len() % 4, 0);
        assert_eq!(TopLevel::parse(&bytes).unwrap(), c);
        // Truncar en cualquier punto debe fallar sin pánico.
        for cut in [1, 4, bytes.len() / 2, bytes.len() - 1] {
            assert!(TopLevel::parse(&bytes[..cut]).is_err());
        }
    }
}

#[test]
fn tl_known_encoding() {
    // decryptedMessageService#73164160 random_id:long action:decryptedMessageActionNoop#a82fdd63
    let t = layer(DecryptedMessage::Service { random_id: 1, action: Action::Noop });
    let b = t.to_bytes();
    assert_eq!(&b[..4], &0x1be3_1789u32.to_le_bytes());
    assert_eq!(&b[4..8], &[254, 44, 1, 0], "bytes largos: 0xFE + longitud de 3 bytes (300)");
    let tail = &b[b.len() - 16..];
    assert_eq!(tail, [hex("60411673"), 1i64.to_le_bytes().to_vec(), hex("63dd2fa8")].concat().as_slice());
}

#[test]
fn tl_rejects_unknown_and_trailing() {
    let mut b = layer(DecryptedMessage::Service { random_id: 1, action: Action::Noop }).to_bytes();
    b.extend_from_slice(&[0; 4]);
    assert_eq!(TopLevel::parse(&b).unwrap_err(), SecretError::Tl("bytes sobrantes"));
    assert_eq!(TopLevel::parse(&0xdead_beefu32.to_le_bytes()).unwrap_err(), SecretError::UnsupportedConstructor(0xdead_beef));
}

#[test]
fn key_visualization_matches_independent_vector() {
    let mut k = [0u8; 256];
    k.copy_from_slice(&hex("5dfec997ee8e4d93857d88567c97f89960e4dd457151a8514dbdc0e8c2102fe395a3ae59d073938282d6ed7d5f1e90841c4ee6b5bdce3c028b8c505a8efaa1604cb24d8f1eaf4b3fb153c6eabc5d436d02b3eb972d943f565b4d23fa1853692f2b38da408af3f13d91cdef8887743b91b039085300e4639800b87664941be76642eba468c2da31b6672a9763df0a0224d571296a5970de1671ed0ba856a763cfeaa52ace6a517e22e818ff2de43be75a27ff0d68d22e26489937085abb6b40a8b3ad933cd0ac1c492ecdf30269a65fc9c498dbc86d279f821c9fcf881ff88595865a1a0f0382deecda1f7b0eccf45fd3127fea583e10ff993d58af00e6609341"));
    let key = SecretKey::from_bytes(k);
    let v = KeyVisualization::new(&key, &key);
    // SHA1(clave)[..16] || SHA256(clave)[..20], calculado en Python.
    assert_eq!(v.bytes().to_vec(), hex("8ab12754b5e0cdc8a78f84224f9734823fae67def02c1cd8723a2e057addc685228ba272"));
    assert_eq!(v.hex().split(' ').count(), 9);
    let g = v.grid();
    // 0x8a = 10 00 10 10
    assert_eq!(&g[0][..4], &[2, 0, 2, 2]);
    assert!(g.iter().flatten().all(|&c| c < 4));
    assert!(format!("{key:?}").contains("censurada"));
}

#[test]
fn timer_keeps_earliest_and_cancels() {
    let mut t = SelfDestructTimer::new();
    assert_eq!(t.start(1, 0, 100), None);
    assert_eq!(t.start(1, 10, 100), Some(110));
    assert_eq!(t.start(1, 10, 200), Some(110), "releer no alarga");
    assert_eq!(t.start(2, 5, 100), Some(105));
    assert_eq!(t.next_deadline(), Some(105));
    t.cancel(2);
    assert_eq!(t.next_deadline(), Some(110));
    assert_eq!(t.expire(200), vec![1]);
    assert!(t.is_empty());
    assert_eq!(tc_secret::ttl::effective_ttl(0, 7), 7);
    assert_eq!(tc_secret::ttl::effective_ttl(3, 7), 3);
}
