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
            let (u, face) = hit_coordinate(fx, fy, i, j, block_size, cos_a, sin_a);

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

        d += 1.0;
    }
}

/// Traduce el punto de impacto a una coordenada horizontal de textura.
///
/// El rayo avanza de un píxel a la vez, así que al detectarse el choque el
/// punto ya está *dentro* de la celda, pero apenas cruzando una de sus caras:
/// una de las dos coordenadas locales queda pegada a la orilla y la otra
/// puede valer cualquier cosa. La que está pegada a la orilla dice qué cara
/// se cruzó; la otra es la que recorre la pared y sirve de coordenada.
fn hit_coordinate(
    fx: f32,
    fy: f32,
    i: usize,
    j: usize,
    block_size: usize,
    cos_a: f32,
    sin_a: f32,
) -> (f32, Face) {
    let block = block_size as f32;

    let hit_x = fx - (i * block_size) as f32;
    let hit_y = fy - (j * block_size) as f32;

    let edge_x = hit_x.min(block - hit_x);
    let edge_y = hit_y.min(block - hit_y);

    if edge_x < edge_y {
        // Se cruzó una cara vertical: la pared corre a lo largo del eje y.
        // Mirando hacia el este (cos > 0) la derecha del jugador es el sur,
        // así que la textura avanza con y; mirando al oeste, al revés.
        let u = hit_y / block;
        let u = if cos_a > 0.0 { u } else { 1.0 - u };
        (u, Face::Vertical)
    } else {
        // Se cruzó una cara horizontal: la pared corre a lo largo del eje x.
        // Mirando hacia el sur (sin > 0) la derecha del jugador es el oeste.
        let u = hit_x / block;
        let u = if sin_a > 0.0 { 1.0 - u } else { u };
        (u, Face::Horizontal)
    }
}
