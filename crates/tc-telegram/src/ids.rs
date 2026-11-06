//! Correspondencia de identificadores: Telegram tiene tres espacios (usuarios, chats básicos,
//! canales/supergrupos) y el dominio dos (`UserId`, `GroupId`). Los grupos usan el mismo
//! «id marcado» que la Bot API: chat básico = `-id`, canal = `-(10^12 + id)`.
use grammers_tl_types as tl;
use tc_core::{GroupId, PeerRef, UserId};

const CHANNEL_OFFSET: i64 = 1_000_000_000_000;

/// Destino concreto de un `GroupId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupTarget {
    Chat(i64),
    Channel(i64),
}

pub fn chat_group(id: i64) -> GroupId {
    GroupId(-id)
}

pub fn channel_group(id: i64) -> GroupId {
    GroupId(-(CHANNEL_OFFSET + id))
}

pub fn decode_group(g: GroupId) -> Option<GroupTarget> {
    match g.0 {
        x if x < -CHANNEL_OFFSET => Some(GroupTarget::Channel(-x - CHANNEL_OFFSET)),
        x if x < 0 => Some(GroupTarget::Chat(-x)),
        _ => None,
    }
}

pub fn peer_from_tl(p: &tl::enums::Peer) -> PeerRef {
    match p {
        tl::enums::Peer::User(u) => PeerRef::User(UserId(u.user_id)),
        tl::enums::Peer::Chat(c) => PeerRef::Group(chat_group(c.chat_id)),
        tl::enums::Peer::Channel(c) => PeerRef::Group(channel_group(c.channel_id)),
    }
}

/// Usuario autor si el `Peer` es un usuario.
pub fn user_of(p: &tl::enums::Peer) -> Option<UserId> {
    match p {
        tl::enums::Peer::User(u) => Some(UserId(u.user_id)),
        _ => None,
    }
}

/// Id de canal si el par es un canal/supergrupo.
pub fn channel_of(peer: PeerRef) -> Option<i64> {
    match peer {
        PeerRef::Group(g) => match decode_group(g) {
            Some(GroupTarget::Channel(c)) => Some(c),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_ids_roundtrip_and_never_collide() {
        for id in [1, 42, 999_999_999_999] {
            assert_eq!(decode_group(chat_group(id)), Some(GroupTarget::Chat(id)));
        }
        for id in [1, 42, 997_852_516_352, 2_000_000_000_000] {
            assert_eq!(decode_group(channel_group(id)), Some(GroupTarget::Channel(id)));
        }
        assert_ne!(chat_group(5), channel_group(5));
        assert_eq!(decode_group(GroupId(5)), None);
        assert_eq!(channel_group(1234).0, -1_000_000_001_234);
    }

    #[test]
    fn peers_map_to_domain() {
        let u = tl::enums::Peer::User(tl::types::PeerUser { user_id: 7 });
        let c = tl::enums::Peer::Chat(tl::types::PeerChat { chat_id: 8 });
        let ch = tl::enums::Peer::Channel(tl::types::PeerChannel { channel_id: 9 });
        assert_eq!(peer_from_tl(&u), PeerRef::User(UserId(7)));
        assert_eq!(peer_from_tl(&c), PeerRef::Group(GroupId(-8)));
        assert_eq!(channel_of(peer_from_tl(&ch)), Some(9));
        assert_eq!(channel_of(peer_from_tl(&c)), None);
        assert_eq!(user_of(&u), Some(UserId(7)));
    }
}
