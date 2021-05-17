use alloc::{vec, vec::Vec};

use super::Bitmap;
type N = i16;

/// Segment of a vector graphics path.
///
/// This is used in the [path() function](Bitmap::path) of the [Bitmap struct](Bitmap).
/// See the documentation there for more details.
#[derive(Debug, PartialEq)]
pub enum PathSegment {
    /// Represents a relative move without drawing.
    ///
    /// The first entry is the relative x distance `dx` (so the horizontal distance),
    /// and the second entry is the relative vertical distance `dy`. This segment begins a new
    /// subpath.
    ///
    /// This is like a `m` entry in a SVG path, but there the order of `dx` and `dy` are
    /// switched.
    ///
    /// A list of path segments returned by [path()](Bitmap::path) does _not_
    /// start with this. The first path is assumed to start implicitly.
    Move(i16, i16),
    /// A horizontal draw, relative distance.
    ///
    /// This is like a `h` entry in a  SVG path.
    Horizontal(i16),
    /// A vertical draw, relative distance.
    ///
    /// This is like a `v` entry in a  SVG path.
    Vertical(i16),
    /// Close the current (sub)path. Can occur multiple times.
    ///
    /// This is like a `z` entry in a  SVG path.
    Close,
}

#[derive(Debug)]
enum MicroSteps {
    Jump(N),
    Step(N),
}

impl Bitmap<bool> {
    /// Get vector path drawing instructions for this bitmap.
    ///
    /// This function computes a sequence of relative draw, relative move, and close instructions.
    /// The resulting path shows the bitmap if filled properly (see below).
    ///
    /// The coordinate system is identical to the one of the function [pixels()](Self::pixels).
    /// The starting position is not needed in this function because only
    /// relative coordinates are returned.
    ///
    /// # Filling rule
    ///
    /// The even-odd filling rule (as known in vector graphics) must be used. It is supported
    /// by many vector graphic formats, including SVG and PDF.
    ///
    /// # Example
    ///
    /// The `examples/` directory contains a SVG and PDF code example using this
    /// helper.
    ///
    /// # Implementation
    ///
    /// The outline is modeled as a graph which is then decomposed into
    /// Eulerian circuits. The worst case runtime is quadratic in the number of
    /// "pixels". This could be improved by using a hash set data structure or
    /// something similar.
    pub fn path(&self) -> Vec<PathSegment> {
        let mut graph = bits_to_edge_graph(&self.bits, self.width() as N, self.height() as N);
        let mut elements = Vec::new();

        let mut alternatives = Vec::new();
        let mut insert = 0;
        // loop over the eulerian walks in the graph (composed of multiple in general)
        loop {
            // complete an euler walk, Hierholzer's algorithm
            'euler: loop {
                *graph.edge_mut(&graph.pos.clone()) = false;
                let start = graph.pos.start_node();
                elements.insert(insert, MicroSteps::Step(graph.pos.end_node()));
                insert += 1;

                // walk until we find start node again
                loop {
                    let pos = graph.pos.clone();
                    if graph.step_and_had_alternatives() {
                        alternatives.push((insert, pos));
                    }
                    let end = graph.pos.end_node();
                    elements.insert(insert, MicroSteps::Step(end));
                    if end == start {
                        break;
                    }
                    insert += 1;
                }
                // are there remaining edges for this euler walk?
                for (idx, pos) in alternatives.drain(0..) {
                    if let Some(new_pos) = graph.can_step(&pos) {
                        graph.pos = new_pos;
                        insert = idx;
                        continue 'euler;
                    }
                }
                break;
            }

            // are there edges remaining in the graph, then start a new euler tour
            if let Some(pos) = graph.has_edge() {
                elements.push(MicroSteps::Jump(pos.start_node()));
                graph.pos = pos;
                insert = elements.len();
                continue;
            }
            break;
        }
        compress_path(elements, self.width() as N)
    }
}

fn compress_path(mut micro_steps: Vec<MicroSteps>, width: N) -> Vec<PathSegment> {
    let mut steps = Vec::new();
    let mut pos = (0, 0);
    micro_steps.reverse();

    let ij = |n: N| (n / (width + 1), n % (width + 1));

    // step, "work in progress"
    let mut step_wip = None;
    while let Some(micro_step) = micro_steps.pop() {
        match micro_step {
            MicroSteps::Step(n) => {
                let (i, j) = ij(n);
                match step_wip {
                    // check if we can combine step with step_wip
                    Some(PathSegment::Horizontal(m)) if i == pos.0 => {
                        step_wip = Some(PathSegment::Horizontal(m + (j - pos.1)));
                    }
                    Some(PathSegment::Vertical(m)) if j == pos.1 => {
                        step_wip = Some(PathSegment::Vertical(m + (i - pos.0)));
                    }
                    // start new step_wip
                    mut other => {
                        if let Some(other) = other.take() {
                            steps.push(other);
                        }
                        if i == pos.0 {
                            step_wip = Some(PathSegment::Horizontal(j - pos.1));
                        } else {
                            step_wip = Some(PathSegment::Vertical(i - pos.0));
                        }
                    }
                }
                pos = (i, j);
            }
            MicroSteps::Jump(n) => {
                // drop content of step_wip, just add close
                step_wip = None;
                steps.push(PathSegment::Close);
                let (i, j) = ij(n);
                steps.push(PathSegment::Move(j - pos.1, i - pos.0));
                pos = (i, j);
            }
        }
    }
    steps.push(PathSegment::Close);
    steps
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    Up,
    Down,
    Right,
    Left,
}

impl Direction {
    fn flip(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Right => Self::Left,
            Self::Left => Self::Right,
        }
    }
}

/// Oriented position in the graph (on  an edge)
#[derive(Debug, Clone, PartialEq)]
struct Position {
    i: N,
    j: N,
    width: N,
    height: N,
    dir: Direction,
}

impl Position {
    /// Get node id of node the position points to.
    fn end_node(&self) -> N {
        let w = self.width + 1;
        let i = self.i;
        let j = self.j;
        match self.dir {
            Direction::Up | Direction::Left => i * w + j,
            Direction::Down => (i + 1) * w + j,
            Direction::Right => i * w + j + 1,
        }
    }

    /// Get node if of node the position comes from.
    fn start_node(&self) -> N {
        self.flip().end_node()
    }

    fn flip(&self) -> Position {
        Position {
            dir: self.dir.flip(),
            ..self.clone()
        }
    }

    fn straight(&self) -> Option<Self> {
        let i = self.i;
        let j = self.j;
        let (i2, j2) = match self.dir {
            Direction::Up => (i - 1, j),
            Direction::Down => (i + 1, j),
            Direction::Right => (i, j + 1),
            Direction::Left => (i, j - 1),
        };
        self.check(i2, j2, self.dir)
    }

    fn left(&self) -> Option<Self> {
        let i = self.i;
        let j = self.j;
        let (i2, j2, dir) = match self.dir {
            Direction::Up => (i, j - 1, Direction::Left),
            Direction::Down => (i + 1, j, Direction::Right),
            Direction::Right => (i - 1, j + 1, Direction::Up),
            Direction::Left => (i, j, Direction::Down),
        };
        self.check(i2, j2, dir)
    }

    fn right(&self) -> Option<Self> {
        let i = self.i;
        let j = self.j;
        let (i2, j2, dir) = match self.dir {
            Direction::Up => (i, j, Direction::Right),
            Direction::Down => (i + 1, j - 1, Direction::Left),
            Direction::Right => (i, j + 1, Direction::Down),
            Direction::Left => (i - 1, j, Direction::Up),
        };
        self.check(i2, j2, dir)
    }

    fn check(&self, i: N, j: N, dir: Direction) -> Option<Self> {
        if 0 <= i && i <= self.height && 0 <= j && j <= self.width {
            Some(Self {
                i,
                j,
                width: self.width,
                height: self.height,
                dir,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
struct Graph {
    /// Two marker for each grid cell, (edge left, edge top)
    edges: Vec<(bool, bool)>,
    pos: Position,
}

impl Graph {
    fn left_mut(&mut self, i: N, j: N) -> &mut bool {
        let i = i as usize;
        let j = j as usize;
        &mut self.edges[i * (self.pos.width as usize + 1) + j].0
    }

    fn top_mut(&mut self, i: N, j: N) -> &mut bool {
        let i = i as usize;
        let j = j as usize;
        &mut self.edges[i * (self.pos.width as usize + 1) + j].1
    }

    fn right_mut(&mut self, i: N, j: N) -> &mut bool {
        self.left_mut(i, j + 1)
    }

    fn bottom_mut(&mut self, i: N, j: N) -> &mut bool {
        self.top_mut(i + 1, j)
    }

    fn left(&self, i: N, j: N) -> &bool {
        let i = i as usize;
        let j = j as usize;
        &self.edges[i * (self.pos.width as usize + 1) + j].0
    }

    fn top(&self, i: N, j: N) -> &bool {
        let i = i as usize;
        let j = j as usize;
        &self.edges[i * (self.pos.width as usize + 1) + j].1
    }

    fn can_step(&self, pos: &Position) -> Option<Position> {
        None.or_else(|| pos.straight().filter(|p| self.edge(p)))
            .or_else(|| pos.left().filter(|p| self.edge(p)))
            .or_else(|| pos.right().filter(|p| self.edge(p)))
    }

    fn step_and_had_alternatives(&mut self) -> bool {
        let mut found = None;
        let mut alternatives = false;

        fn free(s: &mut Graph, p: Option<Position>) -> Option<(Position, &mut bool)> {
            p.map(move |p: Position| {
                let edge = s.edge_mut(&p);
                (p, edge)
            })
            .filter(|e| *e.1)
        }

        if let Some((pos, edge)) = free(self, self.pos.straight()) {
            *edge = false;
            found = Some(pos);
        }

        if let Some((pos, edge)) = free(self, self.pos.left()) {
            if found.is_none() {
                *edge = false;
                found = Some(pos);
            } else {
                alternatives = true;
            }
        }

        if !alternatives {
            if let Some((pos, edge)) = free(self, self.pos.right()) {
                if found.is_none() {
                    *edge = false;
                    found = Some(pos);
                } else {
                    alternatives = true;
                }
            }
        }

        self.pos = found.unwrap();
        alternatives
    }

    fn edge(&self, pos: &Position) -> bool {
        match pos.dir {
            Direction::Left | Direction::Right => *self.top(pos.i, pos.j),
            Direction::Up | Direction::Down => *self.left(pos.i, pos.j),
        }
    }

    fn edge_mut<'a>(&'a mut self, pos: &Position) -> &'a mut bool {
        match pos.dir {
            Direction::Left | Direction::Right => self.top_mut(pos.i, pos.j),
            Direction::Up | Direction::Down => self.left_mut(pos.i, pos.j),
        }
    }

    fn has_edge(&self) -> Option<Position> {
        for i in 0..=self.pos.height {
            for j in 0..=self.pos.width {
                if *self.top(i, j) {
                    return Some(Position {
                        i,
                        j,
                        width: self.pos.width,
                        height: self.pos.height,
                        dir: Direction::Right,
                    });
                }
                if *self.left(i, j) {
                    return Some(Position {
                        i,
                        j,
                        width: self.pos.width,
                        height: self.pos.height,
                        dir: Direction::Up,
                    });
                }
            }
        }
        None
    }
}

fn bits_to_edge_graph(bits: &[bool], width: N, height: N) -> Graph {
    let mut graph = Graph {
        edges: vec![(false, false); (width as usize + 1) * (height as usize + 1)],
        pos: Position {
            i: 0,
            j: 0,
            width,
            height,
            dir: Direction::Right,
        },
    };
    for i in 0..height {
        for j in 0..width {
            let idx = i * width + j;
            if !bits[idx as usize] {
                continue;
            }
            if j == 0 || !bits[(idx - 1) as usize] {
                *graph.left_mut(i, j) = true;
            }
            if j == width - 1 || !bits[(idx + 1) as usize] {
                *graph.right_mut(i, j) = true;
            }
            if i == 0 || !bits[(idx - width) as usize] {
                *graph.top_mut(i, j) = true;
            }
            if i == height - 1 || !bits[(idx + width) as usize] {
                *graph.bottom_mut(i, j) = true;
            }
        }
    }
    graph
}

#[test]
fn mini_2x2_one_euler() {
    let bm = Bitmap {
        bits: vec![true, false, true, true],
        width: 2,
    };
    assert_eq!(
        bits_to_edge_graph(&bm.bits, 2, 2),
        Graph {
            edges: vec![
                (true, true),
                (true, false),
                (false, false),
                (true, false),
                (false, true),
                (true, false),
                (false, true),
                (false, true),
                (false, false),
            ],
            pos: Position {
                i: 0,
                j: 0,
                width: 2,
                height: 2,
                dir: Direction::Right,
            },
        }
    );
    assert_eq!(
        bm.path(),
        vec![
            PathSegment::Horizontal(1),
            PathSegment::Vertical(1),
            PathSegment::Horizontal(1),
            PathSegment::Vertical(1),
            PathSegment::Horizontal(-2),
            PathSegment::Close,
        ],
    );
}

#[test]
fn mini_2x3_one_euler() {
    let bm = Bitmap {
        bits: vec![true, false, true, true, true, false],
        width: 3,
    };
    assert_eq!(
        bm.path(),
        vec![
            PathSegment::Horizontal(1),
            PathSegment::Vertical(1),
            PathSegment::Horizontal(2),
            PathSegment::Vertical(-1),
            PathSegment::Horizontal(-1),
            PathSegment::Vertical(2),
            PathSegment::Horizontal(-2),
            PathSegment::Close,
        ],
    );
}

#[test]
fn mini_3x2_two_euler() {
    let bm = Bitmap {
        bits: vec![true, true, false, false, false, true],
        width: 2,
    };
    assert_eq!(
        bm.path(),
        vec![
            PathSegment::Horizontal(2),
            PathSegment::Vertical(1),
            PathSegment::Horizontal(-2),
            PathSegment::Close,
            PathSegment::Move(1, 2),
            PathSegment::Horizontal(1),
            PathSegment::Vertical(1),
            PathSegment::Horizontal(-1),
            PathSegment::Close,
        ],
    );
}
