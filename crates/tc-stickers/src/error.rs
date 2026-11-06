//! Errores de decodificación de stickers.

#[derive(Debug, thiserror::Error)]
pub enum StickerError {
    #[error("imagen no válida: {0}")]
    Image(#[from] image::ImageError),
    #[error("gzip no válido: {0}")]
    Gzip(std::io::Error),
    #[error("el TGS descomprimido supera el tope de {0} bytes")]
    TooLarge(usize),
    #[error("el contenido Lottie no es JSON válido")]
    NotLottie,
    #[error("tamaño de fotograma no válido")]
    BadSize,
    #[error("soporte Lottie no compilado (feature `lottie`)")]
    LottieDisabled,
    #[error("rlottie no pudo cargar la animación")]
    LottieLoad,
    #[error("fotograma {0} fuera de rango")]
    FrameOutOfRange(usize),
}
