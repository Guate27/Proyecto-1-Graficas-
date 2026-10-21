use nalgebra_glm::Vec2;

/// Un sprite colocado en el mundo. A diferencia de las paredes, que están
/// alineadas a la retícula del laberinto, un enemigo vive en una posición
/// libre y siempre se dibuja de frente al jugador.
pub struct Enemy {
    pub pos: Vec2,
    /// Carácter con el que el `TextureManager` encuentra su imagen.
    pub texture_key: char,
}

impl Enemy {
    pub fn new(x: f32, y: f32, texture_key: char) -> Enemy {
        Enemy {
            pos: Vec2::new(x, y),
            texture_key,
        }
    }
}
