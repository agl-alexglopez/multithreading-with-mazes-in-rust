pub use crate::utilities::build_util;
pub use crate::utilities::maze;

use rand::prelude::*;
use std::{thread, time};

pub fn generate_recursive_backtracker_maze(maze: &mut maze::Maze) {
    build_util::fill_maze_with_walls(maze);
    let mut gen = thread_rng();
    let start: maze::Point = maze::Point {
        row: 2 * (gen.gen_range(1..maze.row_size() - 2) / 2) + 1,
        col: 2 * (gen.gen_range(1..maze.col_size() - 2) / 2) + 1,
    };
    let mut random_direction_indices: Vec<usize> = (0..build_util::NUM_DIRECTIONS).collect();
    let mut cur: maze::Point = start;
    let mut branches_remain: bool = true;
    while branches_remain {
        random_direction_indices.shuffle(&mut gen);
        branches_remain = false;
        for i in &random_direction_indices {
            let direction: &maze::Point = &build_util::GENERATE_DIRECTIONS[*i];
            let next = maze::Point {
                row: cur.row + direction.row,
                col: cur.col + direction.col,
            };
            if build_util::can_build_new_square(maze, next) {
                branches_remain = true;
                build_util::carve_path_markings(maze, cur, next);
                cur = next;
                break;
            }
        }
        if !branches_remain && cur != start {
            let dir: build_util::BacktrackMarker = (maze[cur.row as usize][cur.col as usize]
                & build_util::MARKERS_MASK)
                >> build_util::MARKER_SHIFT;
            // The solvers will need these bits later so we need to clear bits.
            maze[cur.row as usize][cur.col as usize] &= !build_util::MARKERS_MASK;
            let backtracking: &maze::Point = &build_util::BACKTRACKING_POINTS[dir as usize];
            cur.row += backtracking.row;
            cur.col += backtracking.col;
            branches_remain = true;
        }
    }
    build_util::clear_and_flush_grid(maze);
}

pub fn animate_recursive_backtracker_maze(maze: &mut maze::Maze, speed: build_util::BuilderSpeed) {
    let animation: build_util::SpeedUnit = build_util::BUILDER_SPEEDS[speed as usize];
    build_util::fill_maze_with_walls_animated(maze);
    build_util::clear_and_flush_grid(maze);
    let mut gen = thread_rng();
    let start: maze::Point = maze::Point {
        row: 2 * (gen.gen_range(1..maze.row_size() - 2) / 2) + 1,
        col: 2 * (gen.gen_range(1..maze.col_size() - 2) / 2) + 1,
    };
    let mut random_direction_indices: Vec<usize> = (0..build_util::NUM_DIRECTIONS).collect();
    let mut cur: maze::Point = start;
    let mut branches_remain: bool = true;
    while branches_remain {
        random_direction_indices.shuffle(&mut gen);
        branches_remain = false;
        for i in &random_direction_indices {
            let direction = &build_util::GENERATE_DIRECTIONS[*i];
            let next = maze::Point {
                row: cur.row + direction.row,
                col: cur.col + direction.col,
            };
            if build_util::can_build_new_square(maze, next) {
                branches_remain = true;
                build_util::carve_path_markings_animated(maze, cur, next, animation);
                cur = next;
                break;
            }
        }
        if !branches_remain && cur != start {
            let dir: build_util::BacktrackMarker = (maze[cur.row as usize][cur.col as usize]
                & build_util::MARKERS_MASK)
                >> build_util::MARKER_SHIFT;
            // The solvers will need these bits later so we need to clear bits.
            maze[cur.row as usize][cur.col as usize] &= !build_util::MARKERS_MASK;
            let backtracking: &maze::Point = &build_util::BACKTRACKING_POINTS[dir as usize];
            build_util::flush_cursor_maze_coordinate(maze, cur);
            thread::sleep(time::Duration::from_micros(animation));
            cur.row += backtracking.row;
            cur.col += backtracking.col;
            branches_remain = true;
        }
    }
}
