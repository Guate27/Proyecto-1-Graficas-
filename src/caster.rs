use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;

/// Cara de la celda contra la que chocó el rayo. Determina cuál de las dos
/// coordenadas del punto de impacto recorre la pared.
#[derive(Clone, Copy, PartialEq)]
pub enum Face {
    /// Cara vertical en el mapa (izquierda o derecha de la celda).
    Vertical,
    /// Cara horizontal en el mapa (arriba o abajo de la celda).
    Horizontal,
}

pub struct Intersect {
    pub distance: f32,
    pub impact: char,
    /// Posición horizontal del impacto dentro de la pared, de 0 a 1,
    /// medida de izquierda a derecha desde el punto de vista del jugador.
    pub u: f32,
    pub face: Face,
}

pub fn cast_ray(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    a: f32,
    block_size: usize,
    draw_line: bool,
) -> Intersect {
    let mut d = 0.0;

    let cos_a = a.cos();
    let sin_a = a.sin();

    if draw_line {
        framebuffer.set_current_color(0xFFDDDD);
    }

    //La columna de la celda en la que estaba el rayo en el paso anterior.
    let mut previous_i = player.pos.x as usize / block_size;

    loop {
        let fx = player.pos.x + d * cos_a;
        let fy = player.pos.y + d * sin_a;

        let x = fx as usize;
        let y = fy as usize;

        let i = x / block_size;
        let j = y / block_size;

        if j >= maze.len() || i >= maze[j].len() {
            return Intersect {
                distance: d,
                impact: ' ',
                u: 0.0,
                face: Face::Vertical,
            };
        }

        if maze[j][i] != ' ' {
            let face = if i != previous_i {
                Face::Vertical
            } else {
                Face::Horizontal
            };

            let u = hit_coordinate(fx, fy, i, j, block_size, cos_a, sin_a, face);

            return Intersect {
                distance: d,
                impact: maze[j][i],
                u,
                face,
            };
        }

        if draw_line {
            framebuffer.point(x, y);
        }

        // Se actualiza la columna anterior antes de avanzar al siguiente paso
        previous_i = i;

        d += 1.0;
    }
}

// Calcula la coordenada horizontal de la textura (u) en el punto de impacto.
// La cara ya se decidió en cast_ray con previous_i: en una cara vertical la pared
// corre a lo largo del eje y, y en una horizontal a lo largo del eje x.
fn hit_coordinate(
    fx: f32,
    fy: f32,
    i: usize,
    j: usize,
    block_size: usize,
    cos_a: f32,
    sin_a: f32,
    face: Face,
) -> f32 {
    let block = block_size as f32;

    // Posición del impacto dentro de la celda (de 0 a 100)
    let hit_x = fx - (i * block_size) as f32;
    let hit_y = fy - (j * block_size) as f32;

    match face {
        // Mirando hacia el este (cos > 0) la textura avanza con y; mirando al oeste, al revés.
        Face::Vertical => {
            let u = hit_y / block;
            if cos_a > 0.0 { u } else { 1.0 - u }
        }
        // Mirando hacia el sur (sin > 0) la textura avanza al revés de x.
        Face::Horizontal => {
            let u = hit_x / block;
            if sin_a > 0.0 { 1.0 - u } else { u }
        }
    }
}