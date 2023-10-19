use crate::solve::{self, THREAD_MASKS};
use maze;
use print;
use speed;

use rand::prelude::*;
use std::{thread, time};

// Public Solver Functions-------------------------------------------------------------------------

pub fn hunt(monitor: solve::SolverMonitor) {
    let all_start: maze::Point = if let Ok(mut lk) = monitor.lock() {
        let all_start = solve::pick_random_point(&lk.maze);
        lk.maze[all_start.row as usize][all_start.col as usize] |= solve::START_BIT;
        let finish: maze::Point = solve::pick_random_point(&lk.maze);
        lk.maze[finish.row as usize][finish.col as usize] |= solve::FINISH_BIT;
        all_start
    } else {
        print::maze_panic!("Solve thread panic!");
    };

    let mut handles = Vec::with_capacity(solve::NUM_THREADS - 1);
    for (i_thread, &mask) in solve::THREAD_MASKS.iter().skip(1).enumerate() {
        let monitor_clone = monitor.clone();
        handles.push(thread::spawn(move || {
            hunter(
                monitor_clone,
                solve::ThreadGuide {
                    index: i_thread + 1,
                    paint: mask,
                    start: all_start,
                    speed: 0,
                },
            );
        }));
    }
    hunter(
        monitor.clone(),
        solve::ThreadGuide {
            index: 0,
            paint: solve::THREAD_MASKS[0],
            start: all_start,
            speed: 0,
        },
    );
    for handle in handles {
        handle.join().unwrap();
    }
    if let Ok(lk) = monitor.lock() {
        solve::print_paths(&lk.maze);
        return;
    }
    print::maze_panic!("Solve thread panic!");
}

pub fn gather(monitor: solve::SolverMonitor) {
    let all_start: maze::Point = if let Ok(mut lk) = monitor.lock() {
        let all_start = solve::pick_random_point(&lk.maze);
        lk.maze[all_start.row as usize][all_start.col as usize] |= solve::START_BIT;
        for _ in 0..solve::NUM_GATHER_FINISHES {
            let finish: maze::Point = solve::pick_random_point(&lk.maze);
            lk.maze[finish.row as usize][finish.col as usize] |= solve::FINISH_BIT;
        }
        all_start
    } else {
        print::maze_panic!("Solve thread panic!");
    };

    let mut handles = Vec::with_capacity(solve::NUM_THREADS - 1);
    for (i_thread, &mask) in solve::THREAD_MASKS.iter().skip(1).enumerate() {
        let monitor_clone = monitor.clone();
        handles.push(thread::spawn(move || {
            gatherer(
                monitor_clone,
                solve::ThreadGuide {
                    index: i_thread + 1,
                    paint: mask,
                    start: all_start,
                    speed: 0,
                },
            );
        }));
    }
    gatherer(
        monitor.clone(),
        solve::ThreadGuide {
            index: 0,
            paint: solve::THREAD_MASKS[0],
            start: all_start,
            speed: 0,
        },
    );
    for handle in handles {
        handle.join().unwrap();
    }
    if let Ok(lk) = monitor.lock() {
        solve::print_paths(&lk.maze);
        return;
    }
    print::maze_panic!("Solve thread panic!");
}

pub fn corner(monitor: solve::SolverMonitor) {
    let mut corner_starts: [maze::Point; 4] = if let Ok(mut lk) = monitor.lock() {
        let corner_starts = solve::set_corner_starts(&lk.maze);
        for p in corner_starts {
            lk.maze[p.row as usize][p.col as usize] |= solve::START_BIT;
        }

        let finish = maze::Point {
            row: lk.maze.row_size() / 2,
            col: lk.maze.col_size() / 2,
        };
        for d in maze::ALL_DIRECTIONS {
            let next = maze::Point {
                row: finish.row + d.row,
                col: finish.col + d.col,
            };
            lk.maze[next.row as usize][next.col as usize] |= maze::PATH_BIT;
        }
        lk.maze[finish.row as usize][finish.col as usize] |= maze::PATH_BIT;
        lk.maze[finish.row as usize][finish.col as usize] |= solve::FINISH_BIT;
        corner_starts
    } else {
        print::maze_panic!("Thread panic.");
    };

    corner_starts.shuffle(&mut thread_rng());
    let mut handles = Vec::with_capacity(solve::NUM_THREADS - 1);
    for (i_thread, &mask) in solve::THREAD_MASKS.iter().skip(1).enumerate() {
        let monitor_clone = monitor.clone();
        handles.push(thread::spawn(move || {
            hunter(
                monitor_clone,
                solve::ThreadGuide {
                    index: i_thread + 1,
                    paint: mask,
                    start: corner_starts[i_thread + 1],
                    speed: 0,
                },
            );
        }));
    }
    hunter(
        monitor.clone(),
        solve::ThreadGuide {
            index: 0,
            paint: THREAD_MASKS[0],
            start: corner_starts[0],
            speed: 0,
        },
    );
    for handle in handles {
        handle.join().unwrap();
    }
    if let Ok(lk) = monitor.lock() {
        solve::print_paths(&lk.maze);
        return;
    }
    print::maze_panic!("Solve thread panic!");
}

pub fn animate_hunt(monitor: solve::SolverMonitor, speed: speed::Speed) {
    let animation = solve::SOLVER_SPEEDS[speed as usize];
    let all_start: maze::Point = if let Ok(mut lk) = monitor.lock() {
        if lk.maze.exit() {
            return;
        }
        let all_start = solve::pick_random_point(&lk.maze);
        lk.maze[all_start.row as usize][all_start.col as usize] |= solve::START_BIT;
        let finish: maze::Point = solve::pick_random_point(&lk.maze);
        lk.maze[finish.row as usize][finish.col as usize] |= solve::FINISH_BIT;
        solve::flush_cursor_path_coordinate(&lk.maze, finish);
        thread::sleep(time::Duration::from_micros(animation));
        all_start
    } else {
        print::maze_panic!("Thread panic.");
    };

    let mut handles = Vec::with_capacity(solve::NUM_THREADS - 1);
    for (i_thread, &mask) in solve::THREAD_MASKS.iter().skip(1).enumerate() {
        let monitor_clone = monitor.clone();
        handles.push(thread::spawn(move || {
            animated_hunter(
                monitor_clone,
                solve::ThreadGuide {
                    index: i_thread + 1,
                    paint: mask,
                    start: all_start,
                    speed: animation,
                },
            );
        }));
    }
    animated_hunter(
        monitor.clone(),
        solve::ThreadGuide {
            index: 0,
            paint: THREAD_MASKS[0],
            start: all_start,
            speed: animation,
        },
    );
    for handle in handles {
        handle.join().unwrap();
    }
}

pub fn animate_gather(monitor: solve::SolverMonitor, speed: speed::Speed) {
    let animation = solve::SOLVER_SPEEDS[speed as usize];
    let all_start: maze::Point = if let Ok(mut lk) = monitor.lock() {
        if lk.maze.exit() {
            return;
        }
        let all_start = solve::pick_random_point(&lk.maze);
        lk.maze[all_start.row as usize][all_start.col as usize] |= solve::START_BIT;

        for _ in 0..solve::NUM_GATHER_FINISHES {
            let finish: maze::Point = solve::pick_random_point(&lk.maze);
            lk.maze[finish.row as usize][finish.col as usize] |= solve::FINISH_BIT;
            solve::flush_cursor_path_coordinate(&lk.maze, finish);
            thread::sleep(time::Duration::from_micros(animation));
        }
        all_start
    } else {
        print::maze_panic!("Thread panic.");
    };

    let mut handles = Vec::with_capacity(solve::NUM_THREADS - 1);
    for (i_thread, &mask) in solve::THREAD_MASKS.iter().skip(1).enumerate() {
        let monitor_clone = monitor.clone();
        handles.push(thread::spawn(move || {
            animated_gatherer(
                monitor_clone,
                solve::ThreadGuide {
                    index: i_thread + 1,
                    paint: mask,
                    start: all_start,
                    speed: animation,
                },
            );
        }));
    }
    animated_gatherer(
        monitor,
        solve::ThreadGuide {
            index: 0,
            paint: THREAD_MASKS[0],
            start: all_start,
            speed: animation,
        },
    );

    for handle in handles {
        handle.join().unwrap();
    }
}

pub fn animate_corner(monitor: solve::SolverMonitor, speed: speed::Speed) {
    let animation = solve::SOLVER_SPEEDS[speed as usize];
    let mut corner_starts: [maze::Point; 4] = if let Ok(mut lk) = monitor.lock() {
        if lk.maze.exit() {
            return;
        }
        let corner_starts = solve::set_corner_starts(&lk.maze);
        for p in corner_starts {
            lk.maze[p.row as usize][p.col as usize] |= solve::START_BIT;
            solve::flush_cursor_path_coordinate(&lk.maze, p);
            thread::sleep(time::Duration::from_micros(animation));
        }

        let finish = maze::Point {
            row: lk.maze.row_size() / 2,
            col: lk.maze.col_size() / 2,
        };
        for d in maze::ALL_DIRECTIONS {
            let next = maze::Point {
                row: finish.row + d.row,
                col: finish.col + d.col,
            };
            lk.maze[next.row as usize][next.col as usize] |= maze::PATH_BIT;
            solve::flush_cursor_path_coordinate(&lk.maze, next);
            thread::sleep(time::Duration::from_micros(animation));
        }
        lk.maze[finish.row as usize][finish.col as usize] |= maze::PATH_BIT;
        lk.maze[finish.row as usize][finish.col as usize] |= solve::FINISH_BIT;
        solve::flush_cursor_path_coordinate(&lk.maze, finish);
        thread::sleep(time::Duration::from_micros(animation));
        corner_starts
    } else {
        print::maze_panic!("Thread panic.");
    };

    corner_starts.shuffle(&mut thread_rng());
    let mut handles = Vec::with_capacity(solve::NUM_THREADS - 1);
    for (i_thread, &mask) in solve::THREAD_MASKS.iter().skip(1).enumerate() {
        let monitor_clone = monitor.clone();
        handles.push(thread::spawn(move || {
            animated_hunter(
                monitor_clone,
                solve::ThreadGuide {
                    index: i_thread + 1,
                    paint: mask,
                    start: corner_starts[i_thread + 1],
                    speed: animation,
                },
            );
        }));
    }
    animated_hunter(
        monitor,
        solve::ThreadGuide {
            index: 0,
            paint: solve::THREAD_MASKS[0],
            start: corner_starts[0],
            speed: animation,
        },
    );
    for handle in handles {
        handle.join().unwrap();
    }
}

// Dispatch Functions for each Thread--------------------------------------------------------------

fn hunter(monitor: solve::SolverMonitor, guide: solve::ThreadGuide) {
    let seen: solve::ThreadCache = guide.paint << solve::THREAD_TAG_OFFSET;
    let mut dfs: Vec<maze::Point> = Vec::with_capacity(solve::INITIAL_PATH_LEN);
    dfs.push(guide.start);

    let mut rng = thread_rng();
    let mut rng_arr: Vec<usize> = (0..solve::NUM_DIRECTIONS).collect();
    'branching: while let Some(&cur) = dfs.last() {
        if let Ok(mut lk) = monitor.lock() {
            if lk.win.is_some() {
                for p in dfs {
                    lk.maze[p.row as usize][p.col as usize] |= guide.paint;
                }
                return;
            }
            if (lk.maze[cur.row as usize][cur.col as usize] & solve::FINISH_BIT) != 0 {
                lk.win.get_or_insert(guide.index);
                for p in dfs {
                    if (lk.maze[p.row as usize][p.col as usize] & solve::FINISH_BIT) == 0 {
                        lk.maze[p.row as usize][p.col as usize] |= guide.paint;
                    }
                }
                return;
            }
            lk.maze[cur.row as usize][cur.col as usize] |= seen;
        } else {
            print::maze_panic!("Solve thread panic!");
        }

        // Bias threads towards their original dispatch direction with do-while loop.
        rng_arr.shuffle(&mut rng);
        for &i in &rng_arr {
            let p: &maze::Point = &maze::CARDINAL_DIRECTIONS[i];
            let next = maze::Point {
                row: cur.row + p.row,
                col: cur.col + p.col,
            };

            if match monitor.lock() {
                Ok(lk) => {
                    (lk.maze[next.row as usize][next.col as usize] & seen) == 0
                        && (lk.maze[next.row as usize][next.col as usize] & maze::PATH_BIT) != 0
                }
                Err(p) => print::maze_panic!("Solve thread panic: {}", p),
            } {
                dfs.push(next);
                continue 'branching;
            }
        }
        dfs.pop();
    }
}

fn animated_hunter(monitor: solve::SolverMonitor, guide: solve::ThreadGuide) {
    let seen: solve::ThreadCache = guide.paint << solve::THREAD_TAG_OFFSET;
    let mut dfs: Vec<maze::Point> = Vec::with_capacity(solve::INITIAL_PATH_LEN);
    dfs.push(guide.start);
    let mut rng = thread_rng();
    let mut rng_arr: Vec<usize> = (0..solve::NUM_DIRECTIONS).collect();
    'branching: while let Some(&cur) = dfs.last() {
        if let Ok(mut lk) = monitor.lock() {
            if lk.maze.exit() || lk.win.is_some() {
                return;
            }
            if (lk.maze[cur.row as usize][cur.col as usize] & solve::FINISH_BIT) != 0 {
                lk.maze[cur.row as usize][cur.col as usize] |= seen | guide.paint;
                solve::flush_cursor_path_coordinate(&lk.maze, cur);
                lk.win.get_or_insert(guide.index);
                return;
            }
            lk.maze[cur.row as usize][cur.col as usize] |= seen | guide.paint;
            solve::flush_cursor_path_coordinate(&lk.maze, cur);
        } else {
            print::maze_panic!("Solve thread panic!");
        }

        thread::sleep(time::Duration::from_micros(guide.speed));

        // Bias threads towards their original dispatch direction with do-while loop.
        rng_arr.shuffle(&mut rng);
        for &i in &rng_arr {
            let p: &maze::Point = &maze::CARDINAL_DIRECTIONS[i];
            let next = maze::Point {
                row: cur.row + p.row,
                col: cur.col + p.col,
            };

            if match monitor.lock() {
                Ok(lk) => {
                    (lk.maze[next.row as usize][next.col as usize] & seen) == 0
                        && (lk.maze[next.row as usize][next.col as usize] & maze::PATH_BIT) != 0
                }
                Err(p) => print::maze_panic!("Solve thread panic: {}", p),
            } {
                dfs.push(next);
                continue 'branching;
            }
        }

        match monitor.lock() {
            Ok(mut lk) => {
                lk.maze[cur.row as usize][cur.col as usize] &= !guide.paint;
                solve::flush_cursor_path_coordinate(&lk.maze, cur);
            }
            Err(p) => print::maze_panic!("Solve thread panic!: {}", p),
        }
        thread::sleep(time::Duration::from_micros(guide.speed));
        dfs.pop();
    }
}

fn gatherer(monitor: solve::SolverMonitor, guide: solve::ThreadGuide) {
    let seen: solve::ThreadCache = guide.paint << solve::THREAD_TAG_OFFSET;
    let mut dfs: Vec<maze::Point> = Vec::with_capacity(solve::INITIAL_PATH_LEN);
    dfs.push(guide.start);
    let mut rng = thread_rng();
    let mut rng_arr: Vec<usize> = (0..solve::NUM_DIRECTIONS).collect();
    'branching: while let Some(&cur) = dfs.last() {
        if let Ok(mut lk) = monitor.lock() {
            if (lk.maze[cur.row as usize][cur.col as usize] & solve::FINISH_BIT) != 0
                && (lk.maze[cur.row as usize][cur.col as usize] & solve::CACHE_MASK) == 0
            {
                lk.maze[cur.row as usize][cur.col as usize] |= seen;
                for p in dfs {
                    if (lk.maze[p.row as usize][p.col as usize] & solve::FINISH_BIT) == 0 {
                        lk.maze[p.row as usize][p.col as usize] |= guide.paint;
                    }
                }
                return;
            }
            lk.maze[cur.row as usize][cur.col as usize] |= seen;
        } else {
            print::maze_panic!("Solve thread panic!");
        }

        // Bias threads towards their original dispatch direction with do-while loop.
        rng_arr.shuffle(&mut rng);
        for &i in &rng_arr {
            let p: &maze::Point = &maze::CARDINAL_DIRECTIONS[i];
            let next = maze::Point {
                row: cur.row + p.row,
                col: cur.col + p.col,
            };

            if match monitor.lock() {
                Ok(lk) => {
                    (lk.maze[next.row as usize][next.col as usize] & seen) == 0
                        && (lk.maze[next.row as usize][next.col as usize] & maze::PATH_BIT) != 0
                }
                Err(p) => print::maze_panic!("Solve thread panic: {}", p),
            } {
                dfs.push(next);
                continue 'branching;
            }
        }
        dfs.pop();
    }
}

fn animated_gatherer(monitor: solve::SolverMonitor, guide: solve::ThreadGuide) {
    let seen: solve::ThreadCache = guide.paint << solve::THREAD_TAG_OFFSET;
    let mut dfs: Vec<maze::Point> = Vec::with_capacity(solve::INITIAL_PATH_LEN);
    dfs.push(guide.start);
    let mut rng = thread_rng();
    let mut rng_arr: Vec<usize> = (0..solve::NUM_DIRECTIONS).collect();
    'branching: while let Some(&cur) = dfs.last() {
        if let Ok(mut lk) = monitor.lock() {
            if lk.maze.exit() {
                return;
            }
            if (lk.maze[cur.row as usize][cur.col as usize] & solve::FINISH_BIT) != 0
                && (lk.maze[cur.row as usize][cur.col as usize] & solve::CACHE_MASK) == 0
            {
                lk.maze[cur.row as usize][cur.col as usize] |= seen | guide.paint;
                solve::flush_cursor_path_coordinate(&lk.maze, cur);
                return;
            }
            lk.maze[cur.row as usize][cur.col as usize] |= seen | guide.paint;
            solve::flush_cursor_path_coordinate(&lk.maze, cur);
        } else {
            print::maze_panic!("Solve thread panic!");
        }

        thread::sleep(time::Duration::from_micros(guide.speed));

        // Bias threads towards their original dispatch direction with do-while loop.
        rng_arr.shuffle(&mut rng);
        for &i in &rng_arr {
            let p: &maze::Point = &maze::CARDINAL_DIRECTIONS[i];
            let next = maze::Point {
                row: cur.row + p.row,
                col: cur.col + p.col,
            };

            if match monitor.lock() {
                Ok(lk) => {
                    (lk.maze[next.row as usize][next.col as usize] & seen) == 0
                        && (lk.maze[next.row as usize][next.col as usize] & maze::PATH_BIT) != 0
                }
                Err(p) => print::maze_panic!("Solve thread panic: {}", p),
            } {
                dfs.push(next);
                continue 'branching;
            }
        }

        match monitor.lock() {
            Ok(mut lk) => {
                lk.maze[cur.row as usize][cur.col as usize] &= !guide.paint;
                solve::flush_cursor_path_coordinate(&lk.maze, cur);
            }
            Err(p) => print::maze_panic!("Solve thread panic!: {}", p),
        };
        thread::sleep(time::Duration::from_micros(guide.speed));
        dfs.pop();
    }
}
