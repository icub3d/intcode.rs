use std::{
    collections::{HashSet, VecDeque},
    ops::Add,
};

use enum_iterator::{all, Sequence};
use intcode::{
    instruction::Instruction,
    ipc::Channel,
    process::{Process, State},
};

use anyhow::{anyhow, Result};
use pathfinding::directed::dijkstra::dijkstra_all;

// The input for the Intcode program.
const INPUT: &str = include_str!("../../inputs/day15");

// The possible movements the robot can make.
#[derive(Debug, Clone, Copy, Sequence)]
enum Movement {
    North,
    South,
    East,
    West,
}

// Convert a movement command into an integer for the Intcode program.
impl From<Movement> for isize {
    fn from(m: Movement) -> isize {
        match m {
            Movement::North => 1,
            Movement::South => 2,
            Movement::West => 3,
            Movement::East => 4,
        }
    }
}

// The possible replies from the robot.
enum Reply {
    Wall,
    Moved,
    Found,
}

// Convert an integer into a reply from the robot.
impl From<isize> for Reply {
    fn from(i: isize) -> Self {
        match i {
            0 => Reply::Wall,
            1 => Reply::Moved,
            _ => Reply::Found,
        }
    }
}

// A point in 2D space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: isize,
    y: isize,
}

impl Point {
    fn new(x: isize, y: isize) -> Self {
        Point { x, y }
    }
}

// Add a movement to a point to get a new point.
impl Add<Movement> for Point {
    type Output = Self;

    fn add(self, movement: Movement) -> Self {
        match movement {
            Movement::North => Point {
                x: self.x,
                y: self.y + 1,
            },
            Movement::South => Point {
                x: self.x,
                y: self.y - 1,
            },
            Movement::East => Point {
                x: self.x + 1,
                y: self.y,
            },
            Movement::West => Point {
                x: self.x - 1,
                y: self.y,
            },
        }
    }
}

// A node in the search space.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Node {
    // The position of the node. This is helpful in part 2 when we need to do a BFS.
    position: Point,
    // The state of the Intcode program at this node.
    state: State,
    // Whether or not the oxygen system is at this node.
    oxygen: bool,
}

fn main() {
    // For part 2, we'll be starting at the oxygen position and doing a BFS, so we'll need to keep
    // track of the grid and the oxygen position.
    let mut grid = HashSet::new();
    let mut oxygen = None;

    // For part 1, we can simply use dijkstra's algorithm to find the shortest path to the oxygen
    // system.
    let start = Node {
        position: Point::new(0, 0),
        state: State::new(INPUT),
        oxygen: false,
    };

    // The neighbors function for part 1.
    let neighbors_fn = |n: &Node| {
        // Get our neighbors.
        let neighbors = neighbors(n).unwrap();
        // Add them to the grid and check if they are the oxygen system for part 2.
        for neighbor in &neighbors {
            grid.insert(neighbor.position);
            if neighbor.oxygen {
                oxygen = Some(neighbor.position);
            }
        }
        // Return the neighbors and their costs (always 1 because we are 'unweighted').
        neighbors.into_iter().map(|n| (n, 1))
    };

    // Run dijkstra's algorithm to find the shortest path to the oxygen system. Notice that we use
    // `dijkstra_all` instead of `dijkstra` because we want to find the shortest path to all nodes.
    // This will ensure that all nodes are visited for part 2. If you are only interested in the
    // shortest path for part 1, you would use `dijkstra` instead.
    let dists = dijkstra_all(&start, neighbors_fn);

    // Find the node that has the oxygen system and print solution for part 1.
    let p1 = dists
        .iter()
        .find(|(node, _)| node.oxygen)
        .map(|(_, dist)| dist.1)
        .expect("no oxygen system found");
    println!("p1: {}", p1);

    let mut frontier = VecDeque::new();
    frontier.push_back((0, oxygen.unwrap()));
    let mut seen = HashSet::new();
    let mut last = std::isize::MIN;
    while let Some((dist, point)) = frontier.pop_front() {
        if !seen.insert(point) {
            continue;
        }
        last = last.max(dist);
        let neighbors = neighbors_p2(&point, &grid);
        for neighbor in neighbors {
            frontier.push_back((dist + 1, neighbor));
        }
    }
    println!("p2: {}", last);
}

// Get the neighbors of a node.
fn neighbors(node: &Node) -> Result<Vec<Node>> {
    let mut neighbors = Vec::new();

    // For each direction, perform the I/O loop and add the neighbor to the list of neighbors if it
    // is a valid next state.
    for dir in all::<Movement>() {
        // Clone the state and perform the I/O loop. Note the use of `block_on` to run the async
        // function in a synchronous context.
        let state = node.state.clone();
        let (state, reply) = futures::executor::block_on(async move { io_loop(state, dir).await })?;

        // If the reply is a wall, we don't want to add the neighbor to the list of neighbors.
        // Otherwise, we add the neighbor to the list of neighbors.
        let position = node.position + dir;
        let oxygen = match reply {
            Reply::Wall => continue,
            Reply::Found => true,
            _ => false,
        };
        neighbors.push(Node {
            position,
            state,
            oxygen,
        });
    }

    // Return the list of neighbors.
    Ok(neighbors)
}

// Perform a single I/O loop with the given state.
async fn io_loop(state: State, movement: Movement) -> Result<(State, Reply)> {
    // Create a new channel for the input and output and send the movement command.
    let (_, mut input_tx, input_rx) = Channel::new(false);
    let (_, output_tx, mut output_rx) = Channel::new(false);
    input_tx.send(movement.into()).await?;

    // Create a new process with the given state and run it until the output is received.
    let mut process = Process::with_state(state, input_rx, output_tx);
    let breakpoint =
        |_: &State, instruction: &Instruction| matches!(instruction, Instruction::Output(_));
    process.run_until(breakpoint).await?;
    process.step().await?;

    // Get the output and return it with the state.
    let output = output_rx.recv().await.ok_or(anyhow!("no output = bad"))?;
    Ok((process.state(), output.into()))
}

// Get the neighbors of a point in the grid for part 2.
fn neighbors_p2(point: &Point, grid: &HashSet<Point>) -> Vec<Point> {
    all::<Movement>()
        .map(|dir| *point + dir)
        .filter(|p| grid.contains(p))
        .collect()
}
