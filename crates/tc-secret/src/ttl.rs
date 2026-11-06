//! Temporizador de autodestrucción (sans-IO: el llamador aporta la hora y decide
//! cuándo despertar con [`SelfDestructTimer::next_deadline`]).
//!
//! Semántica: un mensaje con `ttl > 0` empieza a contar cuando se lee. En el
//! receptor, al mostrarlo/leerlo (y entonces se envía `ReadMessages`); en el emisor,
//! al recibir `ReadMessages` con su `random_id`. El `ttl` del propio mensaje
//! prevalece sobre el del chat (`SetMessageTTL`).

use std::collections::{BTreeSet, HashMap};

/// `ttl` efectivo de un mensaje: el suyo si es > 0, si no el del chat.
pub fn effective_ttl(message_ttl: i32, chat_ttl: i32) -> i32 {
    if message_ttl > 0 { message_ttl } else { chat_ttl.max(0) }
}

/// Cola de destrucción por `random_id`.
#[derive(Debug, Default, Clone)]
pub struct SelfDestructTimer {
    by_deadline: BTreeSet<(i64, i64)>,
    by_id: HashMap<i64, i64>,
}

impl SelfDestructTimer {
    /// Timer vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Arranca la cuenta atrás de `random_id` leído en `read_at` (segundos Unix).
    /// Devuelve el instante de destrucción, o `None` si `ttl <= 0`. Si ya estaba
    /// programado se conserva el plazo más temprano (leer dos veces no lo alarga).
    pub fn start(&mut self, random_id: i64, ttl: i32, read_at: i64) -> Option<i64> {
        if ttl <= 0 {
            return None;
        }
        let at = read_at.saturating_add(i64::from(ttl));
        if let Some(&old) = self.by_id.get(&random_id) {
            if old <= at {
                return Some(old);
            }
            self.by_deadline.remove(&(old, random_id));
        }
        self.by_id.insert(random_id, at);
        self.by_deadline.insert((at, random_id));
        Some(at)
    }

    /// Cancela (p. ej. el mensaje se borró antes).
    pub fn cancel(&mut self, random_id: i64) {
        if let Some(at) = self.by_id.remove(&random_id) {
            self.by_deadline.remove(&(at, random_id));
        }
    }

    /// Próximo instante en el que hay algo que destruir.
    pub fn next_deadline(&self) -> Option<i64> {
        self.by_deadline.first().map(|&(at, _)| at)
    }

    /// Saca y devuelve los `random_id` cuyo plazo es `<= now`, en orden de plazo.
    pub fn expire(&mut self, now: i64) -> Vec<i64> {
        let mut out = Vec::new();
        while let Some(&(at, id)) = self.by_deadline.first() {
            if at > now {
                break;
            }
            self.by_deadline.pop_first();
            self.by_id.remove(&id);
            out.push(id);
        }
        out
    }

    /// Mensajes programados.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// ¿Vacío?
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}
