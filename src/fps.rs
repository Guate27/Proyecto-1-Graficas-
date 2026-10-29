use std::time::Instant;

// Cada cuánto se recalculan los FPS 
const UPDATE_INTERVAL: f32 = 0.5;

// Cuenta cuántos frames se dibujan y calcula.
pub struct FpsCounter {
    // El momento en que empezó de medición actual.
    window_start: Instant,
    // Cuántos frames se han dibujado desde `window_start`.
    frames: u32,
    // Cantidad de FPS actual
    fps: f32,
}

impl FpsCounter {
    pub fn new() -> FpsCounter {
        FpsCounter {
            window_start: Instant::now(),
            frames: 0,
            fps: 0.0,
        }
    }

    // función que hace el conteo de fps.
    pub fn tick(&mut self) {
        self.frames += 1;
        // Tiempo trancurrido desde que se empezó la medición 
        let elapsed = self.window_start.elapsed().as_secs_f32();

        // Cálculo de los FPS en el intervalo de tiempo establecido
        if elapsed >= UPDATE_INTERVAL {
            self.fps = self.frames as f32 / elapsed;

            // Se reinicia la ventana para empezar a medir el siguiente tramo.
            self.frames = 0;
            self.window_start = Instant::now();
        }
    }

    // La canidad de FPS actual redondeada a un numero entero 
    pub fn fps(&self) -> u32 {
        self.fps.round() as u32
    }
}