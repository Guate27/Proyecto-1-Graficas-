mod caster;
mod enemy;
mod framebuffer;
mod maze;
mod player;
mod textures;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::f32::consts::PI;
use std::time::Duration;

use crate::caster::{cast_ray, Face};
use crate::enemy::Enemy;
use crate::framebuffer::Framebuffer;
use crate::maze::{load_maze, Maze};
use crate::player::{process_events, Player};
use crate::textures::{TextureManager, TRANSPARENT};

const BLOCK_SIZE: usize = 100;
const NUM_RAYS_2D: usize = 5;
const FOV: f32 = PI / 3.0;
const REPORT_EVERY: u64 = 60;
const REPORT_COLUMNS: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];
const SKY_COLOR: u32 = 0x87CEEB;
const FLOOR_COLOR: u32 = 0x5A5A5A;
/// Distancia mínima a la que se dibuja un sprite. Más cerca que esto el
/// tamaño en pantalla se dispara y el enemigo tapa la vista entera.
const MIN_SPRITE_DISTANCE: f32 = 20.0;

fn cell_color(cell: char) -> u32 {
    match cell {
        '+' => 0x00AAFF,
        '-' => 0xFF5555,
        '|' => 0xFF5555,
        'g' | 'G' => 0x00FF00,
        _ => 0xFFDDDD,
    }
}

fn draw_cell(framebuffer: &mut Framebuffer, xo: usize, yo: usize, cell: char) {
    if cell == ' ' {
        return;
    }

    framebuffer.set_current_color(cell_color(cell));

    for x in xo..xo + BLOCK_SIZE {
        for y in yo..yo + BLOCK_SIZE {
            framebuffer.point(x, y);
        }
    }
}

fn ray_angle(player: &Player, i: usize, num_rays: usize) -> f32 {
    let ray_fraction = i as f32 / (num_rays - 1) as f32;
    player.a - FOV / 2.0 + FOV * ray_fraction
}

fn projection_plane_distance(width: usize) -> f32 {
    (width as f32 / 2.0) / (FOV / 2.0).tan()
}

fn draw_dot(framebuffer: &mut Framebuffer, x: f32, y: f32, radius: usize, color: u32) {
    framebuffer.set_current_color(color);

    let cx = x as usize;
    let cy = y as usize;

    for x in cx.saturating_sub(radius)..=cx + radius {
        for y in cy.saturating_sub(radius)..=cy + radius {
            framebuffer.point(x, y);
        }
    }
}

fn render_2d(framebuffer: &mut Framebuffer, maze: &Maze, player: &Player, enemies: &[Enemy]) {
    for (row, line) in maze.iter().enumerate() {
        for (col, &cell) in line.iter().enumerate() {
            draw_cell(framebuffer, col * BLOCK_SIZE, row * BLOCK_SIZE, cell);
        }
    }

    for enemy in enemies {
        draw_dot(framebuffer, enemy.pos.x, enemy.pos.y, 5, 0xFF00FF);
    }

    draw_dot(framebuffer, player.pos.x, player.pos.y, 3, 0xFFFF00);

    for i in 0..NUM_RAYS_2D {
        let angle = ray_angle(player, i, NUM_RAYS_2D);
        cast_ray(framebuffer, maze, player, angle, BLOCK_SIZE, true);
    }
}

fn render_world(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    texture_manager: &TextureManager,
    depth: &mut [f32],
    fisheye_correction: bool,
    textured: bool,
) {
    let num_rays = framebuffer.width;
    let half_height = framebuffer.height as f32 / 2.0;
    let plane_distance = projection_plane_distance(framebuffer.width);

    for y in 0..framebuffer.height {
        framebuffer.set_current_color(if (y as f32) < half_height {
            SKY_COLOR
        } else {
            FLOOR_COLOR
        });

        for x in 0..framebuffer.width {
            framebuffer.point(x, y);
        }
    }

    for i in 0..num_rays {
        let angle = ray_angle(player, i, num_rays);
        let intersect = cast_ray(framebuffer, maze, player, angle, BLOCK_SIZE, false);

        if intersect.impact == ' ' {
            // Sin pared en esta columna no hay nada que pueda tapar un sprite.
            depth[i] = f32::INFINITY;

            continue;
        }

        let distance = if fisheye_correction {
            intersect.distance * (angle - player.a).cos()
        } else {
            intersect.distance
        };

        let distance = distance.max(1.0);

        // Se anota a qué distancia quedó la pared de esta columna: es contra
        // este valor que después se decide si un sprite se ve o queda tapado.
        depth[i] = distance;

        let stake_height = (BLOCK_SIZE as f32 / distance) * plane_distance;

        // Los extremos sin recortar son los que definen la coordenada
        // vertical de la textura: si se midiera contra el pedazo visible, una
        // pared cercana mostraría la textura completa comprimida en la
        // pantalla en lugar de mostrar solo el pedazo que le toca.
        let stake_top = half_height - stake_height / 2.0;

        let first = stake_top.max(0.0) as usize;
        let last = (half_height + stake_height / 2.0).min(framebuffer.height as f32) as usize;

        if !textured {
            framebuffer.set_current_color(cell_color(intersect.impact));

            for y in first..last {
                framebuffer.point(i, y);
            }

            continue;
        }

        // La textura se busca una vez por columna, no una vez por píxel.
        let texture = texture_manager.get(intersect.impact);

        for y in first..last {
            let v = (y as f32 - stake_top) / stake_height;

            framebuffer.set_current_color(texture.sample(intersect.u, v));
            framebuffer.point(i, y);
        }
    }
}

/// Devuelve un ángulo equivalente dentro de `-π..π`.
///
/// La resta de dos ángulos puede dar un valor fuera de ese rango, y sin
/// normalizarlo un enemigo que está justo enfrente parecería estar a casi una
/// vuelta completa de distancia angular.
fn normalize_angle(angle: f32) -> f32 {
    let full_turn = 2.0 * PI;

    let angle = angle % full_turn;

    if angle > PI {
        angle - full_turn
    } else if angle < -PI {
        angle + full_turn
    } else {
        angle
    }
}

/// Datos de un enemigo ya proyectados a la pantalla.
struct SpriteProjection {
    /// Distancia perpendicular a la dirección de vista; es la que se compara
    /// contra el buffer de profundidad para saber si una pared lo tapa.
    depth: f32,
    /// Lado del cuadrado que ocupa el sprite, en píxeles.
    size: f32,
    left: f32,
    top: f32,
}

/// Proyecta un enemigo a la pantalla, o `None` si no puede verse.
fn project_sprite(
    framebuffer: &Framebuffer,
    player: &Player,
    enemy: &Enemy,
    fisheye_correction: bool,
) -> Option<SpriteProjection> {
    let dx = enemy.pos.x - player.pos.x;
    let dy = enemy.pos.y - player.pos.y;

    let distance = (dx * dx + dy * dy).sqrt();

    if distance < MIN_SPRITE_DISTANCE {
        return None;
    }

    let offset = normalize_angle(dy.atan2(dx) - player.a);

    // Un enemigo ocupa lo mismo que una celda del laberinto, así que sus
    // bordes quedan a media celda de su centro. Visto desde el jugador eso se
    // traduce en un medio ancho angular que crece conforme se acerca.
    let half_angle = (BLOCK_SIZE as f32 / 2.0).atan2(distance);

    // Si ni siquiera el borde más cercano al centro de la vista entra en el
    // campo de visión, el enemigo está al costado o a la espalda.
    if offset.abs() - half_angle >= FOV / 2.0 {
        return None;
    }

    // La misma distancia perpendicular que se usa para las paredes, para que
    // un enemigo y una pared a la misma distancia se midan con la misma vara.
    let depth = if fisheye_correction {
        distance * offset.cos()
    } else {
        distance
    };

    // Un enemigo al costado tiene distancia perpendicular casi nula: se acota
    // para que la comparación contra el buffer de profundidad siga teniendo
    // sentido en lugar de volverse cero o negativa.
    let depth = depth.max(1.0);

    // Los rayos se reparten linealmente en el ángulo, así que una columna sale
    // de interpolar linealmente el desvío dentro del FOV.
    let column = |angle: f32| (angle / FOV + 0.5) * (framebuffer.width - 1) as f32;

    // El tamaño se mide proyectando los **dos bordes** del enemigo. Deducirlo
    // de la distancia perpendicular sería un error: esa distancia tiende a
    // cero cuando el enemigo pasa por el costado, y el sprite se dispararía
    // hasta tapar la pantalla completa durante los cuadros en que cruza.
    let left = column(offset - half_angle);
    let size = column(offset + half_angle) - left;

    let center_y = framebuffer.height as f32 / 2.0;

    Some(SpriteProjection {
        depth,
        size,
        left,
        top: center_y - size / 2.0,
    })
}

fn draw_sprite(
    framebuffer: &mut Framebuffer,
    player: &Player,
    enemy: &Enemy,
    texture_manager: &TextureManager,
    depth: &[f32],
    fisheye_correction: bool,
) {
    let Some(projection) = project_sprite(framebuffer, player, enemy, fisheye_correction) else {
        return;
    };

    let first_x = projection.left.max(0.0) as usize;
    let last_x = (projection.left + projection.size).min(framebuffer.width as f32);

    if last_x <= 0.0 {
        return;
    }

    let last_x = last_x as usize;

    let first_y = projection.top.max(0.0) as usize;
    let last_y = (projection.top + projection.size).min(framebuffer.height as f32);

    if last_y <= 0.0 {
        return;
    }

    let last_y = last_y as usize;

    // La textura se busca una vez por sprite, no una vez por píxel.
    let texture = texture_manager.get(enemy.texture_key);

    for x in first_x..last_x {
        // Prueba de profundidad: si la pared de esta columna está más cerca
        // que el enemigo, el enemigo queda detrás de ella y no se dibuja.
        if depth[x] <= projection.depth {
            continue;
        }

        let u = (x as f32 - projection.left) / projection.size;

        for y in first_y..last_y {
            let v = (y as f32 - projection.top) / projection.size;

            let color = texture.sample(u, v);

            if color == TRANSPARENT {
                continue;
            }

            framebuffer.set_current_color(color);
            framebuffer.point(x, y);
        }
    }
}

fn render_enemies(
    framebuffer: &mut Framebuffer,
    player: &Player,
    enemies: &[Enemy],
    texture_manager: &TextureManager,
    depth: &[f32],
    fisheye_correction: bool,
) {
    // De atrás hacia adelante: el buffer de profundidad resuelve qué tapan
    // las paredes, pero entre sprites el orden de dibujo es lo único que
    // decide cuál queda encima.
    let mut order: Vec<&Enemy> = enemies.iter().collect();

    order.sort_by(|a, b| {
        let da = (a.pos - player.pos).norm();
        let db = (b.pos - player.pos).norm();

        db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
    });

    for enemy in order {
        draw_sprite(
            framebuffer,
            player,
            enemy,
            texture_manager,
            depth,
            fisheye_correction,
        );
    }
}

fn print_report(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    enemies: &[Enemy],
    depth: &[f32],
    fisheye_correction: bool,
) {
    let num_rays = framebuffer.width;
    let half_height = framebuffer.height as f32 / 2.0;
    let plane_distance = projection_plane_distance(framebuffer.width);

    println!(
        "\njugador ({:.0}, {:.0})   vista {:.1}°   ojo de pez: {}",
        player.pos.x,
        player.pos.y,
        player.a.to_degrees(),
        if fisheye_correction {
            "corregido"
        } else {
            "sin corregir"
        }
    );
    println!(
        "  columna    desvío   distancia   corregida    altura    arriba     abajo   pared   cara         u"
    );

    for fraction in REPORT_COLUMNS {
        let i = ((num_rays - 1) as f32 * fraction) as usize;
        let angle = ray_angle(player, i, num_rays);
        let intersect = cast_ray(framebuffer, maze, player, angle, BLOCK_SIZE, false);

        let corrected = intersect.distance * (angle - player.a).cos();
        let distance = if fisheye_correction {
            corrected
        } else {
            intersect.distance
        };
        let stake_height = (BLOCK_SIZE as f32 / distance.max(1.0)) * plane_distance;

        let stake_top = (half_height - stake_height / 2.0).max(0.0) as usize;
        let stake_bottom =
            (half_height + stake_height / 2.0).min(framebuffer.height as f32) as usize;

        println!(
            "  {:>7}   {:>6.1}°   {:>9.1}   {:>9.1}   {:>7.1}   {:>7}   {:>7}   {:>5}   {:<10}   {:>5.3}",
            i,
            (angle - player.a).to_degrees(),
            intersect.distance,
            corrected,
            stake_height,
            stake_top,
            stake_bottom,
            if intersect.impact == ' ' {
                '·'
            } else {
                intersect.impact
            },
            match intersect.face {
                Face::Vertical => "vertical",
                Face::Horizontal => "horizontal",
            },
            intersect.u
        );
    }

    if enemies.is_empty() {
        return;
    }

    println!("  enemigo   distancia      desvío    columna    tamaño   estado");

    for (n, enemy) in enemies.iter().enumerate() {
        let distance = (enemy.pos - player.pos).norm();
        let offset = normalize_angle((enemy.pos.y - player.pos.y).atan2(enemy.pos.x - player.pos.x) - player.a);

        match project_sprite(framebuffer, player, enemy, fisheye_correction) {
            None => println!(
                "  {:>7}   {:>9.1}   {:>9.1}°   {:>7}   {:>7}   fuera de vista",
                n, distance, offset.to_degrees(), "—", "—"
            ),
            Some(projection) => {
                let center = projection.left + projection.size / 2.0;

                // Cuántas de sus columnas sobreviven la prueba de profundidad.
                let first = projection.left.max(0.0) as usize;
                let last = (projection.left + projection.size).min(num_rays as f32).max(0.0) as usize;

                let visible = (first..last).filter(|&x| depth[x] > projection.depth).count();
                let total = last.saturating_sub(first);

                println!(
                    "  {:>7}   {:>9.1}   {:>9.1}°   {:>7.0}   {:>7.0}   {}",
                    n,
                    projection.depth,
                    offset.to_degrees(),
                    center,
                    projection.size,
                    if total == 0 {
                        "fuera de pantalla".to_string()
                    } else if visible == 0 {
                        "tapado por pared".to_string()
                    } else {
                        format!("{} de {} columnas visibles", visible, total)
                    }
                );
            }
        }
    }
}

fn main() {
    let window_width = 1300;
    let window_height = 900;
    let framebuffer_width = 1300;
    let framebuffer_height = 900;
    let frame_delay = Duration::from_millis(16);

    let (maze, mut player, enemies) = load_maze("./maze.txt", BLOCK_SIZE);

    // Una entrada por columna de la pantalla: la distancia a la pared que se
    // dibujó ahí. Los sprites la consultan para saber si quedan tapados.
    let mut depth = vec![f32::INFINITY; framebuffer_width];

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
    framebuffer.set_background_color(0x333355);

    let mut window = Window::new(
        "Maze Runner",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();

    // Las texturas se leen del disco una sola vez, antes del ciclo de render.
    let texture_manager = TextureManager::new();

    let mut mode_3d = true;
    let mut fisheye_correction = true;
    let mut report_enabled = true;
    let mut textured = true;
    let mut title_dirty = true;
    let mut frame: u64 = 0;

    println!(
        "altura = (BLOCK_SIZE / distancia) * distancia_al_plano_de_proyección\n\
         BLOCK_SIZE = {}   FOV = {:.1}°   distancia_al_plano = {:.1}",
        BLOCK_SIZE,
        FOV.to_degrees(),
        projection_plane_distance(framebuffer_width)
    );

    while window.is_open() && !window.is_key_down(Key::Escape) {
        frame += 1;

        process_events(&window, &mut player, &maze, BLOCK_SIZE);

        if window.is_key_pressed(Key::M, KeyRepeat::No) {
            mode_3d = !mode_3d;
            title_dirty = true;
        }

        if window.is_key_pressed(Key::F, KeyRepeat::No) {
            fisheye_correction = !fisheye_correction;
            title_dirty = true;
        }

        if window.is_key_pressed(Key::P, KeyRepeat::No) {
            report_enabled = !report_enabled;
            title_dirty = true;
        }

        if window.is_key_pressed(Key::T, KeyRepeat::No) {
            textured = !textured;
            title_dirty = true;
        }

        if title_dirty {
            window.set_title(&format!(
                "Maze Runner — vista: {} (M) — texturas: {} (T) — ojo de pez: {} (F) — consola: {} (P)",
                if mode_3d { "3D" } else { "2D" },
                if textured { "sí" } else { "no" },
                if fisheye_correction {
                    "corregido"
                } else {
                    "sin corregir"
                },
                if report_enabled { "activa" } else { "apagada" },
            ));
            title_dirty = false;
        }

        if report_enabled && frame % REPORT_EVERY == 0 {
            print_report(
                &mut framebuffer,
                &maze,
                &player,
                &enemies,
                &depth,
                fisheye_correction,
            );
        }

        let i = player.pos.x as usize / BLOCK_SIZE;
        let j = player.pos.y as usize / BLOCK_SIZE;
        if maze.get(j).and_then(|row| row.get(i)) == Some(&'g') {
            println!("¡Meta alcanzada! Fin del juego.");
            break;
        }

        framebuffer.clear();

        if mode_3d {
            render_world(
                &mut framebuffer,
                &maze,
                &player,
                &texture_manager,
                &mut depth,
                fisheye_correction,
                textured,
            );

            render_enemies(
                &mut framebuffer,
                &player,
                &enemies,
                &texture_manager,
                &depth,
                fisheye_correction,
            );
        } else {
            render_2d(&mut framebuffer, &maze, &player, &enemies);
        }

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}
