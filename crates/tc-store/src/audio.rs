//! Repositorio de la migración 0005: datos extra de los medios (notas de voz, audios y vídeos
//! redondos) que la tabla `media` no recoge.

use crate::db::Store;
use crate::error::Result;
use rusqlite::{OptionalExtension, params};
use tc_core as core;

/// Fila de `media_extra`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MediaExtraRow {
    pub media_id: i64,
    /// Vídeo redondo (se guarda en `media` como `video`).
    pub round: bool,
    /// Hay datos de audio (`tc_core::AudioInfo`).
    pub has_audio: bool,
    pub voice: bool,
    pub title: Option<String>,
    pub performer: Option<String>,
    /// Forma de onda decodificada (un valor 0..=31 por barra).
    pub waveform: Vec<u8>,
    pub mime: Option<String>,
}

impl Store {
    pub fn upsert_media_extra(&self, r: &MediaExtraRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO media_extra (media_id, voice, round, has_audio, title, performer, waveform, mime)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(media_id) DO UPDATE SET voice=excluded.voice, round=excluded.round,
               has_audio=excluded.has_audio, title=excluded.title, performer=excluded.performer,
               waveform=excluded.waveform, mime=excluded.mime",
            params![r.media_id, r.voice, r.round, r.has_audio, r.title, r.performer, r.waveform, r.mime],
        )?;
        Ok(())
    }

    pub fn get_media_extra(&self, media_id: i64) -> Result<Option<MediaExtraRow>> {
        Ok(self
            .conn
            .query_row(
                "SELECT voice, round, has_audio, title, performer, waveform, mime FROM media_extra WHERE media_id = ?1",
                params![media_id],
                |r| {
                    Ok(MediaExtraRow {
                        media_id,
                        voice: r.get(0)?,
                        round: r.get(1)?,
                        has_audio: r.get(2)?,
                        title: r.get(3)?,
                        performer: r.get(4)?,
                        waveform: r.get(5)?,
                        mime: r.get(6)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn delete_media_extra(&self, media_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM media_extra WHERE media_id = ?1", params![media_id])?;
        Ok(())
    }

    /// Guarda (o borra si no hay) los datos extra de un medio.
    pub(crate) fn save_media_extra(&self, md: &core::Media) -> Result<()> {
        let round = md.kind == core::MediaKind::VideoNote;
        match (md.audio.as_ref(), round) {
            (None, false) => self.delete_media_extra(md.id),
            (a, round) => self.upsert_media_extra(&MediaExtraRow {
                media_id: md.id,
                round,
                has_audio: a.is_some(),
                voice: a.is_some_and(|a| a.voice),
                title: a.and_then(|a| a.title.clone()),
                performer: a.and_then(|a| a.performer.clone()),
                waveform: a.map(|a| a.waveform.clone()).unwrap_or_default(),
                mime: a.and_then(|a| a.mime.clone()),
            }),
        }
    }
}

/// Aplica los datos extra guardados a un medio leído de `media`.
pub(crate) fn apply_media_extra(kind: &mut core::MediaKind, row: Option<MediaExtraRow>) -> Option<core::AudioInfo> {
    let row = row?;
    if row.round && *kind == core::MediaKind::Video {
        *kind = core::MediaKind::VideoNote;
    }
    row.has_audio.then_some(core::AudioInfo { voice: row.voice, title: row.title, performer: row.performer, waveform: row.waveform, mime: row.mime })
}

#[cfg(test)]
mod tests {
    use crate::convert::{find_chat_key, load_message, persist_message};
    use crate::db::Store;
    use tc_core::*;

    fn voice(id: i32, audio: Option<AudioInfo>, kind: MediaKind) -> Message {
        Message {
            id: MsgId(id),
            peer: PeerRef::User(UserId(10)),
            sender: Some(UserId(10)),
            date: Timestamp(100),
            edit_date: None,
            outgoing: false,
            text: String::new(),
            entities: vec![],
            reply_to: None,
            forwarded_from: None,
            media: Some(Media {
                id: 500 + i64::from(id),
                kind,
                file_name: None,
                size_bytes: 1234,
                width: 0,
                height: 0,
                duration_secs: Some(9),
                local_path: None,
                file: None,
                audio,
            }),
            sticker: None,
            reactions: vec![],
            random_id: None,
            state: SendState::Sent,
            ttl_secs: None,
        }
    }

    #[test]
    fn voice_notes_and_music_survive_the_cache() {
        let s = Store::open_in_memory().unwrap();
        let info = AudioInfo { voice: true, title: None, performer: None, waveform: vec![0, 31, 7, 12], mime: Some("audio/ogg".into()) };
        let music = AudioInfo { voice: false, title: Some("Rescoldo".into()), performer: Some("Las Ascuas".into()), waveform: vec![], mime: Some("audio/mpeg".into()) };
        for m in [voice(1, Some(info), MediaKind::Voice), voice(2, Some(music), MediaKind::Audio), voice(3, None, MediaKind::VideoNote)] {
            persist_message(&s, &m, None).unwrap();
            let ck = find_chat_key(&s, m.peer).unwrap().unwrap();
            let row = s.get_message(ck, i64::from(m.id.0)).unwrap().unwrap();
            assert_eq!(load_message(&s, m.peer, &row).unwrap().media, m.media);
        }
        // Sin datos de audio (p. ej. al editar el mensaje a otro medio) la fila se borra.
        let mut m = voice(1, None, MediaKind::File);
        m.media.as_mut().unwrap().id = 501;
        persist_message(&s, &m, None).unwrap();
        assert!(s.get_media_extra(501).unwrap().is_none());
    }
}
