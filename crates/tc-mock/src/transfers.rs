//! Transferencias simuladas (ADR-0011): progreso por trozos de `send_media` y descargas a disco con
//! `download_media_to`, cancelación y fallos inyectables.
//!
//! El ritmo lo marca `TransferSim`: entre trozo y trozo se espera `delay` con el reloj de tokio, así
//! que en pruebas con `#[tokio::test(start_paused = true)]` el tiempo es virtual y exacto (se puede
//! avanzar con `tokio::time::advance` o dejar que tokio lo adelante solo).
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tc_core::*;
use tokio::sync::watch;

use crate::{Inner, MockBackend};

/// Como mucho tantos eventos `Running` por transferencia (los trozos crecen si hace falta), igual que
/// la red real limita la frecuencia de los avisos.
pub const MAX_STEPS: u64 = 100;

/// Cómo simula el mock las transferencias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferSim {
    /// Bytes por trozo (mínimo 1; ver `MAX_STEPS`).
    pub chunk: u64,
    /// Espera antes de cada trozo (reloj de tokio). Cero: solo cede el turno.
    pub delay: Duration,
    /// Falla con ese error cuando lleva `.0` bytes o más (antes de escribir el siguiente trozo).
    pub fail_at: Option<(u64, BackendError)>,
}

impl Default for TransferSim {
    fn default() -> Self {
        Self { chunk: 64 * 1024, delay: Duration::ZERO, fail_at: None }
    }
}

/// Registro de transferencias en curso del mock.
#[derive(Default)]
pub(crate) struct Transfers {
    sim: Mutex<TransferSim>,
    next: AtomicU64,
    active: Mutex<HashMap<TransferId, watch::Sender<bool>>>,
}

impl Transfers {
    fn sim(&self) -> TransferSim {
        self.sim.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn active(&self) -> std::sync::MutexGuard<'_, HashMap<TransferId, watch::Sender<bool>>> {
        self.active.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn next_download(&self) -> TransferId {
        TransferId::Download(self.next.fetch_add(1, Ordering::Relaxed) + 1)
    }

    /// Da de alta `id` y devuelve el receptor de su cancelación (sustituye a otra con el mismo id).
    fn register(&self, id: TransferId) -> watch::Receiver<bool> {
        let (tx, rx) = watch::channel(false);
        self.active().insert(id, tx);
        rx
    }

    /// Da de baja `id` si sigue siendo la misma transferencia que `rx`.
    fn unregister(&self, id: TransferId, rx: &watch::Receiver<bool>) {
        let mut active = self.active();
        if active.get(&id).is_some_and(|tx| tx.subscribe().same_channel(rx)) {
            active.remove(&id);
        }
    }
}

impl MockBackend {
    /// Cambia la simulación de las transferencias siguientes (y de las que estén en curso, que la
    /// leen al empezar: no les afecta).
    pub fn set_transfer_sim(&self, sim: TransferSim) {
        *self.inner.transfers.sim.lock().unwrap_or_else(|e| e.into_inner()) = sim;
    }

    pub fn transfer_sim(&self) -> TransferSim {
        self.inner.transfers.sim()
    }

    /// Transferencias en curso (para pruebas).
    pub fn active_transfers(&self) -> usize {
        self.inner.transfers.active().len()
    }

    /// Simula la subida de `media` (ya validada) con progreso y cancelación. `Ok` = subida completa;
    /// en error ya se emitió el estado terminal.
    pub(crate) async fn simulate_upload(&self, peer: PeerRef, media: &OutgoingMedia) -> BackendResult<UploadGuard> {
        let id = TransferId::upload_of(media).ok_or(BackendError::InvalidInput("random_ids"))?;
        let t = Transfer {
            id,
            kind: TransferKind::Upload,
            peer,
            msg: None,
            name: media.items.first().map(|i| i.file_name.clone()).unwrap_or_default(),
            done: 0,
            total: media.items.iter().map(|i| i.size_bytes).sum(),
            state: TransferState::Running,
        };
        let rx = self.inner.transfers.register(id);
        self.inner.emit(BackendEvent::TransferProgress(t.clone()));
        let mut guard = UploadGuard { inner: Arc::clone(&self.inner), t, rx };
        match self.inner.run_chunks(&mut guard.t, &mut guard.rx, |_, _| Ok(())).await {
            Ok(()) => Ok(guard),
            Err(e) => {
                guard.finish(Err(e.clone()));
                Err(e)
            }
        }
    }

    pub(crate) async fn download_to(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant, dest: PathBuf) -> BackendResult<TransferId> {
        self.inner.gate().await?;
        check_dest(&dest)?;
        // El mock genera los bytes en memoria (son pocos); la red real los escribe según llegan.
        let bytes = tc_core::MediaApi::download_media(self, peer, msg, file, variant).await?;
        let id = self.inner.transfers.next_download();
        let mut t = Transfer {
            id,
            kind: TransferKind::Download,
            peer,
            msg: Some(msg),
            name: dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            done: 0,
            total: bytes.len() as u64,
            state: TransferState::Running,
        };
        let mut rx = self.inner.transfers.register(id);
        self.inner.emit(BackendEvent::TransferProgress(t.clone()));
        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            let part = partial_path(&dest);
            let r = async {
                let mut f = std::fs::File::create(&part).map_err(storage)?;
                inner
                    .run_chunks(&mut t, &mut rx, |off, len| f.write_all(&bytes[off as usize..(off + len) as usize]).map_err(storage))
                    .await?;
                f.sync_all().map_err(storage)?;
                drop(f);
                std::fs::rename(&part, &dest).map_err(storage)
            }
            .await;
            if r.is_err() {
                let _ = std::fs::remove_file(&part);
            }
            inner.transfers.unregister(id, &rx);
            t.state = terminal(r);
            inner.emit(BackendEvent::TransferProgress(t));
        });
        Ok(id)
    }

    pub(crate) fn cancel(&self, id: TransferId) {
        if let Some(tx) = self.inner.transfers.active().get(&id) {
            let _ = tx.send(true);
        }
    }
}

/// Subida ya transferida y aún sin estado terminal: `finish` lo emite tras crear los mensajes.
pub(crate) struct UploadGuard {
    inner: Arc<Inner>,
    t: Transfer,
    rx: watch::Receiver<bool>,
}

impl UploadGuard {
    pub(crate) fn finish(mut self, r: BackendResult<()>) {
        self.inner.transfers.unregister(self.t.id, &self.rx);
        self.t.state = terminal(r);
        if self.t.state == TransferState::Done {
            self.t.done = self.t.total;
        }
        self.inner.emit(BackendEvent::TransferProgress(self.t.clone()));
    }
}

impl Inner {
    /// Recorre `t.total` bytes por trozos: espera (cancelable), comprueba fallo inyectado, llama a
    /// `chunk(offset, len)` y emite `Running`. `Err(Cancelled)` si se canceló.
    async fn run_chunks(
        &self,
        t: &mut Transfer,
        cancel: &mut watch::Receiver<bool>,
        mut chunk: impl FnMut(u64, u64) -> BackendResult<()>,
    ) -> BackendResult<()> {
        let sim = self.transfers.sim();
        let step = sim.chunk.max(1).max(t.total.div_ceil(MAX_STEPS));
        while t.done < t.total {
            if sim.delay.is_zero() {
                tokio::task::yield_now().await;
            } else {
                tokio::select! {
                    _ = tokio::time::sleep(sim.delay) => {}
                    r = cancel.changed() => if r.is_err() { tokio::time::sleep(sim.delay).await },
                }
            }
            if *cancel.borrow() {
                return Err(BackendError::Cancelled);
            }
            if let Some((at, e)) = &sim.fail_at
                && t.done >= *at
            {
                return Err(e.clone());
            }
            let len = step.min(t.total - t.done);
            chunk(t.done, len)?;
            t.done += len;
            self.emit(BackendEvent::TransferProgress(t.clone()));
        }
        // Un fallo en 0 bytes de un archivo vacío también se inyecta.
        match &sim.fail_at {
            Some((at, e)) if t.total == 0 && *at == 0 => Err(e.clone()),
            _ => Ok(()),
        }
    }
}

fn terminal(r: BackendResult<()>) -> TransferState {
    match r {
        Ok(()) => TransferState::Done,
        Err(BackendError::Cancelled) => TransferState::Cancelled,
        Err(e) => TransferState::Failed(e),
    }
}

fn storage(e: std::io::Error) -> BackendError {
    BackendError::Storage(e.to_string())
}

/// El directorio de `dest` debe existir y `dest` no puede ser un directorio.
fn check_dest(dest: &Path) -> BackendResult<()> {
    let parent = match dest.parent() {
        Some(p) if p.as_os_str().is_empty() => Path::new("."),
        Some(p) => p,
        None => return Err(BackendError::InvalidInput("dest")),
    };
    if dest.file_name().is_none() || !parent.is_dir() || dest.is_dir() {
        return Err(BackendError::InvalidInput("dest"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
