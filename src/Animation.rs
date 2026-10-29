use crate::textures::Texture;

// Animación de los sprites (imágenes 2D): varias imágenes que se van mostrando una por una en bucle rápidamente
pub struct Animation {
    frames: Vec<Texture>, // Lista de imágenes que forma la animación ordenadas en la secuencia en que se muestran
    frame_duration: f32,  // Segundos que dura cada frame con  sprite (imagen 2D) en pantalla
}

impl Animation {
    // Guarda a todas las imágenes de la animación guardadas previamente en memoria
    pub fn load(paths: &[&str], frame_duration: f32) -> Animation {
        let mut frames = Vec::new();

        for path in paths {
            frames.push(Texture::load(path));
        }

        Animation {
            frames,
            frame_duration,
        }
    }

    // Calcula la siguiente imagen que toca mostrar después de "times" segundos
    pub fn frame_at(&self, time: f32) -> &Texture {
        // Cálculo de cuántas imágenes de la animación
        let frames_passed = (time / self.frame_duration) as usize;

        //Cálculo de la siguiente imágen de la animación que se va a mostrar
        let index = frames_passed % self.frames.len();

        &self.frames[index]
    }
}