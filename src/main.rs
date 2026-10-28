mod caster;
mod enemy;
mod fps;
mod framebuffer;
mod maze;
mod player;
mod shading;
mod text;
mod textures;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::f32::consts::PI;
use std::time::Duration;

use crate::caster::{cast_ray, Face};
use crate::enemy::Enemy;
use crate::fps::FpsCounter;
use crate::framebuffer::Framebuffer;
use crate::maze::{load_maze, Maze};
use crate::player::{process_events, process_mouse, Player};
use crate::shading::{distance_brightness, mix, shade, HORIZONTAL_FACE_BRIGHTNESS};
use crate::text::{draw_text, fill_rect, text_height, text_width};
use crate::textures::{TextureManager, TRANSPARENT};

const BLOCK_SIZE: usize = 100;
const NUM_RAYS_2D: usize = 5;
const FOV: f32 = PI / 3.0;
const REPORT_EVERY: u64 = 60;
const REPORT_COLUMNS: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];
/// Colores del techo y del piso: A cada colot se le aplica un degradado. Cerca del jugador el techo y el piso es más claro, y a medida que se va alejando el piso y el techo en el campo de visión del jugador se va oscureciendo (la
const CEILING_NEAR: u32 = 0x3A3550;
const FLOOR_NEAR: u32 = 0x4A4856;
const HORIZON_COLOR: u32 = 0x0E0D16;
/// Distancia mínima a la que se dibuja un sprite. Más cerca que esto el
/// tamaño en pantalla se dispara y el enemigo tapa la vista entera.
const MIN_SPRITE_DISTANCE: f32 = 20.0;
/// Cuántos pixeles mide cada celda del laberinto dentro del minimapa.
const MINIMAP_CELL: usize = 8;
/// Separación entre el minimapa y el borde de la ventana.
const MINIMAP_MARGIN: usize = 10;

fn cell_color(cell: char) -> u32 {
    match cell {
        '+' => 0x00AAFF,
        '-' => 0xFF5555,
        '|' => 0xFFB000,
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
        // Qué tan lejos está cada fila de pixéles de la ventana del centro de la ventana: 1.0 en el borde de arriba o de abajo de la pantalla, 0.0 justo en la mitad.
        let closeness = ((y as f32 - half_height) / half_height).abs();

        let near_color = if (y as f32) < half_height {
            CEILING_NEAR
        } else {
            FLOOR_NEAR
        };

        framebuffer.set_current_color(mix(HORIZON_COLOR, near_color, closeness));

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

        // El brillo de una pared, piso o techo: se oscurece con la distancia
        let mut brightness = distance_brightness(distance);

        if intersect.face == Face::Horizontal {
            brightness *= HORIZONTAL_FACE_BRIGHTNESS;
        }

        // Los extremos sin recortar son los que definen la coordenada
        // vertical de la textura: si se midiera contra el pedazo visible, una
        // pared cercana mostraría la textura completa comprimida en la
        // pantalla en lugar de mostrar solo el pedazo que le toca.
        let stake_top = half_height - stake_height / 2.0;

        let first = stake_top.max(0.0) as usize;
        let last = (half_height + stake_height / 2.0).min(framebuffer.height as f32) as usize;

        if !textured {
            // Aplicación del brillo de las paredes
            framebuffer.set_current_color(shade(cell_color(intersect.impact), brightness));

            for y in first..last {
                framebuffer.point(i, y);
            }

            continue;
        }

        // La textura se busca una vez por columna, no una vez por píxel.
        let texture = texture_manager.get(intersect.impact);

        for y in first..last {
            let v = (y as f32 - stake_top) / stake_height;

            let color = texture.sample(intersect.u, v);

            framebuffer.set_current_color(shade(color, brightness));
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

    // El sprite se oscurece con la distancia igual que las paredes, para que un enemigo lejano no brille en medio de la oscuridad.
    let brightness = distance_brightness(projection.depth);

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

            framebuffer.set_current_color(shade(color, brightness));
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

/// Dibuja el contador de FPS en la esquina superior izquierda, sobre un
/// rectángulo oscuro para que se lea encima de cualquier pared.
fn draw_fps(framebuffer: &mut Framebuffer, fps: u32) {
    let text = format!("FPS: {}", fps);

    let scale = 4;
    let margin = 10;
    let padding = 6;

    let width = text_width(&text, scale) + padding * 2;
    let height = text_height(scale) + padding * 2;

    fill_rect(framebuffer, margin, margin, width, height, 0x111111);

    // Verde si va bien, amarillo si se acerca al mínimo, rojo si está debajo.
    let color = if fps >= 30 {
        0x00FF66
    } else if fps >= 15 {
        0xFFDD00
    } else {
        0xFF3333
    };

    draw_text(
        framebuffer,
        margin + padding,
        margin + padding,
        &text,
        scale,
        color,
    );
}

/// Dibuja el laberinto en miniatura en la esquina superior derecha, con la
/// posición del jugador, hacia dónde mira y los enemigos.
///
/// Es el mismo mapa que la vista 2D, pero "encogido": cada celda, que en el
/// mundo mide BLOCK_SIZE (100) pixeles, aquí mide MINIMAP_CELL (8).
fn draw_minimap(framebuffer: &mut Framebuffer, maze: &Maze, player: &Player, enemies: &[Enemy]) {
    let rows = maze.len();
    let cols = maze.iter().map(|line| line.len()).max().unwrap_or(0);

    let map_width = cols * MINIMAP_CELL;
    let map_height = rows * MINIMAP_CELL;

    // Si la ventana fuera demasiado pequeña para el minimapa, mejor no
    // dibujarlo que restar de más y provocar un error.
    if map_width + MINIMAP_MARGIN * 2 > framebuffer.width
        || map_height + MINIMAP_MARGIN * 2 > framebuffer.height
    {
        return;
    }

    // Esquina superior izquierda del minimapa: pegado a la derecha de la
    // ventana, dejando el margen. La esquina izquierda ya la usan los FPS.
    let origin_x = framebuffer.width - map_width - MINIMAP_MARGIN;
    let origin_y = MINIMAP_MARGIN;

    // Un marco claro y un fondo oscuro para que se distinga de la vista 3D.
    fill_rect(framebuffer, origin_x - 2, origin_y - 2, map_width + 4, map_height + 4, 0xDDDDDD);
    fill_rect(framebuffer, origin_x, origin_y, map_width, map_height, 0x1A1A2E);

    // Las paredes: cada celda que no es piso se pinta como un cuadrito con
    // el mismo color que usa la vista 2D.
    for (row, line) in maze.iter().enumerate() {
        for (col, &cell) in line.iter().enumerate() {
            if cell == ' ' {
                continue;
            }

            fill_rect(
                framebuffer,
                origin_x + col * MINIMAP_CELL,
                origin_y + row * MINIMAP_CELL,
                MINIMAP_CELL,
                MINIMAP_CELL,
                cell_color(cell),
            );
        }
    }

    // Para pasar de una posición del mundo a una del minimapa se multiplica
    // por esta escala: 8 / 100 = 0.08. Un punto a 150 px en el mundo queda a
    // 12 px dentro del minimapa.
    let scale = MINIMAP_CELL as f32 / BLOCK_SIZE as f32;

    for enemy in enemies {
        let ex = origin_x as f32 + enemy.pos.x * scale;
        let ey = origin_y as f32 + enemy.pos.y * scale;

        fill_rect(framebuffer, ex as usize - 1, ey as usize - 1, 3, 3, 0xFF00FF);
    }

    let px = origin_x as f32 + player.pos.x * scale;
    let py = origin_y as f32 + player.pos.y * scale;

    // Una línea corta hacia donde mira el jugador, con la misma fórmula de
    // cos y sin que se usa para caminar: se avanza un pixel a la vez.
    framebuffer.set_current_color(0xFFFFFF);

    for step in 0..12 {
        let lx = px + step as f32 * player.a.cos();
        let ly = py + step as f32 * player.a.sin();

        framebuffer.point(lx as usize, ly as usize);
    }

    // El jugador encima de todo, como un cuadrito amarillo de 4×4.
    fill_rect(framebuffer, px as usize - 2, py as usize - 2, 4, 4, 0xFFFF00);
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
    let mut fps_counter = FpsCounter::new();
    // Dónde estaba el mouse en el frame anterior; empieza vacío porque
    // antes del primer frame no hay nada con qué comparar.
    let mut last_mouse_x: Option<f32> = None;

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
        process_mouse(&window, &mut player, &mut last_mouse_x);

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

            // En la vista 3D se agrega el minimapa en la esquina; en la 2D no
            // hace falta porque ya se está viendo el mapa completo.
            draw_minimap(&mut framebuffer, &maze, &player, &enemies);
        } else {
            render_2d(&mut framebuffer, &maze, &player, &enemies);
        }

        // El contador se dibuja al final para que quede encima de todo lo
        // demás, y se avisa que se terminó un frame más.
        fps_counter.tick();
        draw_fps(&mut framebuffer, fps_counter.fps());

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}