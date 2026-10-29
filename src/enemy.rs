use nalgebra_glm::Vec2;

// Integración de un enemigo (sprite: imagen 2D) en el mapa. Siempre se dibuja de frente al jugador.
pub struct Enemy {
    pub pos: Vec2, // Posición del enemigo en píxeles del mapa
}

impl Enemy {
    pub fn new(x: f32, y: f32) -> Enemy {
        Enemy {
            pos: Vec2::new(x, y),
        }
    }
}
