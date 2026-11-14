//! macOS: `NSUserNotification` con mac-notification-sys.
//!
//! Se eligió frente a `UNUserNotificationCenter` (objc2-user-notifications) porque funciona también
//! con el binario suelto (`cargo run`), donde UserNotifications aborta al no haber bundle; el coste
//! es una API obsoleta desde macOS 11 que sigue funcionando (ADR-0013). Dentro de `Parlo.app` el
//! remitente es el bundle id `app.parlo.Parlo`; fuera, se suplanta el de la app instalada si la
//! encuentra (o el del Finder).
//!
//! Cada aviso espera su clic en un hilo propio (la biblioteca bloquea hasta la respuesta). Sin
//! sustitución por chat ni retirada: la biblioteca no expone el identificador.

use crate::{
    Action, AppInfo, Capabilities, ChatKey, EventSender, Notification, Notifier, NotifyError,
    NotifyEvent, Result, icon::icon_path,
};
use mac_notification_sys::{MainButton, NotificationResponse};

pub(crate) struct MacNotifier {
    tx: EventSender,
}

impl MacNotifier {
    pub(crate) fn new(app: &AppInfo, tx: EventSender) -> Result<Self> {
        let bundle = mac_notification_sys::get_bundle_identifier(&app.name)
            .unwrap_or_else(|| app.app_id.clone());
        if let Err(e) = mac_notification_sys::set_application(&bundle) {
            // `AlreadySet` si otro código ya lo fijó: no impide avisar.
            tracing::debug!("mac-notification-sys set_application({bundle}): {e}");
        }
        Ok(Self { tx })
    }
}

fn response_to_event(
    chat: ChatKey,
    actions: &[Action],
    r: NotificationResponse,
) -> Option<NotifyEvent> {
    match r {
        NotificationResponse::Click => Some(NotifyEvent::Clicked(chat)),
        NotificationResponse::ActionButton(label) => actions
            .iter()
            .find(|a| a.label == label)
            .map(|a| NotifyEvent::Action(chat, a.id.clone())),
        NotificationResponse::None
        | NotificationResponse::CloseButton(_)
        | NotificationResponse::Reply(_) => None,
    }
}

impl Notifier for MacNotifier {
    fn notify(&self, n: Notification) -> Result<()> {
        let tx = self.tx.clone();
        std::thread::Builder::new()
            .name("tc-notify-mac".into())
            .spawn(move || {
                let image = n
                    .icon
                    .as_ref()
                    .and_then(icon_path)
                    .map(|p| p.to_string_lossy().into_owned());
                let labels: Vec<&str> = n.actions.iter().map(|a| a.label.as_str()).collect();
                let mut opts = mac_notification_sys::Notification::new();
                opts.title(&n.title).message(&n.body).wait_for_click(true);
                match labels.as_slice() {
                    [] => {}
                    [one] => {
                        opts.main_button(MainButton::SingleAction(one));
                    }
                    many => {
                        opts.main_button(MainButton::DropdownActions("Acciones", many));
                    }
                }
                if let Some(img) = &image {
                    opts.content_image(img);
                }
                if !n.silent {
                    opts.default_sound();
                }
                match opts.send() {
                    Ok(r) => {
                        if let Some(ev) = response_to_event(n.chat_key.clone(), &n.actions, r) {
                            tx.send(ev);
                        }
                    }
                    Err(e) => tracing::warn!("aviso de macOS no enviado: {e}"),
                }
            })
            .map(|_| ())
            .map_err(|e| NotifyError::Backend(e.to_string()))
    }

    fn withdraw(&self, _chat: &ChatKey) -> Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            actions: true,
            replace: false,
            withdraw: false,
            click: true,
        }
    }
}
