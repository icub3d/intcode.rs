const INPUT: &str = include_str!("../../inputs/day19");

use std::collections::HashSet;

use intcode::ipc::Channel;
use intcode::process::Process;
use tokio::sync::mpsc;

async fn check_point(x: isize, y: isize) -> bool {
    let (_, mut input_tx, input_rx) = Channel::new(true);
    let (_, output_tx, mut output_rx) = Channel::new(true);
    input_tx.send(x).await.unwrap();
    input_tx.send(y).await.unwrap();
    tokio::spawn(async move {
        let mut process = Process::new(INPUT, input_rx, output_tx);
        process.run().await.unwrap();
    });
    if let Some(output) = output_rx.recv().await {
        output == 1
    } else {
        false
    }
}

#[tokio::main]
async fn main() {
    let now = std::time::Instant::now();
    let (tx, mut rx) = mpsc::channel(32);
    for x in 0..50 {
        for y in 0..50 {
            let tx = tx.clone();
            tokio::spawn(async move {
                if check_point(x, y).await {
                    tx.send(Point::new(x, y)).await.unwrap();
                }
            });
        }
    }
    drop(tx);
    let mut grid = HashSet::new();
    while let Some(point) = rx.recv().await {
        grid.insert(point);
    }
    println!("p1: {} ({:?})", grid.len(), now.elapsed());
    for y in 0..50 {
        for x in 0..50 {
            print!(
                "{}",
                if grid.contains(&Point::new(x, y)) {
                    '#'
                } else {
                    '.'
                }
            );
        }
        println!();
    }

    let mut x = 50;
    loop {
        let min_y = grid
            .iter()
            .filter(|p| p.x == x - 1)
            .map(|p| p.y)
            .min()
            .unwrap();
        let max_y = grid
            .iter()
            .filter(|p| p.x == x - 1)
            .map(|p| p.y)
            .max()
            .unwrap();
        let mut start = 0;
        let mut end = 0;
        let mut y = min_y;
        while y < max_y {
            if check_point(x, y).await {
                if start == 0 {
                    start = y;
                }
                grid.insert(Point::new(x, y));
            }
            y += 1;
        }
        while check_point(x, y).await {
            grid.insert(Point::new(x, y));
            end = y;
            y += 1;
        }

        if end - start >= 100 {
            let y = start + 99;
            let x = x - 99;
            if grid.contains(&Point::new(x, y)) {
                println!("p2: {} ({:?})", x * 10000 + start, now.elapsed());
                break;
            }
        }

        x += 1;
    }
}

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
