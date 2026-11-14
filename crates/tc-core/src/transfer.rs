//! Transferencias de archivos (subidas de `send_media` y descargas a disco de `download_media_to`)
//! con progreso y cancelación. Contrato completo en ADR-0011 y docs/architecture.md §3.
//!
//! Resumen del contrato:
//! - Cada transferencia se anuncia con `BackendEvent::TransferProgress(Transfer)`: primero `Running`
//!   (con `done` creciente) y al final **exactamente un** estado terminal (`Done`, `Failed` o `Cancelled`).
//! - Una subida se identifica con `TransferId::Upload(random_ids[0])` de su `OutgoingMedia` (la UI lo
//!   conoce antes de llamar a `send_media`, así puede cancelar mientras espera); una descarga, con el
//!   `TransferId::Download(n)` que devuelve `download_media_to`.
//! - `cancel_transfer` es idempotente: cancelar algo ya terminado o desconocido no es un error.
use std::path::{Path, PathBuf};

use crate::{error::BackendError, ids::*, model::OutgoingMedia};

/// Identidad de una transferencia. Dos espacios separados para que una subida y una descarga nunca
/// choquen: el de las subidas lo pone la UI (el `RandomId` del primer elemento) y el de las descargas
/// el backend (contador propio).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TransferId {
    Upload(RandomId),
    Download(u64),
}

impl TransferId {
    /// Id de la subida de `media` (`None` si no trae `random_ids`; el backend la rechazará).
    pub fn upload_of(media: &OutgoingMedia) -> Option<Self> {
        media.random_ids.first().map(|r| Self::Upload(*r))
    }

    pub fn kind(self) -> TransferKind {
        match self {
            Self::Upload(_) => TransferKind::Upload,
            Self::Download(_) => TransferKind::Download,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransferKind {
    Upload,
    Download,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferState {
    /// En curso (`done` puede ser 0 al empezar).
    Running,
    /// Terminada: en una descarga, el archivo ya está en su destino; en una subida, los mensajes ya
    /// se confirmaron (`MessageSent` llega antes que este estado).
    Done,
    /// Falló (red, disco, permisos…): no queda archivo parcial ni se envió nada pendiente.
    Failed(BackendError),
    /// Cancelada con `cancel_transfer`: tampoco queda archivo parcial.
    Cancelled,
}

impl TransferState {
    /// ¿Es el último evento de la transferencia?
    pub fn is_finished(&self) -> bool {
        !matches!(self, Self::Running)
    }
}

/// Estado de una transferencia en un instante (lo que lleva `BackendEvent::TransferProgress`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    pub id: TransferId,
    /// Igual a `id.kind()`; se repite para que la UI no tenga que derivarlo.
    pub kind: TransferKind,
    pub peer: PeerRef,
    /// Mensaje al que pertenece el archivo (descargas). En subidas, `None`: el mensaje aún no existe.
    pub msg: Option<MsgId>,
    /// Nombre a mostrar (el del archivo; en una subida de varios elementos, el del primero).
    pub name: String,
    /// Bytes transferidos (no decrece; en `Done` vale `total`).
    pub done: u64,
    /// Bytes totales; 0 si no se conocen (la UI muestra progreso indeterminado).
    pub total: u64,
    pub state: TransferState,
}

impl Transfer {
    /// Fracción 0..=1 (`None` si el total es desconocido).
    pub fn fraction(&self) -> Option<f32> {
        (self.total > 0).then(|| (self.done.min(self.total) as f64 / self.total as f64) as f32)
    }
}

/// Ruta del archivo parcial de una descarga a `dest`: `<dest>.part` en el mismo directorio (así el
/// renombrado final es atómico). Al terminar bien se renombra a `dest`; si falla o se cancela, se borra.
pub fn partial_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".part");
    dest.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MediaKind, OutgoingMediaItem};

    #[test]
    fn upload_id_comes_from_the_first_random_id() {
        let item = OutgoingMediaItem { kind: MediaKind::File, path: "/tmp/a".into(), file_name: "a".into(), size_bytes: 1, width: 0, height: 0 };
        let mut m = OutgoingMedia { items: vec![item.clone(), item], caption: String::new(), reply_to: None, random_ids: vec![RandomId(7), RandomId(8)] };
        assert_eq!(TransferId::upload_of(&m), Some(TransferId::Upload(RandomId(7))));
        assert_eq!(TransferId::Upload(RandomId(7)).kind(), TransferKind::Upload);
        assert_eq!(TransferId::Download(7).kind(), TransferKind::Download);
        assert_ne!(TransferId::Upload(RandomId(7)), TransferId::Download(7));
        m.random_ids.clear();
        assert_eq!(TransferId::upload_of(&m), None);
    }

    #[test]
    fn fraction_and_finished() {
        let mut t = Transfer {
            id: TransferId::Download(1),
            kind: TransferKind::Download,
            peer: PeerRef::User(UserId(1)),
            msg: Some(MsgId(3)),
            name: "a.pdf".into(),
            done: 25,
            total: 100,
            state: TransferState::Running,
        };
        assert_eq!(t.fraction(), Some(0.25));
        assert!(!t.state.is_finished());
        t.total = 0;
        assert_eq!(t.fraction(), None);
        for s in [TransferState::Done, TransferState::Cancelled, TransferState::Failed(BackendError::Network)] {
            assert!(s.is_finished());
        }
    }

    /// Un backend que solo implementa lo obligatorio de `MediaApi` sigue compilando y responde
    /// «no soportado» a lo nuevo.
    struct Old;
    #[async_trait::async_trait]
    impl crate::api::MediaApi for Old {
        async fn send_media(&self, _: PeerRef, _: OutgoingMedia) -> crate::BackendResult<Vec<crate::model::Message>> {
            Ok(Vec::new())
        }
        async fn download_profile_photo(&self, _: PeerRef, _: crate::model::ProfilePhoto, _: crate::model::PhotoSize) -> crate::BackendResult<Vec<u8>> {
            Ok(Vec::new())
        }
        async fn download_media(&self, _: PeerRef, _: MsgId, _: crate::model::FileRef, _: crate::model::MediaVariant) -> crate::BackendResult<Vec<u8>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn default_methods_are_unsupported() {
        use crate::api::MediaApi;
        let file = crate::model::FileRef {
            kind: crate::model::FileKind::Document,
            id: 1,
            access_hash: 0,
            file_reference: vec![],
            dc_id: 1,
            size_bytes: 10,
            thumb: None,
            full: None,
        };
        let peer = PeerRef::User(UserId(1));
        let r = futures::executor::block_on(Old.download_media_to(peer, MsgId(1), file, crate::model::MediaVariant::Full, PathBuf::from("/tmp/x")));
        assert_eq!(r, Err(crate::api::unsupported()));
        let r = futures::executor::block_on(Old.cancel_transfer(TransferId::Download(1)));
        assert_eq!(r, Err(crate::api::unsupported()));
    }

    #[test]
    fn partial_path_is_a_sibling() {
        assert_eq!(partial_path(Path::new("/d/informe.pdf")), PathBuf::from("/d/informe.pdf.part"));
        assert_eq!(partial_path(Path::new("x")), PathBuf::from("x.part"));
    }
}
