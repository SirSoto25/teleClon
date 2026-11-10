//! Reacciones: el selector solo ofrece las que el chat admite (`MessagesApi::available_reactions`,
//! pedidas al abrir el chat y guardadas por chat); al reaccionar se aplica el cambio en el acto y,
//! si la red lo rechaza (`PremiumRequired`, `ReactionUnavailable`, red...), se deshace y se avisa.

use super::*;

/// Aplica «mi reacción pasa a ser `emoji`» (o se quita con `None`) a una lista de reacciones, como
/// lo hace el servidor: una sola reacción propia por mensaje.
pub fn apply_mine(list: &mut Vec<ReactionCount>, emoji: Option<&str>) {
    if let Some(i) = list.iter().position(|r| r.mine) {
        list[i].mine = false;
        list[i].count = list[i].count.saturating_sub(1);
        if list[i].count == 0 {
            list.remove(i);
        }
    }
    if let Some(e) = emoji {
        match list.iter_mut().find(|r| r.emoji == e) {
            Some(r) => {
                r.count += 1;
                r.mine = true;
            }
            None => list.push(ReactionCount { emoji: e.to_string(), count: 1, mine: true }),
        }
    }
}

/// Texto del aviso cuando la red rechaza una reacción.
pub fn reaction_error_text(e: &BackendError, saved_messages: bool) -> String {
    match e {
        BackendError::PremiumRequired if saved_messages => {
            "En «Mensajes guardados» las reacciones son etiquetas, y las etiquetas necesitan Telegram Premium.".into()
        }
        BackendError::PremiumRequired => "Esa reacción necesita Telegram Premium.".into(),
        BackendError::ReactionUnavailable => "Esa reacción no está disponible en este chat.".into(),
        e => error_text(e),
    }
}

impl App {
    /// Reacciones que se ofrecen en `peer` (`None` mientras se cargan).
    pub fn reaction_choices(&self, peer: PeerRef) -> Option<&[String]> {
        self.reaction_choices.get(&peer).and_then(|c| c.as_deref())
    }

    /// ¿Se puede reaccionar en `peer`? (mientras carga, sí: el botón abre el selector).
    pub fn can_react(&self, peer: PeerRef) -> bool {
        self.reaction_choices(peer).is_none_or(|l| !l.is_empty())
    }

    /// Pide las reacciones del chat si aún no se tienen.
    pub(crate) fn load_reactions(&mut self, peer: PeerRef) -> Task<Msg> {
        if self.reaction_choices.contains_key(&peer) {
            return Task::none();
        }
        self.reaction_choices.insert(peer, None);
        self.call(move |b| async move { b.available_reactions(peer).await }, move |r| Msg::ReactionsLoaded(peer, r))
    }

    pub(crate) fn on_reactions_loaded(&mut self, peer: PeerRef, result: BackendResult<Vec<String>>) -> Task<Msg> {
        let list = match result {
            Ok(l) => l,
            // Sin respuesta: el conjunto gratuito conocido (si no vale, la red lo dirá y se revierte).
            Err(e) => {
                tracing::debug!(error = %e, "no se pudieron pedir las reacciones del chat");
                DEFAULT_REACTIONS.iter().map(|e| (*e).to_string()).collect()
            }
        };
        self.reaction_choices.insert(peer, Some(list));
        Task::none()
    }

    /// Reacción optimista: se ve ya; `Msg::Reacted` la confirma o la deshace.
    pub(crate) fn react(&mut self, id: MsgId, emoji: Option<String>) -> Task<Msg> {
        self.reactions_for = None;
        let Some(c) = self.conv.as_mut() else { return Task::none() };
        let peer = c.peer;
        let Some(m) = c.find_mut(id) else { return Task::none() };
        let prev = m.reactions.clone();
        apply_mine(&mut m.reactions, emoji.as_deref());
        self.call(move |b| async move { b.send_reaction(peer, id, emoji).await }, move |result| Msg::Reacted { peer, id, prev, result })
    }

    pub(crate) fn on_reacted(&mut self, peer: PeerRef, id: MsgId, prev: Vec<ReactionCount>, result: BackendResult<()>) -> Task<Msg> {
        let Err(e) = result else { return Task::none() };
        if let Some(m) = self.conv.as_mut().filter(|c| c.peer == peer).and_then(|c| c.find_mut(id)) {
            m.reactions = prev;
        }
        let saved = self.me_id().is_some_and(|me| peer == PeerRef::User(me));
        if matches!(e, BackendError::PremiumRequired | BackendError::ReactionUnavailable) {
            // Lo que se ofrecía no era válido: se vuelve a preguntar la próxima vez.
            self.reaction_choices.remove(&peer);
        }
        self.toast(reaction_error_text(&e, saved), true);
        Task::none()
    }
}
