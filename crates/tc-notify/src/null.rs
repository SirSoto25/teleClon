//! Notificadores sin SO: [`NullNotifier`] (descarta) y [`RecordingNotifier`] (pruebas).

use std::sync::{Arc, Mutex, MutexGuard};

use crate::{
    Capabilities, ChatKey, EventSender, Notification, Notifier, NotifyEvent, NotifyEvents, Result,
    event_channel,
};

/// No muestra nada. Para entornos sin escritorio o con las notificaciones desactivadas.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullNotifier;

impl Notifier for NullNotifier {
    fn notify(&self, _n: Notification) -> Result<()> {
        Ok(())
    }

    fn withdraw(&self, _chat: &ChatKey) -> Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
}

#[derive(Debug, Default)]
struct Log {
    sent: Vec<Notification>,
    withdrawn: Vec<ChatKey>,
}

/// Guarda lo que se le pide y permite simular clics. Clonarlo comparte el registro, así una prueba
/// conserva una copia mientras la app usa otra como `Box<dyn Notifier>`.
#[derive(Debug, Clone)]
pub struct RecordingNotifier {
    log: Arc<Mutex<Log>>,
    tx: EventSender,
    caps: Capabilities,
}

impl RecordingNotifier {
    /// Con todas las capacidades activas.
    pub fn new() -> (Self, NotifyEvents) {
        Self::with_capabilities(Capabilities {
            actions: true,
            replace: true,
            withdraw: true,
            click: true,
        })
    }

    pub fn with_capabilities(caps: Capabilities) -> (Self, NotifyEvents) {
        let (tx, rx) = event_channel();
        (
            Self {
                log: Arc::default(),
                tx,
                caps,
            },
            rx,
        )
    }

    fn log(&self) -> MutexGuard<'_, Log> {
        self.log.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Todos los avisos pedidos, en orden.
    pub fn sent(&self) -> Vec<Notification> {
        self.log().sent.clone()
    }

    /// Último aviso pedido.
    pub fn last(&self) -> Option<Notification> {
        self.log().sent.last().cloned()
    }

    /// Chats cuyos avisos se retiraron, en orden.
    pub fn withdrawn(&self) -> Vec<ChatKey> {
        self.log().withdrawn.clone()
    }

    pub fn clear(&self) {
        let mut log = self.log();
        log.sent.clear();
        log.withdrawn.clear();
    }

    /// Simula un clic en el aviso de `chat`.
    pub fn click(&self, chat: impl Into<ChatKey>) {
        self.tx.send(NotifyEvent::Clicked(chat.into()));
    }

    /// Simula un clic en el botón `id` del aviso de `chat`.
    pub fn press(&self, chat: impl Into<ChatKey>, id: impl Into<String>) {
        self.tx.send(NotifyEvent::Action(chat.into(), id.into()));
    }
}

impl Notifier for RecordingNotifier {
    fn notify(&self, n: Notification) -> Result<()> {
        self.log().sent.push(n);
        Ok(())
    }

    fn withdraw(&self, chat: &ChatKey) -> Result<()> {
        self.log().withdrawn.push(chat.clone());
        Ok(())
    }

    fn capabilities(&self) -> Capabilities {
        self.caps
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    fn n(chat: &str) -> Notification {
        Notification {
            chat_key: chat.into(),
            title: "Ana".into(),
            body: "hola".into(),
            icon: None,
            actions: vec![],
            silent: false,
        }
    }

    #[test]
    fn null_acepta_todo() {
        let null = NullNotifier;
        assert!(null.notify(n("u:1")).is_ok());
        assert!(null.withdraw(&"u:1".into()).is_ok());
        assert_eq!(null.capabilities(), Capabilities::default());
    }

    #[test]
    fn recording_guarda_y_comparte_registro() {
        let (rec, _rx) = RecordingNotifier::new();
        let boxed: Box<dyn Notifier> = Box::new(rec.clone());
        boxed.notify(n("u:1")).unwrap();
        boxed.notify(n("u:2")).unwrap();
        boxed.withdraw(&"u:1".into()).unwrap();
        assert_eq!(rec.sent().len(), 2);
        assert_eq!(rec.last().unwrap().chat_key, ChatKey::from("u:2"));
        assert_eq!(rec.withdrawn(), vec![ChatKey::from("u:1")]);
        rec.clear();
        assert!(rec.sent().is_empty() && rec.withdrawn().is_empty());
    }

    #[test]
    fn recording_simula_eventos() {
        let (rec, mut rx) = RecordingNotifier::new();
        rec.click("u:1");
        rec.press("g:5", "mark-read");
        let got = futures::executor::block_on(async { [rx.next().await, rx.next().await] });
        assert_eq!(got[0], Some(NotifyEvent::Clicked("u:1".into())));
        assert_eq!(
            got[1],
            Some(NotifyEvent::Action("g:5".into(), "mark-read".into()))
        );
    }

    #[test]
    fn enviar_sin_receptor_no_falla() {
        let (rec, rx) = RecordingNotifier::new();
        drop(rx);
        rec.click("u:1");
    }
}
