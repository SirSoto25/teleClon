//! Qué reacciones se pueden poner en cada chat.
//!
//! Causa del `PREMIUM_ACCOUNT_REQUIRED` visto con la red real: la UI ofrecía una lista fija y la
//! enviaba a cualquier chat. En «Mensajes guardados» las reacciones son *etiquetas*, que son de
//! Premium; además la lista fija tenía emojis fuera del conjunto gratuito («😂») o con otra forma
//! («❤️» con `U+FE0F`; la red usa «❤»). Ahora se ofrece solo:
//! - lo que `messages.getAvailableReactions` da como activo y no premium (salvo cuenta Premium),
//! - ∩ lo que permite el chat (`ChatFull.available_reactions`: todas, algunas o ninguna),
//! - ordenado con `messages.getTopReactions` y `messages.getRecentReactions`,
//! - y nada en «Mensajes guardados» sin Premium ni en chats secretos.
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use grammers_tl_types as tl;
use tc_core::*;

use crate::ids::{GroupTarget, decode_group};

/// Cuánto vale lo que se sabe de un chat antes de volver a preguntar.
const CHAT_TTL: Duration = Duration::from_secs(10 * 60);
/// Cuántas de las más usadas se piden.
const TOP_LIMIT: i32 = 24;

/// Reacciones que admite un chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChatAllowed {
    All,
    Some(Vec<String>),
    None,
}

/// Reacción del catálogo global.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Available {
    pub emoji: String,
    pub premium: bool,
}

#[derive(Default)]
pub(crate) struct ReactionCatalog {
    available: (i32, Vec<Available>),
    top: (i64, Vec<String>),
    recent: (i64, Vec<String>),
    premium: Option<bool>,
    chats: HashMap<PeerRef, (Instant, ChatAllowed)>,
}

/// Emojis de una lista de `Reaction` (las personalizadas y de pago no se ofrecen).
fn emojis(list: &[tl::enums::Reaction]) -> Vec<String> {
    list.iter()
        .filter_map(|r| match r {
            tl::enums::Reaction::Emoji(e) => Some(e.emoticon.clone()),
            _ => None,
        })
        .collect()
}

fn chat_allowed(r: Option<&tl::enums::ChatReactions>) -> ChatAllowed {
    match r {
        // Sin el campo, el chat no restringe: las del catálogo.
        None | Some(tl::enums::ChatReactions::All(_)) => ChatAllowed::All,
        Some(tl::enums::ChatReactions::None) => ChatAllowed::None,
        Some(tl::enums::ChatReactions::Some(s)) => ChatAllowed::Some(emojis(&s.reactions)),
    }
}

/// Lista final: preferidas (`order`) primero y luego el catálogo; solo lo permitido.
pub(crate) fn compose(catalog: &[Available], premium: bool, allowed: &ChatAllowed, order: &[String]) -> Vec<String> {
    let usable: Vec<&str> = catalog.iter().filter(|a| premium || !a.premium).map(|a| a.emoji.as_str()).collect();
    let ok = |e: &str| {
        usable.contains(&e)
            && match allowed {
                ChatAllowed::All => true,
                ChatAllowed::Some(l) => l.iter().any(|x| x == e),
                ChatAllowed::None => false,
            }
    };
    let mut seen = HashSet::new();
    order
        .iter()
        .map(String::as_str)
        .chain(usable.iter().copied())
        .filter(|e| ok(e) && seen.insert(*e))
        .map(str::to_owned)
        .collect()
}

impl crate::Inner {
    fn reactions_cat(&self) -> std::sync::MutexGuard<'_, ReactionCatalog> {
        self.reactions.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// ¿La cuenta es Premium? (una vez por sesión).
    async fn me_premium(&self) -> BackendResult<bool> {
        if let Some(p) = self.reactions_cat().premium {
            return Ok(p);
        }
        let users = self.rpc(&tl::functions::users::GetUsers { id: vec![tl::enums::InputUser::UserSelf] }).await?;
        let premium = users.iter().any(|u| matches!(u, tl::enums::User::User(u) if u.premium));
        self.reactions_cat().premium = Some(premium);
        Ok(premium)
    }

    /// Catálogo global (activas), con caché por `hash`. Si falla, el conjunto gratuito conocido.
    async fn reaction_catalog(&self) -> Vec<Available> {
        let hash = self.reactions_cat().available.0;
        match self.rpc(&tl::functions::messages::GetAvailableReactions { hash }).await {
            Ok(tl::enums::messages::AvailableReactions::Reactions(r)) => {
                let list: Vec<Available> = r
                    .reactions
                    .into_iter()
                    .map(|tl::enums::AvailableReaction::Reaction(a)| a)
                    .filter(|a| !a.inactive)
                    .map(|a| Available { emoji: a.reaction, premium: a.premium })
                    .collect();
                self.reactions_cat().available = (r.hash, list.clone());
                list
            }
            Ok(tl::enums::messages::AvailableReactions::NotModified) => self.reactions_cat().available.1.clone(),
            Err(e) => {
                tracing::warn!(error = %e, "sin catálogo de reacciones: se usa el conjunto gratuito conocido");
                let cached = self.reactions_cat().available.1.clone();
                if cached.is_empty() { DEFAULT_REACTIONS.iter().map(|e| Available { emoji: (*e).into(), premium: false }).collect() } else { cached }
            }
        }
    }

    /// Más usadas y recientes de la cuenta (para ordenar). Los fallos solo quitan el orden.
    async fn preferred_reactions(&self) -> Vec<String> {
        let (top_hash, recent_hash) = {
            let c = self.reactions_cat();
            (c.top.0, c.recent.0)
        };
        let top = match self.rpc(&tl::functions::messages::GetTopReactions { limit: TOP_LIMIT, hash: top_hash }).await {
            Ok(tl::enums::messages::Reactions::Reactions(r)) => {
                let l = emojis(&r.reactions);
                self.reactions_cat().top = (r.hash, l.clone());
                l
            }
            _ => self.reactions_cat().top.1.clone(),
        };
        let recent = match self.rpc(&tl::functions::messages::GetRecentReactions { limit: TOP_LIMIT, hash: recent_hash }).await {
            Ok(tl::enums::messages::Reactions::Reactions(r)) => {
                let l = emojis(&r.reactions);
                self.reactions_cat().recent = (r.hash, l.clone());
                l
            }
            _ => self.reactions_cat().recent.1.clone(),
        };
        top.into_iter().chain(recent).collect()
    }

    /// Lo que permite el chat (`ChatFull.available_reactions`), con caché de `CHAT_TTL`.
    async fn chat_reactions(&self, peer: PeerRef) -> BackendResult<ChatAllowed> {
        if let Some((at, a)) = self.reactions_cat().chats.get(&peer)
            && at.elapsed() < CHAT_TTL
        {
            return Ok(a.clone());
        }
        let allowed = match peer {
            PeerRef::User(_) => ChatAllowed::All,
            PeerRef::Secret(_) => ChatAllowed::None,
            PeerRef::Group(g) => {
                let full = match decode_group(g).ok_or(BackendError::NotFound)? {
                    GroupTarget::Chat(id) => self.rpc(&tl::functions::messages::GetFullChat { chat_id: id }).await?,
                    GroupTarget::Channel(id) => {
                        let channel = self.cache().peers.input_channel(id, &self.net.session)?;
                        self.rpc(&tl::functions::channels::GetFullChannel { channel }).await?
                    }
                };
                let tl::enums::messages::ChatFull::Full(full) = full;
                self.ingest(&full.users, &full.chats);
                match &full.full_chat {
                    tl::enums::ChatFull::Full(c) => chat_allowed(c.available_reactions.as_ref()),
                    tl::enums::ChatFull::ChannelFull(c) => chat_allowed(c.available_reactions.as_ref()),
                    // Comunidades: sin soporte en Parlo; lo que diga el catálogo.
                    _ => ChatAllowed::All,
                }
            }
        };
        self.reactions_cat().chats.insert(peer, (Instant::now(), allowed.clone()));
        Ok(allowed)
    }

    pub(crate) async fn available_reactions(&self, peer: PeerRef) -> BackendResult<Vec<String>> {
        let me = self.ensure_logged_in()?;
        if matches!(peer, PeerRef::Secret(_)) {
            return Ok(Vec::new());
        }
        let premium = self.me_premium().await?;
        // «Mensajes guardados»: reaccionar es etiquetar, y las etiquetas son de Premium.
        if peer == PeerRef::User(me) && !premium {
            return Ok(Vec::new());
        }
        let allowed = self.chat_reactions(peer).await?;
        if allowed == ChatAllowed::None {
            return Ok(Vec::new());
        }
        let catalog = self.reaction_catalog().await;
        let order = self.preferred_reactions().await;
        Ok(compose(&catalog, premium, &allowed, &order))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn av(e: &str, premium: bool) -> Available {
        Available { emoji: e.into(), premium }
    }

    #[test]
    fn compose_filters_premium_and_chat_and_orders_by_use() {
        let cat = vec![av("👍", false), av("❤", false), av("🔥", false), av("🤩", true), av("😁", false)];
        let order = vec!["🔥".to_string(), "🤩".into(), "😂".into(), "🔥".into()];
        // Sin Premium: fuera 🤩; 😂 no está en el catálogo; sin duplicados; las usadas primero.
        assert_eq!(compose(&cat, false, &ChatAllowed::All, &order), ["🔥", "👍", "❤", "😁"]);
        assert_eq!(compose(&cat, true, &ChatAllowed::All, &order), ["🔥", "🤩", "👍", "❤", "😁"]);
        let some = ChatAllowed::Some(vec!["❤".into(), "🤩".into(), "😂".into()]);
        assert_eq!(compose(&cat, false, &some, &order), ["❤"]);
        assert!(compose(&cat, true, &ChatAllowed::None, &order).is_empty());
        // La forma con U+FE0F no es la del catálogo.
        assert!(!compose(&cat, false, &ChatAllowed::All, &[]).contains(&"❤️".to_string()));
    }
}
