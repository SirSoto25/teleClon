//! Pruebas de extremo a extremo entre dos clientes simulados en el mismo proceso.
//! No hay red: los bytes que uno "envía" se entregan a mano al otro.

use std::sync::OnceLock;

use tc_secret::crypto::{self, Role};
use tc_secret::session::REKEY_AFTER_SECS;
use tc_secret::testing::{SeededEntropy, rfc3526_params};
use tc_secret::tl::{Action, DecryptedMessage, DecryptedMessageLayer, PlainMessage, TopLevel, TypingAction};
use tc_secret::{DhParams, DiscardReason, Event, Outbound, Output, SecretChat, SecretError, SelfDestructTimer, StateKind};

const NOW: i64 = 1_800_000_000;

fn dh() -> DhParams {
    static DH: OnceLock<DhParams> = OnceLock::new();
    DH.get_or_init(rfc3526_params).clone()
}

struct Side {
    chat: SecretChat,
    e: SeededEntropy,
}

fn sends(out: &Output) -> Vec<Vec<u8>> {
    out.outbound
        .iter()
        .filter_map(|o| match o {
            Outbound::SendEncrypted { data, .. } => Some(data.clone()),
            _ => None,
        })
        .collect()
}

fn texts(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Message(m) => Some(m.message.clone()),
            _ => None,
        })
        .collect()
}

/// Solicitud + aceptación + intercambio de NotifyLayer. Devuelve (iniciador, receptor).
fn handshake(seed: &[u8]) -> (Side, Side) {
    let mut ea = SeededEntropy::new(&[seed, b"/a"].concat());
    let mut eb = SeededEntropy::new(&[seed, b"/b"].concat());
    let (mut a, out) = SecretChat::request(dh(), &mut ea, &[]).unwrap();
    assert_eq!(a.state_kind(), StateKind::Requested);
    let Outbound::RequestEncryption { g_a } = &out.outbound[0] else { panic!("se esperaba requestEncryption") };

    let mut b = SecretChat::from_request(dh(), g_a).unwrap();
    assert_eq!(b.state_kind(), StateKind::Incoming);
    let out_b = b.accept(&mut eb, &[7u8; 256], NOW).unwrap();
    let Outbound::AcceptEncryption { g_b, key_fingerprint } = &out_b.outbound[0] else { panic!("acceptEncryption") };
    assert!(out_b.events.contains(&Event::Ready { fingerprint: *key_fingerprint }));

    let out_a = a.on_accepted(g_b, *key_fingerprint, &mut ea, NOW).unwrap();
    assert_eq!(a.state_kind(), StateKind::Ready);
    assert_eq!(a.key_fingerprint(), Some(*key_fingerprint));

    let mut a = Side { chat: a, e: ea };
    let mut b = Side { chat: b, e: eb };
    // NotifyLayer cruzados (capa 8, sin secuencia).
    for d in sends(&out_b) {
        let o = a.chat.receive(&d, &mut a.e, NOW).unwrap();
        assert!(o.events.contains(&Event::PeerLayer(144)));
    }
    for d in sends(&out_a) {
        b.chat.receive(&d, &mut b.e, NOW).unwrap();
    }
    assert_eq!(a.chat.peer_layer(), 144);
    assert_eq!(b.chat.peer_layer(), 144);
    assert_eq!(a.chat.seq_counters(), Some((0, 0)), "NotifyLayer no consume secuencia");
    (a, b)
}

/// Entrega todo lo pendiente en ambos sentidos hasta que no quede nada.
fn pump(a: &mut Side, b: &mut Side, mut to_a: Vec<Vec<u8>>, mut to_b: Vec<Vec<u8>>, now: i64) -> (Vec<Event>, Vec<Event>) {
    let (mut ev_a, mut ev_b) = (Vec::new(), Vec::new());
    for _ in 0..50 {
        if to_a.is_empty() && to_b.is_empty() {
            return (ev_a, ev_b);
        }
        let mut next_a = Vec::new();
        let mut next_b = Vec::new();
        for d in to_b.drain(..) {
            let o = b.chat.receive(&d, &mut b.e, now).unwrap();
            next_a.extend(sends(&o));
            ev_b.extend(o.events);
        }
        for d in to_a.drain(..) {
            let o = a.chat.receive(&d, &mut a.e, now).unwrap();
            next_b.extend(sends(&o));
            ev_a.extend(o.events);
        }
        to_a = next_a;
        to_b = next_b;
    }
    panic!("el intercambio no converge");
}

fn send(s: &mut Side, text: &str) -> Vec<u8> {
    let (_, out) = s.chat.send_text(text, &mut s.e).unwrap();
    let mut v = sends(&out);
    assert_eq!(v.len(), 1);
    v.pop().unwrap()
}

/// Descifra con la clave del chat y devuelve el `decryptedMessageLayer` (para mirar seq_no).
fn peek_layer(chat: &SecretChat, sender: Role, data: &[u8]) -> DecryptedMessageLayer {
    let plain = crypto::decrypt(chat.current_key().unwrap(), sender, data).unwrap();
    match TopLevel::parse(&plain).unwrap() {
        TopLevel::Layer(l) => l,
        other => panic!("se esperaba capa: {other:?}"),
    }
}

/// Fabrica un mensaje "del otro extremo" con números de secuencia arbitrarios.
fn forge(receiver: &SecretChat, sender_role: Role, in_seq_no: i32, out_seq_no: i32, message: DecryptedMessage, random_bytes: usize) -> Vec<u8> {
    let layer = DecryptedMessageLayer { random_bytes: vec![9; random_bytes], layer: 144, in_seq_no, out_seq_no, message };
    crypto::encrypt(receiver.current_key().unwrap(), sender_role, &layer.to_bytes(), &mut SeededEntropy::new(b"forge")).unwrap()
}

fn text_msg(id: i64, text: &str) -> DecryptedMessage {
    DecryptedMessage::Message(PlainMessage { random_id: id, message: text.into(), ..PlainMessage::default() })
}

#[test]
fn handshake_derives_same_key_fingerprint_and_visualization() {
    let (a, b) = handshake(b"hs");
    assert_eq!(a.chat.key_fingerprint(), b.chat.key_fingerprint());
    assert_eq!(a.chat.current_key().unwrap().as_bytes(), b.chat.current_key().unwrap().as_bytes());
    let (va, vb) = (a.chat.key_visualization().unwrap(), b.chat.key_visualization().unwrap());
    assert_eq!(va, vb);
    assert_eq!(va.emoji(), vb.emoji());
    assert_eq!(va.hex().len(), 72 + 8);
    // Otra sesión: clave distinta.
    let (c, _) = handshake(b"otro");
    assert_ne!(c.chat.key_fingerprint(), a.chat.key_fingerprint());
}

#[test]
fn round_trip_both_directions_with_seq_numbers() {
    let (mut a, mut b) = handshake(b"rt");
    let m1 = send(&mut a, "hola");
    let l1 = peek_layer(&b.chat, Role::Initiator, &m1);
    assert_eq!((l1.out_seq_no, l1.in_seq_no), (1, 0), "creador: out impar, in par");
    assert!(l1.random_bytes.len() >= 15);
    let o = b.chat.receive(&m1, &mut b.e, NOW).unwrap();
    assert_eq!(texts(&o.events), ["hola"]);

    let r1 = send(&mut b, "¿qué tal? 🦊");
    let lr = peek_layer(&a.chat, Role::Responder, &r1);
    assert_eq!((lr.out_seq_no, lr.in_seq_no), (0, 3), "receptor: out par, in impar (2·1+1)");
    let o = a.chat.receive(&r1, &mut a.e, NOW).unwrap();
    assert_eq!(texts(&o.events), ["¿qué tal? 🦊"]);

    for i in 0..20 {
        let d = send(&mut a, &format!("a{i}"));
        assert_eq!(texts(&b.chat.receive(&d, &mut b.e, NOW).unwrap().events), [format!("a{i}")]);
        let d = send(&mut b, &format!("b{i}"));
        assert_eq!(texts(&a.chat.receive(&d, &mut a.e, NOW).unwrap().events), [format!("b{i}")]);
    }
    assert_eq!(a.chat.seq_counters(), Some((21, 21)));
    assert_eq!(b.chat.seq_counters(), Some((21, 21)));
    // Mensaje largo (> 253 bytes: codificación TL larga) y vacío.
    let long = "ñ".repeat(3000);
    let d = send(&mut a, &long);
    assert_eq!(texts(&b.chat.receive(&d, &mut b.e, NOW).unwrap().events), [long]);
    let d = send(&mut a, "");
    assert_eq!(texts(&b.chat.receive(&d, &mut b.e, NOW).unwrap().events), [""]);
}

#[test]
fn tampered_message_is_rejected_and_state_unchanged() {
    let (mut a, mut b) = handshake(b"tamper");
    let m = send(&mut a, "secreto");
    for i in [8usize, 20, 24, 40, m.len() - 1] {
        let mut t = m.clone();
        t[i] ^= 0x80;
        assert_eq!(b.chat.receive(&t, &mut b.e, NOW).unwrap_err(), SecretError::MsgKeyMismatch, "byte {i}");
    }
    let mut t = m.clone();
    t[3] ^= 1;
    assert_eq!(b.chat.receive(&t, &mut b.e, NOW).unwrap_err(), SecretError::UnknownKeyFingerprint);
    assert_eq!(b.chat.seq_counters(), Some((0, 0)));
    assert_eq!(texts(&b.chat.receive(&m, &mut b.e, NOW).unwrap().events), ["secreto"]);
    // Reflejar a A su propio mensaje: dirección (x) equivocada → msg_key no cuadra.
    assert_eq!(a.chat.receive(&m, &mut a.e, NOW).unwrap_err(), SecretError::MsgKeyMismatch);
}

#[test]
fn invalid_g_b_discards_chat() {
    let p = tc_secret::testing::rfc3526_p();
    let mut p_minus_1 = p.clone();
    p_minus_1[255] -= 1;
    for bad in [vec![1u8], vec![2u8], p_minus_1, p, vec![0u8; 256]] {
        let mut e = SeededEntropy::new(b"gb");
        let (mut a, _) = SecretChat::request(dh(), &mut e, &[]).unwrap();
        let out = a.on_accepted(&bad, 1234, &mut e, NOW).unwrap();
        assert_eq!(a.state_kind(), StateKind::Discarded);
        assert_eq!(out.outbound, vec![Outbound::DiscardEncryption]);
        assert_eq!(out.events, vec![Event::Discarded(DiscardReason::InvalidPublicValue)]);
        assert!(a.send_text("x", &mut e).is_err());
    }
}

#[test]
fn invalid_g_a_is_refused_by_responder() {
    assert_eq!(SecretChat::from_request(dh(), &[1]).unwrap_err(), SecretError::InvalidPublicValue);
    assert_eq!(SecretChat::from_request(dh(), &[0xff; 256]).unwrap_err(), SecretError::InvalidPublicValue);
}

#[test]
fn fingerprint_mismatch_discards_chat() {
    let mut ea = SeededEntropy::new(b"fa");
    let mut eb = SeededEntropy::new(b"fb");
    let (mut a, out) = SecretChat::request(dh(), &mut ea, &[]).unwrap();
    let Outbound::RequestEncryption { g_a } = &out.outbound[0] else { panic!() };
    let mut b = SecretChat::from_request(dh(), g_a).unwrap();
    let out_b = b.accept(&mut eb, &[], NOW).unwrap();
    let Outbound::AcceptEncryption { g_b, key_fingerprint } = &out_b.outbound[0] else { panic!() };
    let out = a.on_accepted(g_b, key_fingerprint ^ 1, &mut ea, NOW).unwrap();
    assert_eq!(out.events, vec![Event::Discarded(DiscardReason::FingerprintMismatch)]);
    assert_eq!(a.state_kind(), StateKind::Discarded);
}

#[test]
fn duplicates_are_dropped() {
    let (mut a, mut b) = handshake(b"dup");
    let m = send(&mut a, "una vez");
    assert_eq!(texts(&b.chat.receive(&m, &mut b.e, NOW).unwrap().events), ["una vez"]);
    let o = b.chat.receive(&m, &mut b.e, NOW).unwrap();
    assert_eq!(o.events, vec![Event::Duplicate { out_seq_no: 1 }]);
    assert!(o.outbound.is_empty());
    assert_eq!(b.chat.seq_counters(), Some((0, 1)));
}

#[test]
fn gap_triggers_resend_and_is_filled_in_order() {
    let (mut a, mut b) = handshake(b"gap");
    let m1 = send(&mut b, "uno");
    let _lost = send(&mut b, "dos");
    let m3 = send(&mut b, "tres");
    assert_eq!(texts(&a.chat.receive(&m1, &mut a.e, NOW).unwrap().events), ["uno"]);
    let o = a.chat.receive(&m3, &mut a.e, NOW).unwrap();
    assert_eq!(o.events, vec![Event::Queued { out_seq_no: 4 }]);
    assert_eq!(a.chat.queued(), 1);
    let resend = sends(&o);
    assert_eq!(resend.len(), 1);
    let l = peek_layer(&b.chat, Role::Initiator, &resend[0]);
    assert!(matches!(l.message, DecryptedMessage::Service { action: Action::Resend { start_seq_no: 2, end_seq_no: 2 }, .. }));
    // B atiende el Resend reenviando "dos"; A lo procesa y vacía la cola.
    let (ev_a, _) = pump(&mut a, &mut b, vec![], resend, NOW);
    assert_eq!(texts(&ev_a), ["dos", "tres"]);
    assert_eq!(a.chat.queued(), 0);
    assert_eq!(a.chat.seq_counters().unwrap().1, 3);
    // Y la conversación sigue.
    let m4 = send(&mut b, "cuatro");
    assert_eq!(texts(&a.chat.receive(&m4, &mut a.e, NOW).unwrap().events), ["cuatro"]);
}

#[test]
fn reordering_without_loss_is_delivered_in_order() {
    let (mut a, mut b) = handshake(b"reorder");
    let ms: Vec<_> = (0..5).map(|i| send(&mut a, &format!("m{i}"))).collect();
    let mut got = Vec::new();
    for i in [0usize, 3, 1, 4, 2] {
        got.extend(texts(&b.chat.receive(&ms[i], &mut b.e, NOW).unwrap().events));
    }
    assert_eq!(got, ["m0", "m1", "m2", "m3", "m4"]);
    assert_eq!(b.chat.queued(), 0);
}

#[test]
fn wrong_parity_aborts_chat() {
    let (mut a, _b) = handshake(b"parity");
    // in_seq_no par desde el receptor: también paridad incorrecta.
    let (mut a2, _) = handshake(b"parity2");
    let d = forge(&a2.chat, Role::Responder, 0, 0, text_msg(5, "x"), 16);
    let o = a2.chat.receive(&d, &mut a2.e, NOW).unwrap();
    assert_eq!(o.events.last(), Some(&Event::Discarded(DiscardReason::SeqViolation)));
    // B (receptor) debe enviar out_seq_no par; uno impar es un reflejo/manipulación.
    let d = forge(&a.chat, Role::Responder, 1, 1, text_msg(5, "x"), 16);
    let o = a.chat.receive(&d, &mut a.e, NOW).unwrap();
    assert_eq!(o.events.last(), Some(&Event::Discarded(DiscardReason::SeqViolation)));
    assert_eq!(o.outbound, vec![Outbound::DiscardEncryption]);
    assert_eq!(a.chat.state_kind(), StateKind::Discarded);
}

#[test]
fn in_seq_from_the_future_aborts_chat() {
    let (mut a, _b) = handshake(b"future");
    // A no ha enviado nada, pero B dice haber recibido 3 mensajes suyos.
    let d = forge(&a.chat, Role::Responder, 2 * 3 + 1, 0, text_msg(5, "x"), 16);
    let o = a.chat.receive(&d, &mut a.e, NOW).unwrap();
    assert_eq!(o.events.last(), Some(&Event::Discarded(DiscardReason::SeqViolation)));
}

#[test]
fn decreasing_in_seq_aborts_chat() {
    let (mut a, mut b) = handshake(b"decr");
    for _ in 0..2 {
        let d = send(&mut a, "x");
        b.chat.receive(&d, &mut b.e, NOW).unwrap();
    }
    let d = send(&mut b, "ok"); // in_seq_no = 2·2+1 = 5
    a.chat.receive(&d, &mut a.e, NOW).unwrap();
    let d = forge(&a.chat, Role::Responder, 1, 2, text_msg(9, "retroceso"), 16);
    let o = a.chat.receive(&d, &mut a.e, NOW).unwrap();
    assert_eq!(o.events.last(), Some(&Event::Discarded(DiscardReason::SeqViolation)));
}

#[test]
fn too_few_random_bytes_is_ignored() {
    let (mut a, _b) = handshake(b"rb");
    let d = forge(&a.chat, Role::Responder, 1, 0, text_msg(5, "x"), 14);
    assert!(matches!(a.chat.receive(&d, &mut a.e, NOW), Err(SecretError::Malformed(_))));
    assert_eq!(a.chat.state_kind(), StateKind::Ready);
    let d = forge(&a.chat, Role::Responder, 1, 0, text_msg(5, "x"), 15);
    assert_eq!(texts(&a.chat.receive(&d, &mut a.e, NOW).unwrap().events), ["x"]);
}

#[test]
fn resend_request_for_unknown_messages_aborts() {
    let (mut a, _b) = handshake(b"resend-bad");
    // B pide reenviar out_seq_no 1..=5 de A, que nunca se enviaron.
    let action = Action::Resend { start_seq_no: 1, end_seq_no: 5 };
    let d = forge(&a.chat, Role::Responder, 1, 0, DecryptedMessage::Service { random_id: 3, action }, 16);
    let o = a.chat.receive(&d, &mut a.e, NOW).unwrap();
    assert_eq!(o.events.last(), Some(&Event::Discarded(DiscardReason::SeqViolation)));
}

#[test]
fn ttl_service_actions_and_self_destruct_timer() {
    let (mut a, mut b) = handshake(b"ttl");
    let o = a.chat.set_ttl(10, &mut a.e).unwrap();
    let o = b.chat.receive(&sends(&o)[0], &mut b.e, NOW).unwrap();
    assert_eq!(o.events, vec![Event::TtlChanged(10)]);
    assert_eq!((a.chat.ttl(), b.chat.ttl()), (10, 10));

    // El mensaje de B lleva ttl = 10; A lo lee en NOW+5 y arranca el temporizador.
    let (rid, o) = b.chat.send_text("se autodestruye", &mut b.e).unwrap();
    let ev = a.chat.receive(&sends(&o)[0], &mut a.e, NOW).unwrap().events;
    let Event::Message(m) = &ev[0] else { panic!() };
    assert_eq!((m.random_id, m.ttl), (rid, 10));
    let mut timer = SelfDestructTimer::new();
    assert_eq!(timer.start(rid, tc_secret::ttl::effective_ttl(m.ttl, a.chat.ttl()), NOW + 5), Some(NOW + 15));
    let o = a.chat.send_action(Action::ReadMessages { random_ids: vec![rid] }, &mut a.e).unwrap();
    let ev = b.chat.receive(&sends(&o)[0], &mut b.e, NOW + 5).unwrap().events;
    assert_eq!(ev, vec![Event::Read(vec![rid])]);
    assert!(timer.expire(NOW + 14).is_empty());
    assert_eq!(timer.expire(NOW + 15), vec![rid]);

    // Resto de acciones de servicio.
    for (action, expected) in [
        (Action::DeleteMessages { random_ids: vec![1, 2] }, Event::Deleted(vec![1, 2])),
        (Action::ScreenshotMessages { random_ids: vec![3] }, Event::Screenshot(vec![3])),
        (Action::FlushHistory, Event::HistoryFlushed),
        (Action::Typing(TypingAction::Typing), Event::Typing(TypingAction::Typing)),
    ] {
        let o = b.chat.send_action(action, &mut b.e).unwrap();
        assert_eq!(a.chat.receive(&sends(&o)[0], &mut a.e, NOW).unwrap().events, vec![expected]);
    }
    // Las acciones internas del protocolo no se pueden mandar a mano.
    assert!(a.chat.send_action(Action::Resend { start_seq_no: 0, end_seq_no: 0 }, &mut a.e).is_err());
    assert!(a.chat.send_action(Action::AbortKey { exchange_id: 1 }, &mut a.e).is_err());
}

fn chat_n(a: &mut Side, b: &mut Side, n: usize) {
    for i in 0..n {
        let d = send(a, &format!("a{i}"));
        b.chat.receive(&d, &mut b.e, NOW).unwrap();
        let d = send(b, &format!("b{i}"));
        a.chat.receive(&d, &mut a.e, NOW).unwrap();
    }
}

#[test]
fn rekey_after_100_messages_switches_both_sides() {
    let (mut a, mut b) = handshake(b"rekey");
    let old_fp = a.chat.key_fingerprint().unwrap();
    let visual = a.chat.key_visualization().unwrap().clone();
    assert!(!a.chat.should_rekey(NOW));
    chat_n(&mut a, &mut b, 51);
    assert!(a.chat.should_rekey(NOW) && b.chat.should_rekey(NOW));

    let o = a.chat.start_rekey(&mut a.e).unwrap();
    assert!(a.chat.rekey_in_progress());
    // RequestKey → B responde AcceptKey.
    let ob = b.chat.receive(&sends(&o)[0], &mut b.e, NOW).unwrap();
    assert!(b.chat.rekey_in_progress());
    // Antes del CommitKey ambos siguen con la clave vieja.
    assert_eq!(b.chat.key_fingerprint(), Some(old_fp));
    // AcceptKey → A envía CommitKey (clave vieja) y cambia.
    let oa = a.chat.receive(&sends(&ob)[0], &mut a.e, NOW).unwrap();
    let new_fp = a.chat.key_fingerprint().unwrap();
    assert_ne!(new_fp, old_fp);
    assert!(oa.events.contains(&Event::KeyChanged { fingerprint: new_fp }));
    assert!(a.chat.has_previous_key(), "A guarda la vieja hasta recibir algo con la nueva");
    // A ya cifra con la nueva clave.
    let after = send(&mut a, "con clave nueva");
    assert_eq!(crypto::peek_fingerprint(&after).unwrap(), new_fp);
    // CommitKey → B cambia y manda Noop con la nueva.
    let ob = b.chat.receive(&sends(&oa)[0], &mut b.e, NOW).unwrap();
    assert!(ob.events.contains(&Event::KeyChanged { fingerprint: new_fp }));
    assert_eq!(b.chat.key_fingerprint(), Some(new_fp));
    assert!(!b.chat.has_previous_key(), "B sin huecos descarta la vieja");
    let noop = sends(&ob);
    assert_eq!(crypto::peek_fingerprint(&noop[0]).unwrap(), new_fp);
    assert_eq!(texts(&b.chat.receive(&after, &mut b.e, NOW).unwrap().events), ["con clave nueva"]);
    // Noop → A descarta la vieja.
    a.chat.receive(&noop[0], &mut a.e, NOW).unwrap();
    assert!(!a.chat.has_previous_key());
    assert!(!a.chat.rekey_in_progress() && !b.chat.rekey_in_progress());
    assert_eq!(a.chat.current_key().unwrap().as_bytes(), b.chat.current_key().unwrap().as_bytes());
    assert_eq!(a.chat.key_visualization().unwrap(), &visual, "la huella visual no cambia con el re-keying");
    assert!(!a.chat.should_rekey(NOW));
    chat_n(&mut a, &mut b, 3);
}

#[test]
fn rekey_survives_messages_in_flight_and_lost_commit_order() {
    let (mut a, mut b) = handshake(b"rekey-flight");
    chat_n(&mut a, &mut b, 2);
    let o = a.chat.start_rekey(&mut a.e).unwrap();
    let ob = b.chat.receive(&sends(&o)[0], &mut b.e, NOW).unwrap();
    let old_from_b = send(&mut b, "viejo en vuelo"); // B aún usa la clave vieja
    let oa = a.chat.receive(&sends(&ob)[0], &mut a.e, NOW).unwrap();
    let commit = sends(&oa);
    let new_from_a = send(&mut a, "nuevo");
    // A recibe el mensaje de B con la clave vieja: debe descifrarlo con la anterior.
    assert_eq!(texts(&a.chat.receive(&old_from_b, &mut a.e, NOW).unwrap().events), ["viejo en vuelo"]);
    // B recibe primero el mensaje nuevo (CommitKey retrasado): cambia de clave por la huella.
    let o1 = b.chat.receive(&new_from_a, &mut b.e, NOW).unwrap();
    assert!(o1.events.iter().any(|e| matches!(e, Event::KeyChanged { .. })));
    assert!(o1.events.iter().any(|e| matches!(e, Event::Queued { .. })));
    // Llega el CommitKey: se llena el hueco y sale el mensaje en orden.
    let o2 = b.chat.receive(&commit[0], &mut b.e, NOW).unwrap();
    assert_eq!(texts(&o2.events), ["nuevo"]);
    let mut to_a = sends(&o1);
    to_a.extend(sends(&o2));
    pump(&mut a, &mut b, to_a, vec![], NOW);
    assert_eq!(a.chat.current_key().unwrap().as_bytes(), b.chat.current_key().unwrap().as_bytes());
    assert!(!a.chat.has_previous_key() && !b.chat.has_previous_key());
    chat_n(&mut a, &mut b, 2);
}

#[test]
fn concurrent_rekey_converges_on_one_instance() {
    for seed in [b"c1".as_slice(), b"c2", b"c3", b"c4"] {
        let (mut a, mut b) = handshake(seed);
        chat_n(&mut a, &mut b, 1);
        let old = a.chat.key_fingerprint();
        let oa = a.chat.start_rekey(&mut a.e).unwrap();
        let ob = b.chat.start_rekey(&mut b.e).unwrap();
        pump(&mut a, &mut b, sends(&ob), sends(&oa), NOW);
        assert_ne!(a.chat.key_fingerprint(), old);
        assert_eq!(a.chat.key_fingerprint(), b.chat.key_fingerprint());
        assert!(!a.chat.rekey_in_progress() && !b.chat.rekey_in_progress());
        assert_eq!(a.chat.state_kind(), StateKind::Ready);
        chat_n(&mut a, &mut b, 2);
    }
}

#[test]
fn rekey_with_invalid_g_a_is_aborted() {
    let (mut a, mut b) = handshake(b"rekey-bad");
    let old = a.chat.key_fingerprint();
    let action = Action::RequestKey { exchange_id: 42, g_a: vec![1] };
    let d = forge(&a.chat, Role::Responder, 1, 0, DecryptedMessage::Service { random_id: 8, action }, 16);
    let o = a.chat.receive(&d, &mut a.e, NOW).unwrap();
    assert!(o.events.contains(&Event::RekeyAborted));
    let l = peek_layer(&b.chat, Role::Initiator, &sends(&o)[0]);
    assert!(matches!(l.message, DecryptedMessage::Service { action: Action::AbortKey { exchange_id: 42 }, .. }));
    assert_eq!(a.chat.key_fingerprint(), old);
    assert_eq!(a.chat.state_kind(), StateKind::Ready);
    let _ = &mut b;
}

#[test]
fn rekey_due_after_a_week() {
    let (mut a, mut b) = handshake(b"week");
    // NotifyLayer ya cuenta como mensaje cifrado con la clave.
    chat_n(&mut a, &mut b, 1);
    assert!(!a.chat.should_rekey(NOW + REKEY_AFTER_SECS));
    assert!(a.chat.should_rekey(NOW + REKEY_AFTER_SECS + 1));
}

#[test]
fn discard_local_and_remote() {
    let (mut a, mut b) = handshake(b"discard");
    let o = a.chat.discard();
    assert_eq!(o.outbound, vec![Outbound::DiscardEncryption]);
    assert_eq!(a.chat.state_kind(), StateKind::Discarded);
    assert!(a.chat.current_key().is_none());
    assert!(a.chat.discard().outbound.is_empty(), "idempotente");
    let o = b.chat.on_remote_discarded();
    assert!(o.outbound.is_empty());
    assert_eq!(o.events, vec![Event::Discarded(DiscardReason::Remote)]);
    assert!(matches!(b.chat.send_text("x", &mut b.e), Err(SecretError::InvalidState(_))));
}

#[test]
fn os_entropy_end_to_end() {
    let mut ea = tc_secret::OsEntropy;
    let mut eb = tc_secret::OsEntropy;
    let (mut a, out) = SecretChat::request(dh(), &mut ea, &[]).unwrap();
    let Outbound::RequestEncryption { g_a } = &out.outbound[0] else { panic!() };
    let mut b = SecretChat::from_request(dh(), g_a).unwrap();
    let ob = b.accept(&mut eb, &[], NOW).unwrap();
    let Outbound::AcceptEncryption { g_b, key_fingerprint } = &ob.outbound[0] else { panic!() };
    a.on_accepted(g_b, *key_fingerprint, &mut ea, NOW).unwrap();
    let (_, o) = a.send_text("con getrandom", &mut ea).unwrap();
    assert_eq!(texts(&b.receive(&sends(&o)[0], &mut eb, NOW).unwrap().events), ["con getrandom"]);
}
