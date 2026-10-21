use minifb::{Key, Window};
use nalgebra_glm::Vec2;
use std::f32::consts::PI;

use crate::maze::Maze;

pub struct Player {
    pub pos: Vec2,
    pub a: f32,
}

const MOVE_SPEED: f32 = 10.0;
const ROTATION_SPEED: f32 = PI / 150.0;
const PLAYER_RADIUS: f32 = 15.0;

fn is_walkable(maze: &Maze, x: f32, y: f32, block_size: usize) -> bool {
    if !(x >= 0.0) || !(y >= 0.0) {
        return false;
    }

    let i = x as usize / block_size;
    let j = y as usize / block_size;

    match maze.get(j).and_then(|row| row.get(i)) {
        Some(&cell) => cell == ' ' || cell == 'g' || cell == 'G',
        None => false,
    }
}

fn can_stand(maze: &Maze, x: f32, y: f32, block_size: usize) -> bool {
    is_walkable(maze, x - PLAYER_RADIUS, y, block_size)
        && is_walkable(maze, x + PLAYER_RADIUS, y, block_size)
        && is_walkable(maze, x, y - PLAYER_RADIUS, block_size)
        && is_walkable(maze, x, y + PLAYER_RADIUS, block_size)
}

pub fn process_events(window: &Window, player: &mut Player, maze: &Maze, block_size: usize) {
    if window.is_key_down(Key::A) {
        player.a -= ROTATION_SPEED;
    }

    if window.is_key_down(Key::D) {
        player.a += ROTATION_SPEED;
    }

    let mut step = 0.0;

    if window.is_key_down(Key::W) {
        step += MOVE_SPEED;
    }

    if window.is_key_down(Key::S) {
        step -= MOVE_SPEED;
    }

    if step == 0.0 {
        return;
    }

    let new_x = player.pos.x + step * player.a.cos();
    let new_y = player.pos.y + step * player.a.sin();

    if can_stand(maze, new_x, player.pos.y, block_size) {
        player.pos.x = new_x;
    }

    if can_stand(maze, player.pos.x, new_y, block_size) {
        player.pos.y = new_y;
    }
}
