use std::collections::{HashMap, HashSet};

const INPUT: &str = include_str!("inputs/day24");

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct Point {
    x: usize,
    y: usize,
}

impl Point {
    fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

#[derive(Debug)]
struct Grid {
    data: HashMap<Point, bool>,
}

impl Grid {
    fn new(s: &str) -> Self {
        let mut data = HashMap::new();
        for (y, line) in s.lines().enumerate() {
            for (x, c) in line.chars().enumerate() {
                data.insert(Point::new(x, y), c == '#');
            }
        }
        Self { data }
    }

    fn adjacent_bugs(&self, point: &Point) -> usize {
        let mut count = 0;
        if let Some(true) = self.data.get(&Point::new(point.x, point.y - 1)) {
            count += 1;
        }
        if let Some(true) = self.data.get(&Point::new(point.x, point.y + 1)) {
            count += 1;
        }
        if let Some(true) = self.data.get(&Point::new(point.x - 1, point.y)) {
            count += 1;
        }
        if let Some(true) = self.data.get(&Point::new(point.x + 1, point.y)) {
            count += 1;
        }
        count
    }

    fn biodiversity(&self) -> usize {
        let mut score = 0;
        for y in 0..5 {
            for x in 0..5 {
                if *self.data.get(&Point::new(x, y)).unwrap() {
                    score += 1 << (y * 5 + x);
                }
            }
        }
        score
    }

    fn to_string(&self) -> String {
        let mut s = String::new();
        for y in 0..5 {
            for x in 0..5 {
                s.push(if *self.data.get(&Point::new(x, y)).unwrap() {
                    '#'
                } else {
                    '.'
                });
            }
            s.push('\n');
        }
        s
    }
}

struct MultiGrid {
    data: HashMap<(i32, Point), bool>,
}

impl MultiGrid {
    fn new(s: &str) -> Self {
        let mut data = HashMap::new();
        for (y, line) in s.lines().enumerate() {
            for (x, c) in line.chars().enumerate() {
                data.insert((0, Point::new(x, y)), c == '#');
            }
        }
        Self { data }
    }

    fn adjacent_bugs(&self, level: i32, point: &Point) -> usize {
        let mut count = 0;
        if let Some(true) = self.data.get(&(level, Point::new(point.x, point.y - 1))) {
            count += 1;
        }
        if let Some(true) = self.data.get(&(level, Point::new(point.x, point.y + 1))) {
            count += 1;
        }
        if let Some(true) = self.data.get(&(level, Point::new(point.x - 1, point.y))) {
            count += 1;
        }
        if let Some(true) = self.data.get(&(level, Point::new(point.x + 1, point.y))) {
            count += 1;
        }

        if point.x == 0 {
            if let Some(true) = self.data.get(&(level - 1, Point::new(1, 2))) {
                count += 1;
            }
        }

        if point.y == 0 {
            if let Some(true) = self.data.get(&(level - 1, Point::new(2, 1))) {
                count += 1;
            }
        }

        if point.x == 4 {
            if let Some(true) = self.data.get(&(level - 1, Point::new(3, 2))) {
                count += 1;
            }
        }   

        if point.y == 4 {
            if let Some(true) = self.data.get(&(level - 1, Point::new(2, 3))) {
                count += 1;
            }
        }

        if point.x == 1 && point.y == 2 {
            for y in 0..5 {
                if let Some(true) = self.data.get(&(level + 1, Point::new(0, y))) {
                    count += 1;
                }
            }
        }

        if point.x == 3 && point.y == 2 {
            for y in 0..5 {
                if let Some(true) = self.data.get(&(level + 1, Point::new(4, y))) {
                    count += 1;
                }
            }
        }

        if point.x == 2 && point.y == 1 {
            for x in 0..5 {
                if let Some(true) = self.data.get(&(level + 1, Point::new(x, 0))) {
                    count += 1;
                }
            }
        }

        if point.x == 2 && point.y == 3 {
            for x in 0..5 {
                if let Some(true) = self.data.get(&(level + 1, Point::new(x, 4))) {
                    count += 1;
                }
            }
        }

        count
    }


    fn bugs(&mut self) -> usize {
        for _ in 0..200 {
            let mut next = HashMap::new();
            let min_level = self.data.keys().map(|(level, _)| *level).min().unwrap();
            let max_level = self.data.keys().map(|(level, _)| *level).max().unwrap();
            for level in min_level - 1..=max_level + 1 {
                for y in 0..5 {
                    for x in 0..5 {
                        if x == 2 && y == 2 {
                            continue;
                        }
                        next.insert((level, Point::new(x, y)), if let Some(true) = self.data.get(&(level, Point::new(x, y))) {
                            self.adjacent_bugs(level, &Point::new(x, y)) == 1
                        } else {
                            self.adjacent_bugs(level, &Point::new(x, y)) == 1 || self.adjacent_bugs(level, &Point::new(x, y)) == 2
                        });
                    }
                }
            }
            self.data = next;
        }
        self.data.values().filter(|&&b| b).count()
    }
}

#[tokio::main]
async fn main() {
    let mut grid = Grid::new(INPUT);
    let mut found = HashSet::new();
    loop {
        let mut next = HashMap::new();
        for (point, bug) in &grid.data {
            let adjacent = grid.adjacent_bugs(point);
            next.insert(point.clone(), if *bug {
                adjacent == 1
            } else {
                adjacent == 1 || adjacent == 2
            });
        }
        grid.data = next;
        if !found.insert(grid.to_string()) {
            break;
        }
    }

    println!("p1: {}", grid.biodiversity());

    let mut multi_grid = MultiGrid::new(INPUT);

    println!("p2: {}", multi_grid.bugs());

}
