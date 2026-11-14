//! Envío de archivos (fase A2, ADR-0011): lo que no depende del estado de la app. Qué es cada
//! archivo elegido (foto, vídeo, audio u otro), su vista previa, cómo se reparte en envíos
//! (álbumes de hasta 10, fotos/vídeos aparte de documentos), límites de Telegram y rutas pegadas.
//!
//! La lectura del disco y el decodificado de las vistas previas van en el pool bloqueante de tokio
//! (`inspect`): nunca en el hilo de la UI.

use std::path::{Path, PathBuf};

use ::image::GenericImageView;
use iced::widget::image::Handle;
use tc_core::MediaKind;

/// Máximo de caracteres del pie de foto (cuentas sin Premium).
pub const MAX_CAPTION_CHARS: usize = 1024;
/// A partir de aquí se enseña el contador del pie.
pub const CAPTION_COUNTER_FROM: usize = 900;
/// Elementos por álbum (Telegram agrupa como mucho 10).
pub const ALBUM_MAX: usize = 10;
/// Una foto «comprimida» no puede pasar de 10 MB: si pesa más, va como archivo.
pub const PHOTO_MAX_BYTES: u64 = 10 * 1024 * 1024;
/// Proporción máxima (lado largo / lado corto) de una foto comprimida.
pub const PHOTO_MAX_RATIO: f32 = 20.0;
/// Límite de un archivo sin Premium (con Premium, 4 GB): se avisa, no se impide (no se sabe aquí).
pub const FILE_WARN_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Lado (px físicos) de la vista previa del diálogo y de la burbuja pendiente.
pub const PREVIEW_PX: u32 = 640;
/// Id del campo del pie de foto (foco al abrir el diálogo).
pub const CAPTION_ID: &str = "send-caption";

/// Qué es un archivo, por su extensión (y, en imágenes, porque se pueda leer su tamaño).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileClass {
    Image,
    Video,
    Audio,
    Other,
}

/// Un archivo elegido para enviar, ya inspeccionado.
#[derive(Debug, Clone)]
pub struct Picked {
    pub path: PathBuf,
    /// Nombre que verá quien lo reciba (el del archivo).
    pub name: String,
    pub size: u64,
    pub class: FileClass,
    /// Dimensiones (solo imágenes legibles; 0 si no se conocen).
    pub width: u32,
    pub height: u32,
    /// Vista previa ya decodificada (solo imágenes).
    pub preview: Option<Handle>,
}

impl Picked {
    /// ¿Se puede mandar como foto comprimida? (imagen legible, ≤ 10 MB y proporción razonable).
    pub fn can_be_photo(&self) -> bool {
        if self.class != FileClass::Image || self.width == 0 || self.height == 0 || self.size > PHOTO_MAX_BYTES {
            return false;
        }
        let (a, b) = (self.width.max(self.height) as f32, self.width.min(self.height) as f32);
        a / b <= PHOTO_MAX_RATIO
    }

    /// Tipo de medio con el que se envía.
    pub fn send_kind(&self, compress: bool) -> MediaKind {
        match self.class {
            FileClass::Image if compress && self.can_be_photo() => MediaKind::Photo,
            FileClass::Video => MediaKind::Video,
            FileClass::Audio => MediaKind::Audio,
            _ => MediaKind::File,
        }
    }
}

/// Clase por extensión (sin distinguir mayúsculas).
pub fn classify(name: &str) -> FileClass {
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "bmp" => FileClass::Image,
        "mp4" | "mov" | "m4v" | "webm" | "mkv" | "avi" => FileClass::Video,
        "mp3" | "m4a" | "aac" | "flac" | "wav" | "ogg" | "oga" | "opus" => FileClass::Audio,
        _ => FileClass::Other,
    }
}

/// Lee tamaño, clase, dimensiones y vista previa de un archivo (bloqueante). El error es un texto
/// para el usuario.
pub fn inspect_one(path: &Path) -> Result<Picked, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let shown = if name.is_empty() { path.display().to_string() } else { name.clone() };
    let meta = std::fs::metadata(path).map_err(|_| format!("No se puede leer «{shown}»."))?;
    if meta.is_dir() {
        return Err(format!("«{shown}» es una carpeta: elige archivos sueltos."));
    }
    if meta.len() == 0 {
        return Err(format!("«{shown}» está vacío."));
    }
    let mut p = Picked { path: path.to_path_buf(), name, size: meta.len(), class: classify(&shown), width: 0, height: 0, preview: None };
    if p.class == FileClass::Image {
        match ::image::ImageReader::open(path).and_then(|r| r.with_guessed_format()).map(|r| r.decode()) {
            Ok(Ok(img)) => {
                (p.width, p.height) = img.dimensions();
                let small = if img.width().max(img.height()) > PREVIEW_PX { img.thumbnail(PREVIEW_PX, PREVIEW_PX) } else { img };
                let rgba = small.to_rgba8();
                p.preview = Some(Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw()));
            }
            // Extensión de imagen pero ilegible: se manda como archivo sin vista previa.
            _ => p.class = FileClass::Other,
        }
    }
    Ok(p)
}

/// Inspecciona `paths` fuera del hilo de la UI. Devuelve los legibles y un aviso por cada fallo.
pub async fn inspect(paths: Vec<PathBuf>) -> (Vec<Picked>, Vec<String>) {
    let work = move || {
        let mut ok = Vec::new();
        let mut errors = Vec::new();
        for p in &paths {
            match inspect_one(p) {
                Ok(x) => ok.push(x),
                Err(e) => errors.push(e),
            }
        }
        (ok, errors)
    };
    match tokio::runtime::Handle::try_current() {
        Ok(h) => h.spawn_blocking(work).await.unwrap_or_else(|_| (Vec::new(), vec!["No se pudieron leer los archivos.".into()])),
        Err(_) => work(),
    }
}

/// Reparte los archivos en envíos: primero fotos y vídeos (un álbum), luego el resto (otro), en
/// trozos de `ALBUM_MAX`, conservando el orden. Telegram no mezcla en un álbum fotos con documentos.
pub fn plan(files: &[Picked], compress: bool) -> Vec<Vec<usize>> {
    let visual = |k: MediaKind| matches!(k, MediaKind::Photo | MediaKind::Video);
    let (a, b): (Vec<usize>, Vec<usize>) = (0..files.len()).partition(|&i| visual(files[i].send_kind(compress)));
    [a, b].into_iter().flat_map(|g| g.chunks(ALBUM_MAX).map(<[usize]>::to_vec).collect::<Vec<_>>()).filter(|g| !g.is_empty()).collect()
}

/// Caracteres del pie (como los cuenta Telegram: sin espacios al principio ni al final).
pub fn caption_len(s: &str) -> usize {
    s.trim().chars().count()
}

/// Tamaño legible con coma decimal («820 B», «1,4 KB», «3,2 MB», «1,1 GB»).
pub fn size_label(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64 / 1024.0;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < UNITS.len() {
        v /= 1024.0;
        u += 1;
    }
    let s = if v >= 100.0 { format!("{v:.0}") } else { format!("{v:.1}") };
    format!("{} {}", s.replace('.', ","), UNITS[u])
}

/// Progreso «1,2 de 3,4 MB» (o solo lo hecho si el total no se sabe).
pub fn progress_label(done: u64, total: u64) -> String {
    if total == 0 {
        return size_label(done);
    }
    format!("{} de {}", size_label(done.min(total)), size_label(total))
}

/// Si el texto pegado es una lista de archivos que existen (rutas absolutas o URI `file://`, una
/// por línea, como lo dejan los gestores de archivos), sus rutas.
pub fn paths_from_text(s: &str) -> Option<Vec<PathBuf>> {
    let mut out = Vec::new();
    for line in s.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let p = match line.strip_prefix("file://") {
            // `file:///home/...` o `file://localhost/home/...`.
            Some(rest) => PathBuf::from(percent_decode(rest.strip_prefix("localhost").unwrap_or(rest))?),
            None => PathBuf::from(line),
        };
        if !p.is_absolute() || !p.is_file() {
            return None;
        }
        out.push(p);
    }
    (!out.is_empty()).then_some(out)
}

/// Decodifica `%XX` (UTF-8). `None` si la secuencia no es válida.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Lee una imagen del portapapeles del sistema (iced solo lee texto) y la guarda como PNG en el
/// directorio temporal. `Ok(None)`: no hay imagen. Bloqueante: llamar con `spawn_blocking`.
pub fn clipboard_image_to_png() -> Result<Option<PathBuf>, String> {
    let mut cb = arboard::Clipboard::new().map_err(|_| "No se pudo abrir el portapapeles.".to_string())?;
    let img = match cb.get_image() {
        Ok(i) => i,
        Err(arboard::Error::ContentNotAvailable) => return Ok(None),
        Err(_) => return Err("No se pudo leer la imagen del portapapeles.".into()),
    };
    let (w, h) = (img.width as u32, img.height as u32);
    let buf = ::image::RgbaImage::from_raw(w, h, img.bytes.into_owned()).ok_or("La imagen del portapapeles está dañada.")?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let dir = std::env::temp_dir().join("parlo-pegado");
    std::fs::create_dir_all(&dir).map_err(|_| "No se pudo guardar la imagen pegada.".to_string())?;
    let path = dir.join(format!("imagen-pegada-{stamp}.png"));
    buf.save(&path).map_err(|_| "No se pudo guardar la imagen pegada.".to_string())?;
    Ok(Some(path))
}

/// `clipboard_image_to_png` fuera del hilo de la UI.
pub async fn paste_image() -> Result<Option<PathBuf>, String> {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => h.spawn_blocking(clipboard_image_to_png).await.unwrap_or_else(|_| Err("No se pudo leer el portapapeles.".into())),
        Err(_) => clipboard_image_to_png(),
    }
}

/// Selector nativo de archivos (portal XDG en Linux, también en Wayland; diálogo del SO en Windows
/// y macOS). Vacío si se cancela.
pub async fn pick_files() -> Vec<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Elegir archivos para enviar")
        .pick_files()
        .await
        .map(|v| v.into_iter().map(|f| f.path().to_path_buf()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picked(name: &str, size: u64, w: u32, h: u32) -> Picked {
        Picked { path: PathBuf::from(format!("/tmp/{name}")), name: name.into(), size, class: classify(name), width: w, height: h, preview: None }
    }

    #[test]
    fn clases_por_extension() {
        assert_eq!(classify("Foto.JPG"), FileClass::Image);
        assert_eq!(classify("a.webp"), FileClass::Image);
        assert_eq!(classify("clip.mp4"), FileClass::Video);
        assert_eq!(classify("tema.flac"), FileClass::Audio);
        assert_eq!(classify("informe.pdf"), FileClass::Other);
        assert_eq!(classify("sin_extension"), FileClass::Other);
        // Un GIF va como archivo (Telegram los quiere en MP4 para animarlos).
        assert_eq!(classify("baile.gif"), FileClass::Other);
    }

    #[test]
    fn foto_comprimida_o_archivo() {
        let p = picked("a.png", 2_000_000, 1200, 800);
        assert_eq!(p.send_kind(true), MediaKind::Photo);
        assert_eq!(p.send_kind(false), MediaKind::File);
        // Más de 10 MB o una tira muy alargada: siempre como archivo.
        assert_eq!(picked("b.png", PHOTO_MAX_BYTES + 1, 1200, 800).send_kind(true), MediaKind::File);
        assert_eq!(picked("c.png", 1000, 4200, 200).send_kind(true), MediaKind::File);
        assert_eq!(picked("d.png", 1000, 0, 0).send_kind(true), MediaKind::File);
        assert_eq!(picked("v.mp4", 1000, 0, 0).send_kind(false), MediaKind::Video);
        assert_eq!(picked("m.mp3", 1000, 0, 0).send_kind(true), MediaKind::Audio);
        assert_eq!(picked("x.pdf", 1000, 0, 0).send_kind(true), MediaKind::File);
    }

    #[test]
    fn reparto_en_albumes() {
        let mut v: Vec<Picked> = (0..12).map(|i| picked(&format!("f{i}.jpg"), 1000, 800, 600)).collect();
        v.insert(3, picked("doc.pdf", 10, 0, 0));
        let g = plan(&v, true);
        // 12 fotos → 10 + 2; el PDF va aparte, después.
        assert_eq!(g.len(), 3);
        assert_eq!(g[0].len(), 10);
        assert_eq!(g[1].len(), 2);
        assert_eq!(g[2], vec![3]);
        // Como archivos: todos documentos, en orden, de 10 en 10.
        let g = plan(&v, false);
        assert_eq!(g.iter().map(Vec::len).collect::<Vec<_>>(), [10, 3]);
        assert_eq!(g[0][..4], [0, 1, 2, 3]);
        assert!(plan(&[], true).is_empty());
    }

    #[test]
    fn tamanos_legibles() {
        assert_eq!(size_label(820), "820 B");
        assert_eq!(size_label(1536), "1,5 KB");
        assert_eq!(size_label(3 * 1024 * 1024 + 200 * 1024), "3,2 MB");
        assert_eq!(size_label(250 * 1024 * 1024), "250 MB");
        assert_eq!(progress_label(512 * 1024, 2 * 1024 * 1024), "512 KB de 2,0 MB");
        assert_eq!(progress_label(10, 0), "10 B");
    }

    #[test]
    fn pie_cuenta_caracteres_no_bytes() {
        assert_eq!(caption_len("  ñandú 🦤  "), 7);
        assert_eq!(caption_len(&"é".repeat(MAX_CAPTION_CHARS)), MAX_CAPTION_CHARS);
    }

    #[test]
    fn rutas_pegadas() {
        let dir = std::env::temp_dir().join(format!("parlo-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("año nuevo.txt");
        std::fs::write(&f, b"hola").unwrap();
        let uri = format!("file://{}", f.to_string_lossy().replace(' ', "%20").replace('ñ', "%C3%B1"));
        assert_eq!(paths_from_text(&uri), Some(vec![f.clone()]));
        assert_eq!(paths_from_text(&format!("{}\n", f.display())), Some(vec![f.clone()]));
        // Texto normal, rutas relativas o que no existen: se pega como texto.
        assert_eq!(paths_from_text("hola, ¿qué tal?"), None);
        assert_eq!(paths_from_text("notas.txt"), None);
        assert_eq!(paths_from_text(&format!("{}\n/no/existe.txt", f.display())), None);
        assert_eq!(paths_from_text("file:///%ZZ"), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inspeccion_de_imagen_y_errores() {
        let dir = std::env::temp_dir().join(format!("parlo-inspect-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("foto.png");
        ::image::RgbaImage::from_pixel(1000, 500, ::image::Rgba([200, 80, 20, 255])).save(&png).unwrap();
        let p = inspect_one(&png).unwrap();
        assert_eq!((p.class, p.width, p.height), (FileClass::Image, 1000, 500));
        assert!(p.preview.is_some());
        // Extensión de imagen con bytes que no lo son: archivo sin vista previa.
        let fake = dir.join("rota.jpg");
        std::fs::write(&fake, b"no soy una foto").unwrap();
        let p = inspect_one(&fake).unwrap();
        assert_eq!(p.class, FileClass::Other);
        assert!(p.preview.is_none());
        assert!(inspect_one(&dir).unwrap_err().contains("carpeta"));
        assert!(inspect_one(&dir.join("no-esta.pdf")).is_err());
        let empty = dir.join("vacio.txt");
        std::fs::write(&empty, b"").unwrap();
        assert!(inspect_one(&empty).unwrap_err().contains("vacío"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
