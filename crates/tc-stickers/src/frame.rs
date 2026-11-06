//! Fotograma RGBA8 sin premultiplicar (compatible con `iced::widget::image::Handle::from_rgba`).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Frame {
    /// Fotograma transparente.
    pub fn transparent(width: u32, height: u32) -> Self {
        Self { width, height, rgba: vec![0; width as usize * height as usize * 4] }
    }

    pub fn byte_len(&self) -> usize {
        self.rgba.len()
    }

    /// Píxel `[r, g, b, a]`; `None` fuera de rango.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.rgba.get(i..i + 4).map(|p| [p[0], p[1], p[2], p[3]])
    }
}
