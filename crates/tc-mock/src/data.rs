//! Datos de muestra originales en español, generados de forma determinista.
use std::collections::{BTreeMap, HashMap};

use tc_core::*;

use crate::rng::Lcg;

pub const ME: UserId = UserId(1000);
/// Chat con el historial largo para probar el scroll virtual.
pub const BIG_CHAT: PeerRef = PeerRef::User(UserId(1001));
pub const BIG_CHAT_LEN: usize = 5_000;
pub const SECRET_ID: SecretChatId = SecretChatId(1);

pub struct Seed {
    pub me: User,
    pub users: BTreeMap<UserId, User>,
    pub groups: BTreeMap<GroupId, Group>,
    pub secrets: BTreeMap<SecretChatId, SecretChat>,
    pub dialogs: HashMap<PeerRef, Dialog>,
    pub messages: HashMap<PeerRef, Vec<Message>>,
    pub sets: Vec<StickerSet>,
}

const PEOPLE: &[(&str, &str, &str)] = &[
    ("Lucía", "Barrientos", "lucia_b"),
    ("Mateo", "Quiroga", "mquiroga"),
    ("Itziar", "Olabarri", "itziar_o"),
    ("Dario", "Fontanals", "dfontanals"),
    ("Xiomara", "Peñalver", "xio_p"),
    ("Anselmo", "Rueda", "anselmo_r"),
    ("Nerea", "Castañeda", "nerea_c"),
    ("Joaquín", "Villalta", "jvillalta"),
    ("Abril", "Montoya", "abril_m"),
    ("Íñigo", "Santaolalla", "inigo_s"),
    ("Pilar", "Echeverría", "pilar_e"),
    ("Tadeo", "Lorenzana", "tadeo_l"),
    ("Sofía", "Arrieta", "sofia_a"),
    ("Casimiro", "Bueno", "casi_bueno"),
    // Nombre con letras «de estilo» Unicode, como los que llegan de la red real (prueba del respaldo de fuentes).
    ("𝒞𝓇𝒾𝓈𝓉𝒾𝓃𝒶", "Iglesias", "cris_iglesias"),
    ("Octavio", "Pereda", "octavio_p"),
    ("Yaiza", "Cabrera", "yaiza_c"),
    ("Bruno", "Salcedo", "bruno_s"),
    ("Elvira", "Maldonado", "elvira_m"),
    ("Rodrigo", "Tenorio", "rodrigo_t"),
];

const GROUPS: &[(&str, GroupKind, u32)] = &[
    ("Cena de los viernes", GroupKind::Basic, 6),
    ("Piso Calle Ancha", GroupKind::Basic, 4),
    ("Senderismo Sierra Norte", GroupKind::Super, 48),
    ("Club de lectura «Tinta y Sal»", GroupKind::Super, 23),
    ("Familia Barrientos-Quiroga", GroupKind::Basic, 11),
    ("Boletín del Huerto Urbano", GroupKind::Channel, 1204),
    ("Ofertas de bicicletas usadas", GroupKind::Channel, 876),
    ("Noticias del barrio", GroupKind::Channel, 3310),
];

const PHRASES: &[&str] = &[
    "Buenos días, ¿has dormido algo?",
    "Te llamo en diez minutos, que ahora estoy en el tren.",
    "Ya he pasado por la panadería, queda una barra de masa madre.",
    "Mañana a las ocho en la plaza, no lleguéis tarde.",
    "Jajaja, no me lo puedo creer.",
    "Mándame la dirección otra vez, que se me ha perdido.",
    "Acabo de ver el correo; lo reviso esta tarde y te cuento.",
    "¿Quedamos el sábado para ir al mercadillo?",
    "Perfecto, así lo dejamos.",
    "Se me ha olvidado el paraguas y está cayendo de todo.",
    "He subido las fotos de la excursión, mirad cuáles os gustan.",
    "Gracias por el favor, te debo una cerveza.",
    "Me han cambiado el turno, el jueves no puedo.",
    "¿Alguien tiene el número del fontanero?",
    "Hoy el café ha sabido a gloria.",
    "Vale, pero que conste que yo avisé.",
    "El tren llega con veinte minutos de retraso, otra vez.",
    "Pásate cuando quieras, la puerta está abierta.",
    "Estoy leyendo un libro buenísimo, luego te lo presto.",
    "No encuentro las llaves por ningún lado, ¿las has visto?",
    "Esta noche toca lentejas, ¿te apuntas?",
    "Feliz cumpleaños, que cumplas muchos más. Un abrazo enorme.",
    "Os paso el borrador del presupuesto para que lo miréis con calma.",
    "Ojalá fuera viernes ya.",
    "Qué bonito se ve el atardecer desde la azotea.",
    "Dame un segundo, que busco el enlace.",
    "Luego te cuento, es una historia larga.",
    "Sí, sí, yo me encargo de reservar la mesa.",
];

const LONG_PHRASE: &str = "Te resumo lo del fin de semana: salimos temprano, paramos a desayunar en el pueblo de la cuesta, \
subimos hasta el mirador por el sendero viejo, comimos bocadillos con vistas al valle y volvimos al anochecer, \
cansados pero contentos. Si os apetece repetir en primavera, lo organizo yo y reservo el albergue con tiempo.";

const CHANNEL_POSTS: &[&str] = &[
    "Este sábado reparto de plantones de tomate y albahaca en la entrada del huerto.",
    "Recordatorio: el compostaje comunitario se vacía los martes por la mañana.",
    "Nueva hornada de bicicletas de paseo, revisadas y con luces. Consultad precios por privado.",
    "Corte de tráfico en la calle Mayor por obras hasta el viernes.",
    "El mercadillo de trueque vuelve el próximo domingo en la plaza del reloj.",
    "Abierto el plazo para apuntarse a las parcelas de otoño.",
];

const REACTIONS: &[&str] = &["👍", "❤️", "😂", "🎉", "🙏", "🔥"];


pub fn build(seed: u64, now: i64) -> Seed {
    let mut rng = Lcg::new(seed);

    let me = User {
        id: ME,
        first_name: "Tú".into(),
        last_name: String::new(),
        username: Some("yo_mismo".into()),
        status: UserStatus::Online,
        is_contact: false,
        is_bot: false,
        photo: crate::images::profile_photo(ME.0),
    };
    let mut users = BTreeMap::new();
    for (i, (first, last, username)) in PEOPLE.iter().enumerate() {
        let id = UserId(1001 + i as i64);
        let status = match i % 4 {
            0 => UserStatus::Online,
            1 => UserStatus::LastSeen(Timestamp(now - 900 * (i as i64 + 1))),
            2 => UserStatus::Recently,
            _ => UserStatus::Offline,
        };
        users.insert(
            id,
            User {
                id,
                first_name: (*first).into(),
                last_name: (*last).into(),
                username: Some((*username).into()),
                status,
                is_contact: true,
                is_bot: false,
                photo: crate::images::profile_photo(id.0),
            },
        );
    }
    let mut groups = BTreeMap::new();
    for (i, (title, kind, members)) in GROUPS.iter().enumerate() {
        let id = GroupId(5000 + i as i64);
        groups.insert(
            id,
            Group { id, kind: *kind, title: (*title).into(), username: None, member_count: *members, photo: crate::images::profile_photo(id.0) },
        );
    }

    let sets = vec![sticker_set(), animated_sticker_set()];

    let mut messages: HashMap<PeerRef, Vec<Message>> = HashMap::new();
    let mut dialogs: HashMap<PeerRef, Dialog> = HashMap::new();
    let user_ids: Vec<UserId> = users.keys().copied().collect();

    // Chats privados: el primero (BIG_CHAT) tiene el historial largo.
    for (i, uid) in user_ids.iter().enumerate() {
        let peer = PeerRef::User(*uid);
        let n = if peer == BIG_CHAT { BIG_CHAT_LEN } else { 12 + rng.below(90) };
        // Los 8 primeros más recientes, el resto más antiguos.
        let last_at = now - (i as i64) * 5_400 - rng.below(1800) as i64;
        let list = gen_history(&mut rng, peer, n, last_at, &user_ids, &sets);
        messages.insert(peer, list);
    }
    // Grupos y canales.
    let group_ids: Vec<GroupId> = groups.keys().copied().collect();
    for (i, gid) in group_ids.iter().enumerate() {
        let peer = PeerRef::Group(*gid);
        let is_channel = groups[gid].kind == GroupKind::Channel;
        let n = 20 + rng.below(160);
        let last_at = now - (i as i64 + 2) * 7_000 - rng.below(3000) as i64;
        let list = if is_channel {
            gen_channel(&mut rng, peer, n.min(60), last_at)
        } else {
            gen_history(&mut rng, peer, n, last_at, &user_ids, &sets)
        };
        messages.insert(peer, list);
    }

    // Chat secreto simulado con una persona concreta.
    let secret_user = user_ids[2];
    let secret = SecretChat {
        id: SECRET_ID,
        user: secret_user,
        state: SecretChatState::Accepted,
        outgoing: true,
        ttl_secs: Some(60),
    };
    let speer = PeerRef::Secret(SECRET_ID);
    let mut sh = gen_history(&mut rng, speer, 8, now - 600, &[secret_user], &sets);
    for m in &mut sh {
        m.ttl_secs = Some(60);
    }
    messages.insert(speer, sh);
    let mut secrets = BTreeMap::new();
    secrets.insert(SECRET_ID, secret);

    // Diálogos derivados del historial.
    let title_of = |peer: PeerRef| -> String {
        match peer {
            PeerRef::User(u) => users[&u].display_name(),
            PeerRef::Group(g) => groups[&g].title.clone(),
            PeerRef::Secret(s) => users[&secrets[&s].user].display_name(),
        }
    };
    let photo_of = |peer: PeerRef| match peer {
        PeerRef::User(u) => users[&u].photo,
        PeerRef::Group(g) => groups[&g].photo,
        PeerRef::Secret(s) => users[&secrets[&s].user].photo,
    };
    let mut peers: Vec<PeerRef> = messages.keys().copied().collect();
    peers.sort_by_key(|p| format!("{p:?}"));
    for (k, peer) in peers.iter().enumerate() {
        let list = &messages[peer];
        let last = list.last().cloned();
        let tail_incoming = list.iter().rev().take_while(|m| !m.outgoing).count() as u32;
        let unread = if *peer == BIG_CHAT { 0 } else if rng.chance(45) { tail_incoming.min(1 + rng.below(6) as u32) } else { 0 };
        let activity = last.as_ref().map_or(Timestamp(0), |m| m.date);
        let read_out = list.iter().rev().find(|m| m.outgoing).map_or(MsgId(0), |m| m.id);
        dialogs.insert(
            *peer,
            Dialog {
                peer: *peer,
                title: title_of(*peer),
                last_message: last,
                unread_count: unread,
                unread_mentions: 0,
                pinned_order: None,
                archived: k % 11 == 10,
                muted: matches!(peer, PeerRef::Group(g) if groups[g].kind == GroupKind::Channel) || k % 7 == 3,
                draft: None,
                activity,
                read_outbox_max_id: read_out,
                photo: photo_of(*peer),
            },
        );
    }
    // Tres fijados y un borrador.
    for (order, peer) in [BIG_CHAT, PeerRef::Group(group_ids[0]), PeerRef::Secret(SECRET_ID)].iter().enumerate() {
        if let Some(d) = dialogs.get_mut(peer) {
            d.pinned_order = Some(order as u32);
            d.archived = false;
        }
    }
    if let Some(d) = dialogs.get_mut(&PeerRef::User(user_ids[4])) {
        d.draft = Some(Draft { text: "Oye, una cosa sobre el sábado…".into(), reply_to: None });
    }

    Seed { me, users, groups, secrets, dialogs, messages, sets }
}

/// Pack propio «Brasa básicos» de `tc-stickers` (12 stickers estáticos originales).
pub fn sticker_set() -> StickerSet {
    tc_stickers::pack::sample_set()
}

/// Set de la variante animada del pack propio. Convenio compartido con `tc-ui`
/// (`tc_ui::stickers::ANIMATED_SET_ID`): id del set 9 000 002 y sticker `i` = `9 000 002 × 100 + i`,
/// con el mismo dibujo que el sticker `i` del pack estático. Se marca `Lottie` porque es animado;
/// el mock no trae bytes Lottie: la UI anima el dibujo propio por código.
pub const ANIMATED_SET_ID: StickerSetId = StickerSetId(9_000_002);

pub fn animated_sticker_set() -> StickerSet {
    let base = tc_stickers::pack::sample_set();
    StickerSet {
        id: ANIMATED_SET_ID,
        short_name: "brasa_en_movimiento".into(),
        title: "Brasa en movimiento".into(),
        stickers: base
            .stickers
            .into_iter()
            .enumerate()
            .map(|(i, s)| Sticker { id: StickerId(ANIMATED_SET_ID.0 * 100 + i as i64), set_id: ANIMATED_SET_ID, format: StickerFormat::Lottie, ..s })
            .collect(),
    }
}

fn gen_history(
    rng: &mut Lcg,
    peer: PeerRef,
    n: usize,
    last_at: i64,
    senders: &[UserId],
    sets: &[StickerSet],
) -> Vec<Message> {
    let mut out: Vec<Message> = Vec::with_capacity(n);
    // Se genera hacia atrás en el tiempo y luego se invierte, para que el último caiga en `last_at`.
    let mut t = last_at;
    let mut times = Vec::with_capacity(n);
    for _ in 0..n {
        times.push(t);
        t -= 20 + rng.below(1500) as i64;
    }
    times.reverse();
    let one_sender = match peer {
        PeerRef::User(u) => Some(u),
        PeerRef::Secret(_) => senders.first().copied(),
        PeerRef::Group(_) => None,
    };
    for (i, date) in times.into_iter().enumerate() {
        let id = MsgId(i as i32 + 1);
        let outgoing = rng.chance(42);
        let sender = if outgoing {
            Some(ME)
        } else {
            Some(one_sender.unwrap_or_else(|| *rng.pick(senders)))
        };
        let mut m = Message {
            id,
            peer,
            sender,
            date: Timestamp(date),
            edit_date: None,
            outgoing,
            text: String::new(),
            entities: vec![],
            reply_to: None,
            forwarded_from: None,
            media: None,
            sticker: None,
            reactions: vec![],
            random_id: None,
            state: SendState::Sent,
            ttl_secs: None,
        };
        let roll = rng.below(100);
        if roll < 4 {
            // Mismo número de tiradas que con un solo set: uno de cada tres sale de la variante animada.
            let k = rng.below(sets[0].stickers.len());
            let set = if k.is_multiple_of(3) && sets.len() > 1 { &sets[1] } else { &sets[0] };
            m.sticker = Some(set.stickers[k.min(set.stickers.len() - 1)].clone());
        } else if roll < 9 {
            let id = 70_000 + i as i64;
            let size_bytes = 80_000 + rng.below(900_000) as u64;
            m.media = Some(Media {
                id,
                kind: MediaKind::Photo,
                file_name: None,
                size_bytes,
                width: *rng.pick(&[1280, 1080, 960]),
                height: *rng.pick(&[720, 1080, 1280]),
                duration_secs: None,
                local_path: None,
                file: Some(crate::images::file_ref(MediaKind::Photo, id, size_bytes)),
                audio: None,
            });
            if rng.chance(50) {
                m.text = rng.pick(PHRASES).to_string();
            }
        } else if roll < 12 {
            m.text = LONG_PHRASE.to_string();
        } else if roll < 14 {
            let id = 80_000 + i as i64;
            let file_name = Some(rng.pick(&["presupuesto.pdf", "ruta_sierra.gpx", "apuntes_tema3.odt"]).to_string());
            let size_bytes = 20_000 + rng.below(4_000_000) as u64;
            m.media = Some(Media {
                id,
                kind: MediaKind::File,
                file_name,
                size_bytes,
                width: 0,
                height: 0,
                duration_secs: None,
                local_path: None,
                file: Some(crate::images::file_ref(MediaKind::File, id, size_bytes)),
                audio: None,
            });
            // La mitad son notas de voz (sin tiradas extra: el resto de datos no cambia).
            if i % 2 == 0
                && let Some(md) = m.media.as_mut()
            {
                let secs = 3 + (size_bytes % 40) as u32;
                md.kind = MediaKind::Voice;
                md.file_name = None;
                md.duration_secs = Some(secs);
                md.size_bytes = crate::audio::wav_len(secs);
                md.file = Some(crate::images::file_ref(MediaKind::Voice, id, md.size_bytes));
                md.audio = Some(AudioInfo { voice: true, waveform: crate::audio::waveform(id), mime: Some("audio/wav".into()), ..Default::default() });
            }
        } else {
            m.text = rng.pick(PHRASES).to_string();
            if m.text.contains("enlace") {
                // Entidad de ejemplo: negrita sobre la primera palabra.
                m.entities.push(TextEntity { kind: EntityKind::Bold, offset: 0, length: 4 });
            }
        }
        if i > 3 && rng.chance(12) {
            m.reply_to = Some(MsgId(1 + rng.below(i) as i32));
        }
        if rng.chance(8) {
            let emoji = rng.pick(REACTIONS).to_string();
            m.reactions.push(ReactionCount { emoji, count: 1 + rng.below(4) as u32, mine: rng.chance(30) });
        }
        if outgoing && rng.chance(5) {
            m.edit_date = Some(Timestamp(date + 60));
        }
        out.push(m);
    }
    out
}

fn gen_channel(rng: &mut Lcg, peer: PeerRef, n: usize, last_at: i64) -> Vec<Message> {
    let mut out: Vec<Message> = (0..n)
        .map(|i| Message {
            id: MsgId(i as i32 + 1),
            peer,
            sender: None,
            date: Timestamp(last_at - (n - 1 - i) as i64 * 86_400 / 3),
            edit_date: None,
            outgoing: false,
            text: rng.pick(CHANNEL_POSTS).to_string(),
            entities: vec![],
            reply_to: None,
            forwarded_from: None,
            media: None,
            sticker: None,
            reactions: if rng.chance(50) {
                vec![ReactionCount { emoji: "👍".into(), count: 3 + rng.below(40) as u32, mine: false }]
            } else {
                vec![]
            },
            random_id: None,
            state: SendState::Sent,
            ttl_secs: None,
        })
        .collect();
    out.sort_by_key(|m| m.id);
    out
}

pub fn incoming_text(rng: &mut Lcg) -> String {
    rng.pick(PHRASES).to_string()
}
