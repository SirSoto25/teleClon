//! Caché en memoria de usuarios y grupos vistos en las respuestas, con sus `access_hash`
//! (necesarios para construir `InputPeer`). Los hashes también se copian a la sesión de
//! grammers para que sobrevivan a un reinicio.
use std::collections::HashMap;

use grammers_session::types::{ChannelKind, PeerAuth, PeerId, PeerInfo};
use grammers_tl_types as tl;
use tc_core::{BackendError, BackendResult, Group, GroupId, GroupKind, PeerRef, User, UserId, UserStatus};

use crate::ids::{GroupTarget, channel_group, chat_group, decode_group};
use crate::session::TcSession;

#[derive(Default)]
pub struct PeerStore {
    pub users: HashMap<UserId, User>,
    pub groups: HashMap<GroupId, Group>,
    user_hash: HashMap<i64, i64>,
    channel_hash: HashMap<i64, i64>,
}

pub fn user_status(s: Option<&tl::enums::UserStatus>) -> UserStatus {
    match s {
        Some(tl::enums::UserStatus::Online(_)) => UserStatus::Online,
        Some(tl::enums::UserStatus::Offline(o)) => UserStatus::LastSeen(tc_core::Timestamp(o.was_online.into())),
        Some(tl::enums::UserStatus::Recently(_)) => UserStatus::Recently,
        _ => UserStatus::Offline,
    }
}

/// Usuario TL → dominio. Las cuentas borradas reciben un nombre fijo.
pub fn user_from_tl(u: &tl::types::User) -> User {
    let first = u.first_name.clone().unwrap_or_default();
    let first = if u.deleted && first.is_empty() { "Cuenta eliminada".to_string() } else { first };
    User {
        id: UserId(u.id),
        first_name: first,
        last_name: u.last_name.clone().unwrap_or_default(),
        username: u.username.clone().or_else(|| {
            u.usernames.as_ref().and_then(|v| {
                v.iter().find_map(|n| match n {
                    tl::enums::Username::Username(n) if n.active => Some(n.username.clone()),
                    _ => None,
                })
            })
        }),
        status: user_status(u.status.as_ref()),
        is_contact: u.contact,
        is_bot: u.bot,
    }
}

/// Chat/canal TL → `Group`. `None` para variantes que no son grupos.
pub fn group_from_tl(c: &tl::enums::Chat) -> Option<Group> {
    Some(match c {
        tl::enums::Chat::Chat(c) => Group {
            id: chat_group(c.id),
            kind: GroupKind::Basic,
            title: c.title.clone(),
            username: None,
            member_count: c.participants_count.max(0) as u32,
        },
        tl::enums::Chat::Forbidden(c) => {
            Group { id: chat_group(c.id), kind: GroupKind::Basic, title: c.title.clone(), username: None, member_count: 0 }
        }
        tl::enums::Chat::Channel(c) => Group {
            id: channel_group(c.id),
            kind: if c.broadcast { GroupKind::Channel } else { GroupKind::Super },
            title: c.title.clone(),
            username: c.username.clone(),
            member_count: c.participants_count.unwrap_or(0).max(0) as u32,
        },
        tl::enums::Chat::ChannelForbidden(c) => Group {
            id: channel_group(c.id),
            kind: if c.broadcast { GroupKind::Channel } else { GroupKind::Super },
            title: c.title.clone(),
            username: None,
            member_count: 0,
        },
        _ => return None,
    })
}

impl PeerStore {
    /// Guarda los usuarios; devuelve la información de autorización para la sesión.
    pub fn ingest_users(&mut self, users: &[tl::enums::User]) -> Vec<PeerInfo> {
        let mut infos = Vec::new();
        for u in users {
            let tl::enums::User::User(u) = u else { continue };
            let mut mapped = user_from_tl(u);
            if u.min {
                // Un usuario «min» trae datos parciales y un hash inutilizable: no pisar lo que ya hay.
                if let Some(prev) = self.users.get(&mapped.id) {
                    mapped.is_contact = prev.is_contact;
                    mapped.status = prev.status;
                }
            } else if let Some(h) = u.access_hash {
                self.user_hash.insert(u.id, h);
                infos.push(PeerInfo::User {
                    id: u.id,
                    auth: Some(PeerAuth::from_hash(h)),
                    bot: Some(u.bot),
                    is_self: Some(u.is_self),
                });
            }
            self.users.insert(mapped.id, mapped);
        }
        infos
    }

    pub fn ingest_chats(&mut self, chats: &[tl::enums::Chat]) -> Vec<PeerInfo> {
        let mut infos = Vec::new();
        for c in chats {
            if let Some(g) = group_from_tl(c) {
                self.groups.insert(g.id, g);
            }
            match c {
                tl::enums::Chat::Chat(c) => infos.push(PeerInfo::Chat { id: c.id }),
                tl::enums::Chat::Channel(ch) if !ch.min => {
                    if let Some(h) = ch.access_hash {
                        self.channel_hash.insert(ch.id, h);
                        let kind = if ch.broadcast {
                            ChannelKind::Broadcast
                        } else if ch.gigagroup {
                            ChannelKind::Gigagroup
                        } else {
                            ChannelKind::Megagroup
                        };
                        infos.push(PeerInfo::Channel { id: ch.id, auth: Some(PeerAuth::from_hash(h)), kind: Some(kind) });
                    }
                }
                tl::enums::Chat::ChannelForbidden(ch) => {
                    self.channel_hash.insert(ch.id, ch.access_hash);
                }
                _ => {}
            }
        }
        infos
    }

    /// Título de la fila de un chat.
    pub fn title(&self, peer: PeerRef, me: Option<UserId>) -> String {
        match peer {
            PeerRef::User(u) if Some(u) == me => "Mensajes guardados".to_string(),
            PeerRef::User(u) => self.users.get(&u).map_or_else(|| format!("Usuario {}", u.0), User::display_name),
            PeerRef::Group(g) => self.groups.get(&g).map_or_else(|| "Grupo".to_string(), |g| g.title.clone()),
            PeerRef::Secret(_) => "Chat secreto".to_string(),
        }
    }

    /// Nombre visible de un `Peer` TL (para «reenviado de»).
    pub fn name_of(&self, p: &tl::enums::Peer) -> Option<String> {
        match crate::ids::peer_from_tl(p) {
            PeerRef::User(u) => self.users.get(&u).map(User::display_name),
            PeerRef::Group(g) => self.groups.get(&g).map(|g| g.title.clone()),
            PeerRef::Secret(_) => None,
        }
    }

    fn user_hash_or_session(&self, id: i64, session: &TcSession) -> Option<i64> {
        self.user_hash.get(&id).copied().or_else(|| session.peer_info(PeerId::user_unchecked(id)).and_then(|p| p.auth()).map(|a| a.hash()))
    }

    fn channel_hash_or_session(&self, id: i64, session: &TcSession) -> Option<i64> {
        self.channel_hash
            .get(&id)
            .copied()
            .or_else(|| session.peer_info(PeerId::channel_unchecked(id)).and_then(|p| p.auth()).map(|a| a.hash()))
    }

    /// `InputPeer` para llamar a la API. Sin hash conocido → `NotFound` (hay que cargar antes
    /// la lista de chats o el historial donde aparece).
    pub fn input_peer(&self, peer: PeerRef, me: Option<UserId>, session: &TcSession) -> BackendResult<tl::enums::InputPeer> {
        match peer {
            PeerRef::User(u) if Some(u) == me => Ok(tl::enums::InputPeer::PeerSelf),
            PeerRef::User(u) => {
                let h = self.user_hash_or_session(u.0, session).ok_or(BackendError::NotFound)?;
                Ok(tl::types::InputPeerUser { user_id: u.0, access_hash: h }.into())
            }
            PeerRef::Group(g) => match decode_group(g).ok_or(BackendError::NotFound)? {
                GroupTarget::Chat(id) => Ok(tl::types::InputPeerChat { chat_id: id }.into()),
                GroupTarget::Channel(id) => {
                    let h = self.channel_hash_or_session(id, session).ok_or(BackendError::NotFound)?;
                    Ok(tl::types::InputPeerChannel { channel_id: id, access_hash: h }.into())
                }
            },
            PeerRef::Secret(_) => Err(BackendError::Forbidden("los chats secretos aún no están disponibles en la red real".into())),
        }
    }

    pub fn input_user(&self, u: UserId, me: Option<UserId>, session: &TcSession) -> BackendResult<tl::enums::InputUser> {
        if Some(u) == me {
            return Ok(tl::enums::InputUser::UserSelf);
        }
        let h = self.user_hash_or_session(u.0, session).ok_or(BackendError::NotFound)?;
        Ok(tl::types::InputUser { user_id: u.0, access_hash: h }.into())
    }

    pub fn input_channel(&self, id: i64, session: &TcSession) -> BackendResult<tl::enums::InputChannel> {
        let h = self.channel_hash_or_session(id, session).ok_or(BackendError::NotFound)?;
        Ok(tl::types::InputChannel { channel_id: id, access_hash: h }.into())
    }
}
