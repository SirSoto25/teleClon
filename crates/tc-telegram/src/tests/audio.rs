//! Notas de voz, audios y vídeos redondos: mapeo de atributos (duración, forma de onda de 5 bits,
//! título/intérprete, MIME) y descarga completa por partes con progreso.
//!
//! Reproduce el fallo visto con la red real: antes una nota de voz llegaba sin forma de onda ni MIME,
//! la música quedaba como «archivo» y la UI no tenía cómo pedir el documento entero.
use futures::StreamExt;
use grammers_tl_types as tl;
use tc_core::*;

use super::fixtures::*;
use super::flows::restored;
use super::*;
use crate::convert::media_from_tl;
use crate::media::{PART_SIZE, media_location};

fn doc(mime: &str, size: i64, attributes: Vec<tl::enums::DocumentAttribute>) -> tl::types::Document {
    tl::types::Document {
        id: 4242,
        access_hash: 77,
        file_reference: vec![5, 6],
        date: 0,
        mime_type: mime.into(),
        size,
        thumbs: None,
        video_thumbs: None,
        dc_id: 4,
        attributes,
    }
}

fn media(d: tl::types::Document, voice: bool, round: bool) -> tl::enums::MessageMedia {
    tl::types::MessageMediaDocument {
        nopremium: false,
        spoiler: false,
        video: false,
        round,
        voice,
        document: Some(d.into()),
        alt_documents: None,
        video_cover: None,
        video_timestamp: None,
        ttl_seconds: None,
    }
    .into()
}

fn audio_attr(voice: bool, duration: i32, title: Option<&str>, performer: Option<&str>, waveform: Option<Vec<u8>>) -> tl::enums::DocumentAttribute {
    tl::types::DocumentAttributeAudio { voice, duration, title: title.map(Into::into), performer: performer.map(Into::into), waveform }.into()
}

#[test]
fn voice_note_keeps_waveform_duration_mime_and_full_file() {
    let wave: Vec<u8> = (0..100).map(|i| (i * 3 % 32) as u8).collect();
    let packed = waveform::encode(&wave);
    assert_eq!(packed.len(), 63, "Telegram manda 100 valores en 63 bytes");
    let d = doc("audio/ogg", 23_456, vec![audio_attr(true, 12, None, None, Some(packed))]);
    let m = media_from_tl(&media(d, true, false)).0.expect("una nota de voz es un medio");
    assert_eq!((m.kind, m.duration_secs, m.size_bytes), (MediaKind::Voice, Some(12), 23_456));
    let a = m.audio.expect("datos de audio");
    assert!(a.voice);
    assert_eq!(&a.waveform[..100], wave.as_slice());
    assert_eq!(a.mime.as_deref(), Some("audio/ogg"));
    // Se descarga el documento entero (sin miniatura), del DC del archivo.
    let f = m.file.expect("descargable");
    assert_eq!((f.kind, f.id, f.dc_id, f.size_bytes, f.thumb.as_deref()), (FileKind::Document, 4242, 4, 23_456, None));
    let Some(tl::enums::InputFileLocation::InputDocumentFileLocation(loc)) = media_location(&f, MediaVariant::Full) else { panic!("ubicación de documento") };
    assert_eq!((loc.id, loc.access_hash, loc.file_reference, loc.thumb_size.as_str()), (4242, 77, vec![5, 6], ""));
    assert!(media_location(&f, MediaVariant::Thumb).is_none());

    // Sin forma de onda (clientes antiguos): sigue siendo voz, con la forma de onda vacía.
    let d = doc("audio/ogg", 100, vec![audio_attr(true, 3, None, None, None)]);
    let m = media_from_tl(&media(d, false, false)).0.unwrap();
    assert_eq!((m.kind, m.audio.map(|a| (a.voice, a.waveform.len()))), (MediaKind::Voice, Some((true, 0))));
}

#[test]
fn music_round_videos_and_bare_audio_documents() {
    let d = doc(
        "audio/mpeg",
        4_000_000,
        vec![audio_attr(false, 215, Some("Rescoldo"), Some("Las Ascuas"), None), tl::types::DocumentAttributeFilename { file_name: "rescoldo.mp3".into() }.into()],
    );
    let m = media_from_tl(&media(d, false, false)).0.unwrap();
    assert_eq!((m.kind, m.duration_secs, m.file_name.as_deref()), (MediaKind::Audio, Some(215), Some("rescoldo.mp3")));
    let a = m.audio.unwrap();
    assert_eq!((a.voice, a.title.as_deref(), a.performer.as_deref(), a.mime.as_deref()), (false, Some("Rescoldo"), Some("Las Ascuas"), Some("audio/mpeg")));

    // Título en blanco: se trata como ausente.
    let d = doc("audio/flac", 10, vec![audio_attr(false, 1, Some("  "), None, None)]);
    assert_eq!(media_from_tl(&media(d, false, false)).0.unwrap().audio.unwrap().title, None);

    // Un .m4a mandado como documento sin atributo de audio: se reproduce igual.
    let d = doc("audio/mp4", 10, vec![tl::types::DocumentAttributeFilename { file_name: "nota.m4a".into() }.into()]);
    let m = media_from_tl(&media(d, false, false)).0.unwrap();
    assert_eq!((m.kind, m.audio.and_then(|a| a.mime)), (MediaKind::Audio, Some("audio/mp4".into())));

    // Vídeo redondo: con su duración y sin datos de audio.
    let round = tl::types::DocumentAttributeVideo {
        round_message: true,
        supports_streaming: true,
        nosound: false,
        duration: 9.4,
        w: 384,
        h: 384,
        preload_prefix_size: None,
        video_start_ts: None,
        video_codec: None,
    };
    let d = doc("video/mp4", 500_000, vec![round.into()]);
    let m = media_from_tl(&media(d, false, true)).0.unwrap();
    assert_eq!((m.kind, m.duration_secs, m.width, m.height, m.audio), (MediaKind::VideoNote, Some(9), 384, 384, None));

    // Un PDF sigue siendo un archivo sin datos de audio.
    let d = doc("application/pdf", 10, vec![]);
    let m = media_from_tl(&media(d, false, false)).0.unwrap();
    assert_eq!((m.kind, m.audio), (MediaKind::File, None));
}

#[tokio::test]
async fn full_voice_download_joins_parts_and_reports_progress() {
    let total = PART_SIZE * 2 + 300;
    let (b, fake, _) = restored(
        move |_dc, id, body| match id {
            x if x == cid::<tl::functions::upload::GetFile>() => {
                let req: tl::functions::upload::GetFile = decode(body);
                let tl::enums::InputFileLocation::InputDocumentFileLocation(loc) = req.location else { panic!("documento") };
                assert_eq!((loc.id, loc.thumb_size.as_str()), (4242, ""));
                let off = req.offset as usize;
                let n = (total - off).min(PART_SIZE);
                ok(tl::enums::upload::File::from(tl::types::upload::File {
                    r#type: tl::enums::storage::FileType::FilePartial,
                    mtime: 0,
                    bytes: (off..off + n).map(|i| (i % 251) as u8).collect(),
                }))
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    let home = b.inner.net.session.home_dc_id().unwrap();
    let mut d = doc("audio/ogg", total as i64, vec![audio_attr(true, 70, None, None, None)]);
    d.dc_id = home;
    let f = media_from_tl(&media(d, true, false)).0.unwrap().file.unwrap();
    let mut events = b.events();
    let bytes = b.download_media(PeerRef::User(UserId(10)), MsgId(3), f, MediaVariant::Full).await.unwrap();
    assert_eq!(bytes.len(), total);
    assert!(bytes.iter().enumerate().all(|(i, &v)| v == (i % 251) as u8), "las partes se unen en orden");
    assert_eq!(fake.count("upload.getFile"), 3);
    let mut progress = Vec::new();
    while let Ok(Some(ev)) = tokio::time::timeout(Duration::from_millis(50), events.next()).await {
        if let BackendEvent::Download { file_id: 4242, done, total: t } = ev {
            progress.push((done, t));
        }
    }
    let t = total as u64;
    assert_eq!(progress, vec![(PART_SIZE as u64, t), (2 * PART_SIZE as u64, t), (t, t)]);
}
