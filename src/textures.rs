use std::collections::HashMap;

/// Color que los sprites usan para marcar sus píxeles transparentes.
///
/// El PNG del enemigo trae canal alfa, pero está opaco de punta a punta: el
/// fondo no es «nada», es un magenta concreto que nunca aparece en el dibujo.
/// La transparencia se decide entonces comparando el color, no leyendo alfa.
pub const TRANSPARENT: u32 = 0x980088;

/// Una textura ya decodificada y guardada en memoria: los píxeles viven en el
/// mismo formato `0xRRGGBB` que usa el `Framebuffer`, así que muestrear una
/// textura no requiere ninguna conversión durante el render.
pub struct Texture {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}



impl Texture {
    fn load(path: &str) -> Texture {
        let image = image::open(path)
            .unwrap_or_else(|e| panic!("no se pudo abrir la textura {}: {}", path, e))
            .to_rgb8();

        let width = image.width() as usize;
        let height = image.height() as usize;

        let pixels = image
            .pixels()
            .map(|p| ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | p[2] as u32)
            .collect();

        Texture {
            width,
            height,
            pixels,
        }
    }

    /// Devuelve el color en coordenadas de textura normalizadas: `u` y `v`
    /// van de 0 a 1 sin importar cuántos píxeles mida la imagen.
    pub fn sample(&self, u: f32, v: f32) -> u32 {
        let x = ((u * self.width as f32) as usize).min(self.width - 1);
        let y = ((v * self.height as f32) as usize).min(self.height - 1);

        self.pixels[y * self.width + x]
    }
}

/// Guarda una textura por cada carácter del laberinto. Las imágenes se leen
/// del disco una sola vez, antes de que arranque el ciclo de render.
pub struct TextureManager {
    textures: HashMap<char, Texture>,
    fallback: Texture,
}

impl TextureManager {
    pub fn new() -> TextureManager {
        let files = [
            ('+', "assets/wall4.png"),
            ('-', "assets/wall2.png"),
            ('|', "assets/wall1.png"),
            ('g', "assets/wall5.png"),
            ('e', "assets/sprite1.png"),
        ];

        let mut textures = HashMap::new();

        for (cell, path) in files {
            textures.insert(cell, Texture::load(path));
        }

        TextureManager {
            textures,
            fallback: Texture::load("assets/wall3.png"),
        }
    }

    /// Busca la textura de un carácter.
    ///
    /// La búsqueda se hace **una vez por pared o por sprite**, nunca por
    /// píxel: cada estaca y cada enemigo pide aquí su textura y después
    /// muestrea cientos de miles de veces sobre la referencia devuelta.
    pub fn get(&self, cell: char) -> &Texture {
        self.textures.get(&cell).unwrap_or(&self.fallback)
    }
}
