use std::collections::HashMap;
use std::ops::{Add, AddAssign};

use intcode::ipc::Channel;
use intcode::process::Process;

// Create a point struct to represent where the robot is or has been.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: isize,
    y: isize,
}

impl Point {
    fn new(x: isize, y: isize) -> Self {
        Self { x, y }
    }
}

// Implement the Add and AddAssign traits for the Point struct. This simplifies some of the code
// below.
impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

// Create an enum to represent the direction the robot is facing.
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

// Here we can implement some helper methods for when the robot turns.
impl Direction {
    fn turn_left(&self) -> Self {
        match self {
            Direction::Up => Direction::Left,
            Direction::Down => Direction::Right,
            Direction::Left => Direction::Down,
            Direction::Right => Direction::Up,
        }
    }

    fn turn_right(&self) -> Self {
        match self {
            Direction::Up => Direction::Right,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
            Direction::Right => Direction::Down,
        }
    }
}

// We can turn the Direction into a point to simplify moving the robot.
impl From<&Direction> for Point {
    fn from(direction: &Direction) -> Self {
        match direction {
            Direction::Up => Point::new(0, -1),
            Direction::Down => Point::new(0, 1),
            Direction::Left => Point::new(-1, 0),
            Direction::Right => Point::new(1, 0),
        }
    }
}

// This is used for both parts 1 and two. We send the robot along it's way and collect the output
// of it's work.
async fn run_robot(start: isize) -> HashMap<Point, isize> {
    let input = include_str!("inputs/day11");

    let (_, mut tx, rx) = Channel::new(true);
    let (_, tx2, mut output) = Channel::new(true);
    tx.send(start).await.unwrap();
    tokio::spawn(async move {
        let mut process = Process::new(input, rx, tx2);
        process.run().await.unwrap();
    });

    // Track our state.
    let mut grid = HashMap::new();
    let mut position = Point::new(0, 0);
    let mut direction = Direction::Up;

    // We alternate between painting and moving so we can track this with a bool.
    let mut paint = true;
    while let Some(value) = output.recv().await {
        // If we are painting, update out grid. Otherwise, turn and move the robot and then send
        // the color of the current position to the robot.
        if paint {
            grid.insert(position, value);
        } else {
            // Turn the robot.
            direction = match value {
                0 => direction.turn_left(),
                1 => direction.turn_right(),
                _ => panic!("Invalid direction"),
            };

            // Move the robot.
            position += (&direction).into();

            // Send the color of the current position to the robot.
            match tx.send(*grid.get(&position).unwrap_or(&0)).await {
                Ok(_) => {}
                Err(_) => break,
            }
        }
        paint = !paint;
    }

    grid
}

#[tokio::main]
async fn main() {
    // For part 1, start on a black panel and then run the robot.
    let grid = run_robot(0).await;
    println!("p1: {}", grid.len());

    // For part 2, start on a white panel and then run the robot.
    let grid = run_robot(1).await;
    println!("p2:");

    // Get the bounds of the grid so we can print it out.
    let min_x = grid.keys().map(|p| p.x).min().unwrap();
    let max_x = grid.keys().map(|p| p.x).max().unwrap();
    let min_y = grid.keys().map(|p| p.y).min().unwrap();
    let max_y = grid.keys().map(|p| p.y).max().unwrap();

    // Print out the grid.
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            print!(
                "{}",
                match grid.get(&Point::new(x, y)).unwrap_or(&0) {
                    0 => ' ',
                    1 => '#',
                    _ => panic!("Invalid color"),
                }
            );
        }
        println!();
    }
}
