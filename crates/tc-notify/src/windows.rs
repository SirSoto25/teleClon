//! Windows 10/11: toasts WinRT con el crate oficial `windows` (Rust, sin C).
//!
//! Requisito del SO: una app de escritorio sin MSIX solo puede mostrar toasts si existe un acceso
//! directo en el menú Inicio con su AppUserModelID (lo crea el instalador Inno, ver
//! `packaging/windows/parlo.iss`). Además el proceso declara el mismo AUMID al arrancar
//! (`SetCurrentProcessExplicitAppUserModelID`) para que la barra de tareas lo agrupe igual.
//!
//! Un aviso por chat: `Tag` = hash de la clave (≤ 16 caracteres, límite de Windows 10 antiguos),
//! `Group` fijo. Un toast nuevo con la misma etiqueta sustituye al anterior; `withdraw` lo quita del
//! centro de actividades. El clic llega por el evento `Activated` mientras el proceso vive (sin
//! activador COM: tras cerrar Parlo, el clic en un aviso antiguo del centro de actividades solo
//! abre la app por el acceso directo).

use std::hash::{DefaultHasher, Hash, Hasher};

use windows::{
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{ToastActivatedEventArgs, ToastNotification, ToastNotificationManager},
    Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID,
    core::{HSTRING, IInspectable, Interface},
};

use crate::{
    AppInfo, Capabilities, ChatKey, EventSender, Notification, Notifier, NotifyError, NotifyEvent,
    Result,
    toast_xml::{ACTION_PREFIX, toast_xml},
};

const GROUP: &str = "chats";

pub(crate) struct ToastNotifier {
    app_id: HSTRING,
    notifier: windows::UI::Notifications::ToastNotifier,
    tx: EventSender,
}

fn backend(e: windows::core::Error) -> NotifyError {
    NotifyError::Backend(e.message().to_string())
}

fn tag(chat: &ChatKey) -> HSTRING {
    let mut h = DefaultHasher::new();
    chat.hash(&mut h);
    HSTRING::from(format!("{:016x}", h.finish()))
}

impl ToastNotifier {
    pub(crate) fn new(app: &AppInfo, tx: EventSender) -> Result<Self> {
        let app_id = HSTRING::from(app.app_id.as_str());
        // SAFETY: llamada Win32 sin punteros propios; el HSTRING vive durante la llamada.
        if let Err(e) = unsafe { SetCurrentProcessExplicitAppUserModelID(&app_id) } {
            tracing::debug!("SetCurrentProcessExplicitAppUserModelID: {e}");
        }
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&app_id)
            .map_err(|e| NotifyError::Unavailable(e.message().to_string()))?;
        Ok(Self {
            app_id,
            notifier,
            tx,
        })
    }
}

impl Notifier for ToastNotifier {
    fn notify(&self, n: Notification) -> Result<()> {
        let doc = XmlDocument::new().map_err(backend)?;
        doc.LoadXml(&HSTRING::from(toast_xml(&n)))
            .map_err(backend)?;
        let toast = ToastNotification::CreateToastNotification(&doc).map_err(backend)?;
        toast.SetTag(&tag(&n.chat_key)).map_err(backend)?;
        toast.SetGroup(&HSTRING::from(GROUP)).map_err(backend)?;
        let tx = self.tx.clone();
        let chat = n.chat_key;
        let handler = TypedEventHandler::<ToastNotification, IInspectable>::new(move |_, args| {
            let arguments = args
                .as_ref()
                .and_then(|i| i.cast::<ToastActivatedEventArgs>().ok())
                .and_then(|a| a.Arguments().ok())
                .map(|s| s.to_string())
                .unwrap_or_default();
            match arguments.strip_prefix(ACTION_PREFIX) {
                Some(id) => tx.send(NotifyEvent::Action(chat.clone(), id.to_owned())),
                None => tx.send(NotifyEvent::Clicked(chat.clone())),
            }
            Ok(())
        });
        toast.Activated(&handler).map_err(backend)?;
        self.notifier.Show(&toast).map_err(backend)
    }

    fn withdraw(&self, chat: &ChatKey) -> Result<()> {
        let history = ToastNotificationManager::History().map_err(backend)?;
        history
            .RemoveGroupedTagWithId(&tag(chat), &HSTRING::from(GROUP), &self.app_id)
            .map_err(backend)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            actions: true,
            replace: true,
            withdraw: true,
            click: true,
        }
    }
}
