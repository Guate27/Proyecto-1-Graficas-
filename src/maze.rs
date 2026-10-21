use std::f32::consts::PI;
use std::fs::File;
use std::io::{BufRead, BufReader};

use nalgebra_glm::Vec2;

use crate::enemy::Enemy;
use crate::player::Player;

pub type Maze = Vec<Vec<char>>;

pub fn load_maze(filename: &str, block_size: usize) -> (Maze, Player, Vec<Enemy>) {
    let file = File::open(filename).expect("no se pudo abrir el archivo del laberinto");

    let reader = BufReader::new(file);

    let mut maze: Maze = Vec::new();

    let mut player_pos: Option<Vec2> = None;
    let mut enemies: Vec<Enemy> = Vec::new();

    for (row, line) in reader.lines().enumerate() {
        let line = line.expect("no se pudo leer una línea del laberinto");

        let mut cells: Vec<char> = Vec::new();

        for (col, character) in line.chars().enumerate() {
            let center_x = (col * block_size + block_size / 2) as f32;
            let center_y = (row * block_size + block_size / 2) as f32;

            match character {
                'p' => {
                    player_pos = Some(Vec2::new(center_x, center_y));

                    cells.push(' ');
                }
                // Los enemigos no son parte de la geometría: marcan una
                // posición y dejan la celda como piso transitable.
                'e' => {
                    enemies.push(Enemy::new(center_x, center_y, 'e'));

                    cells.push(' ');
                }
                _ => cells.push(character),
            }
        }

        maze.push(cells);
    }

    let player = Player {
        pos: player_pos.unwrap_or_else(|| Vec2::new(0.0, 0.0)),
        // ángulo de vista inicial; el jugador podrá girarlo con el teclado.
        a: PI / 3.0,
    };

    (maze, player, enemies)
}
