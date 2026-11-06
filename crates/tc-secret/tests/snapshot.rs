//! Instantáneas de sesión: guardar y restaurar el estado de un chat secreto en cada
//! fase (solicitado, entrante, listo, re-keying a medias, con cola y registro).

use std::sync::OnceLock;

use tc_secret::testing::{SeededEntropy, rfc3526_params};
use tc_secret::{DhParams, Event, Outbound, Output, SecretChat, SecretError, SessionSnapshot, StateKind};

const NOW: i64 = 1_800_000_000;

fn dh() -> DhParams {
    static DH: OnceLock<DhParams> = OnceLock::new();
    DH.get_or_init(rfc3526_params).clone()
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

/// Guarda y restaura (lo que haría la app al reiniciar).
fn reload(chat: &SecretChat) -> SecretChat {
    let snap = chat.snapshot();
    let bytes = snap.as_bytes().to_vec();
    let back = SessionSnapshot::from_bytes(&bytes).unwrap();
    assert_eq!(back.dh_p(), dh().p());
    assert_eq!(back.dh_g(), 2);
    SecretChat::restore(dh(), &back).unwrap()
}

fn send(c: &mut SecretChat, e: &mut SeededEntropy, text: &str) -> Vec<u8> {
    let (_, out) = c.send_text(text, e).unwrap();
    sends(&out).pop().unwrap()
}

#[test]
fn restore_in_every_phase_and_keep_talking() {
    let mut ea = SeededEntropy::new(b"snap/a");
    let mut eb = SeededEntropy::new(b"snap/b");
    let (a, out) = SecretChat::request(dh(), &mut ea, &[]).unwrap();
    let Outbound::RequestEncryption { g_a } = &out.outbound[0] else { panic!() };
    // Reinicio mientras se espera la aceptación: el exponente sobrevive.
    let mut a = reload(&a);
    assert_eq!(a.state_kind(), StateKind::Requested);

    let b = SecretChat::from_request(dh(), g_a).unwrap();
    let mut b = reload(&b);
    assert_eq!(b.state_kind(), StateKind::Incoming);
    let ob = b.accept(&mut eb, &[], NOW).unwrap();
    let Outbound::AcceptEncryption { g_b, key_fingerprint } = &ob.outbound[0] else { panic!() };
    let oa = a.on_accepted(g_b, *key_fingerprint, &mut ea, NOW).unwrap();
    assert_eq!(a.key_fingerprint(), Some(*key_fingerprint));
    for d in sends(&ob) {
        a.receive(&d, &mut ea, NOW).unwrap();
    }
    for d in sends(&oa) {
        b.receive(&d, &mut eb, NOW).unwrap();
    }

    for i in 0..5 {
        let d = send(&mut a, &mut ea, &format!("a{i}"));
        assert_eq!(texts(&b.receive(&d, &mut eb, NOW).unwrap().events), [format!("a{i}")]);
    }
    let o = b.set_ttl(30, &mut eb).unwrap();
    for d in sends(&o) {
        a.receive(&d, &mut ea, NOW).unwrap();
    }

    // Reinicio de ambos con el chat listo: contadores, capa, TTL y huella intactos.
    let (fp, visual, counters) = (a.key_fingerprint(), a.key_visualization().cloned(), a.seq_counters());
    let mut a = reload(&a);
    let mut b = reload(&b);
    assert_eq!((a.key_fingerprint(), a.key_visualization().cloned(), a.seq_counters()), (fp, visual, counters));
    assert_eq!((a.peer_layer(), a.ttl(), b.ttl()), (144, 30, 30));
    let d = send(&mut b, &mut eb, "tras reiniciar");
    assert_eq!(texts(&a.receive(&d, &mut ea, NOW).unwrap().events), ["tras reiniciar"]);

    // Hueco: B recibe el 2º antes que el 1º, se reinicia con el mensaje en cola, y
    // después A atiende el Resend con su registro de enviados restaurado.
    let d1 = send(&mut a, &mut ea, "uno");
    let d2 = send(&mut a, &mut ea, "dos");
    let o = b.receive(&d2, &mut eb, NOW).unwrap();
    assert!(o.events.contains(&Event::Queued { out_seq_no: 2 * 6 + 1 }));
    let resend = sends(&o);
    let mut b = reload(&b);
    assert_eq!(b.queued(), 1);
    let mut a = reload(&a);
    let o = a.receive(&resend[0], &mut ea, NOW).unwrap();
    let resent = sends(&o);
    assert_eq!(resent.len(), 1, "A reenvía el que faltaba desde el registro restaurado");
    assert_eq!(texts(&b.receive(&resent[0], &mut eb, NOW).unwrap().events), ["uno", "dos"]);
    // El original duplicado se descarta.
    assert!(texts(&b.receive(&d1, &mut eb, NOW).unwrap().events).is_empty());

    // Re-keying a medias: A pidió la clave nueva y se reinicia antes de la respuesta.
    let o = a.start_rekey(&mut ea).unwrap();
    let mut a = reload(&a);
    assert!(a.rekey_in_progress());
    let ob = b.receive(&sends(&o)[0], &mut eb, NOW).unwrap();
    let mut b = reload(&b);
    assert!(b.rekey_in_progress());
    let oa = a.receive(&sends(&ob)[0], &mut ea, NOW).unwrap();
    let new_fp = a.key_fingerprint().unwrap();
    assert_ne!(Some(new_fp), fp);
    let mut a = reload(&a);
    assert!(a.has_previous_key());
    let ob = b.receive(&sends(&oa)[0], &mut eb, NOW).unwrap();
    assert_eq!(b.key_fingerprint(), Some(new_fp));
    for d in sends(&ob) {
        a.receive(&d, &mut ea, NOW).unwrap();
    }
    let mut a = reload(&a);
    assert!(!a.has_previous_key() && !a.rekey_in_progress());
    let d = send(&mut a, &mut ea, "clave nueva");
    assert_eq!(texts(&b.receive(&d, &mut eb, NOW).unwrap().events), ["clave nueva"]);

    // Descartado: se restaura descartado.
    a.discard();
    assert_eq!(reload(&a).state_kind(), StateKind::Discarded);
}

#[test]
fn corrupt_or_foreign_snapshots_are_rejected() {
    let mut e = SeededEntropy::new(b"snap/bad");
    let (a, _) = SecretChat::request(dh(), &mut e, &[]).unwrap();
    let bytes = a.snapshot().as_bytes().to_vec();
    assert!(matches!(SessionSnapshot::from_bytes(&bytes[..10]), Err(SecretError::Malformed(_))));
    assert!(matches!(SessionSnapshot::from_bytes(b"XXXX"), Err(SecretError::Malformed(_))));
    let mut v2 = bytes.clone();
    v2[4] = 9;
    assert!(matches!(SessionSnapshot::from_bytes(&v2), Err(SecretError::Malformed(_))));
    // Truncada tras la cabecera o con bytes de sobra.
    let short = SessionSnapshot::from_bytes(&bytes[..bytes.len() - 1]).unwrap();
    assert!(matches!(SecretChat::restore(dh(), &short), Err(SecretError::Malformed(_))));
    let mut long = bytes.clone();
    long.push(0);
    let long = SessionSnapshot::from_bytes(&long).unwrap();
    assert!(matches!(SecretChat::restore(dh(), &long), Err(SecretError::Malformed(_))));
    // Otro p: rechazada.
    let mut other_p = bytes;
    other_p[9 + 4 + 100] ^= 1;
    let snap = SessionSnapshot::from_bytes(&other_p).unwrap();
    assert!(matches!(SecretChat::restore(dh(), &snap), Err(SecretError::InvalidState(_))));
    // El Debug no imprime el contenido.
    assert!(!format!("{snap:?}").contains("TCS1"));
}
