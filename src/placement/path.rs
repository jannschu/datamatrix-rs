use alloc::{vec, vec::Vec};
use core::cell::RefCell;

#[cfg(test)]
use pretty_assertions::assert_eq;

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
    /// This is like a `h` entry in a SVG path.
    Horizontal(i16),
    /// A vertical draw, relative distance.
    ///
    /// This is like a `v` entry in a SVG path.
    Vertical(i16),
    /// Close the current (sub)path. Can occur multiple times.
    ///
    /// This is like a `z` entry in a SVG path.
    Close,
}

#[derive(Debug)]
enum MicroStep {
    Jump((N, N)),
    Step((N, N)),
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
    /// The `examples/` directory contains a SVG, EPS and PDF code example using this
    /// helper.
    ///
    /// # Implementation
    ///
    /// The outline is modeled as a graph which is then decomposed into
    /// Eulerian circuits.
    pub fn path(&self) -> Vec<PathSegment> {
        let mut graph = bits_to_edge_graph(&self.bits, self.width(), self.height());
        let mut pos = if let Some(pos) = graph.edge_left() {
            pos
        } else {
            return vec![];
        };
        let mut elements = Vec::new();

        let mut alternatives = Vec::new();
        let mut insert = 0;
        // loop over the eulerian walks in the graph (composed of multiple in general)
        loop {
            // complete an Eulerian tour, Hierholzer's algorithm
            'euler: loop {
                let mut local_loop = Vec::new();
                let insert_pos = insert;

                graph.remove_edge(&pos);
                let start = pos.start_node();
                local_loop.push(MicroStep::Step(pos.end_node()));
                insert += 1;

                // walk until we find start node again
                loop {
                    let (new_pos, had_alternatives) = graph.follow(&pos);
                    if had_alternatives {
                        alternatives.push((insert, pos));
                    }
                    pos = new_pos.expect("must exist because `pos` was valid");
                    graph.remove_edge(&pos);
                    let end = pos.end_node();
                    local_loop.push(MicroStep::Step(end));
                    if end == start {
                        break;
                    }
                    insert += 1;
                }
                elements.splice(insert_pos..insert_pos, local_loop.drain(..));

                // are there remaining edges for this euler walk?
                for (idx, pos_alt) in alternatives.drain(..) {
                    if let Some(new_pos) = graph.can_step(&pos_alt) {
                        pos = new_pos;
                        insert = idx;
                        continue 'euler;
                    }
                }
                break;
            }

            // are there edges remaining in the graph, then start a new Eulerian tour
            if let Some(new_pos) = graph.edge_left() {
                elements.push(MicroStep::Jump(new_pos.start_node()));
                pos = new_pos;
                insert = elements.len();
                continue;
            }
            break;
        }
        compress_path(elements.into_iter())
    }
}

fn compress_path(micro_steps: impl Iterator<Item = MicroStep>) -> Vec<PathSegment> {
    let mut steps = Vec::new();
    let mut pos = (0, 0);

    // step, "work in progress"
    let mut step_wip = None;
    for micro_step in micro_steps {
        match micro_step {
            MicroStep::Step((i, j)) => {
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
            MicroStep::Jump((i, j)) => {
                // drop content of step_wip, just add close
                step_wip = None;
                steps.push(PathSegment::Close);
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

/// Oriented position in the graph on an edge.
#[derive(Debug, Clone, PartialEq)]
struct Position {
    i: N,
    j: N,
    dir: Direction,
}

impl Position {
    /// Get node coordinate of the node the position points to.
    fn end_node(&self) -> (N, N) {
        let i = self.i;
        let j = self.j;
        match self.dir {
            Direction::Up | Direction::Left => (i, j),
            Direction::Down => (i + 1, j),
            Direction::Right => (i, j + 1),
        }
    }

    /// Get node coordinate of the node the position comes from.
    fn start_node(&self) -> (N, N) {
        self.flip().end_node()
    }

    fn flip(&self) -> Position {
        Position {
            dir: self.dir.flip(),
            ..self.clone()
        }
    }

    fn straight(&self) -> Self {
        let (i, j) = match self.dir {
            Direction::Up => (self.i - 1, self.j),
            Direction::Down => (self.i + 1, self.j),
            Direction::Right => (self.i, self.j + 1),
            Direction::Left => (self.i, self.j - 1),
        };
        Self {
            i,
            j,
            dir: self.dir,
        }
    }

    fn left(&self) -> Self {
        let (i, j, dir) = match self.dir {
            Direction::Up => (self.i, self.j - 1, Direction::Left),
            Direction::Down => (self.i + 1, self.j, Direction::Right),
            Direction::Right => (self.i - 1, self.j + 1, Direction::Up),
            Direction::Left => (self.i, self.j, Direction::Down),
        };
        Self { i, j, dir }
    }

    fn right(&self) -> Self {
        let (i, j, dir) = match self.dir {
            Direction::Up => (self.i, self.j, Direction::Right),
            Direction::Down => (self.i + 1, self.j - 1, Direction::Left),
            Direction::Right => (self.i, self.j + 1, Direction::Down),
            Direction::Left => (self.i - 1, self.j, Direction::Up),
        };
        Self { i, j, dir }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct Edge {
    left: bool,
    top: bool,
}

#[derive(Debug, PartialEq)]
struct Graph {
    edges: Vec<Edge>,
    width: usize,
    height: usize,
    edge_hint: RefCell<usize>,
}

impl Graph {
    fn left(&self, i: N, j: N) -> bool {
        self.has_cell(i, j) && {
            let i = i as usize;
            let j = j as usize;
            self.edges[i * (self.width + 1) + j].left
        }
    }

    fn top(&self, i: N, j: N) -> bool {
        self.has_cell(i, j) && {
            let i = i as usize;
            let j = j as usize;
            self.edges[i * (self.width + 1) + j].top
        }
    }

    fn has_cell(&self, i: N, j: N) -> bool {
        (0..=self.height as N).contains(&i) && (0..=self.width as N).contains(&j)
    }

    fn can_step(&self, pos: &Position) -> Option<Position> {
        None.or_else(|| Some(pos.straight()).filter(|p| self.has_edge(p)))
            .or_else(|| Some(pos.left()).filter(|p| self.has_edge(p)))
            .or_else(|| Some(pos.right()).filter(|p| self.has_edge(p)))
    }

    fn follow(&self, pos: &Position) -> (Option<Position>, bool) {
        let mut found = None;
        let mut alternatives = false;

        macro_rules! try_step {
            ($pos:expr) => {
                if let Some(pos) = Some($pos).filter(|p| self.has_edge(p)) {
                    if found.is_none() {
                        found = Some(pos);
                    } else {
                        alternatives = true;
                    }
                }
            };
        }

        try_step!(pos.straight());
        try_step!(pos.left());
        try_step!(pos.right());

        (found, alternatives)
    }

    fn has_edge(&self, pos: &Position) -> bool {
        match pos.dir {
            Direction::Left | Direction::Right => self.top(pos.i, pos.j),
            Direction::Up | Direction::Down => self.left(pos.i, pos.j),
        }
    }

    fn remove_top(&mut self, i: N, j: N) -> bool {
        self.has_cell(i, j) && {
            let idx = i as usize * (self.width + 1) + j as usize;
            core::mem::replace(&mut self.edges[idx].top, false)
        }
    }

    fn remove_left(&mut self, i: N, j: N) -> bool {
        self.has_cell(i, j) && {
            let idx = i as usize * (self.width + 1) + j as usize;
            core::mem::replace(&mut self.edges[idx].left, false)
        }
    }

    fn remove_edge(&mut self, pos: &Position) -> bool {
        match pos.dir {
            Direction::Left | Direction::Right => self.remove_top(pos.i, pos.j),
            Direction::Up | Direction::Down => self.remove_left(pos.i, pos.j),
        }
    }

    fn edge_left(&self) -> Option<Position> {
        let hint = *self.edge_hint.borrow();
        for (idx, edge) in self.edges[hint..].iter().enumerate() {
            if edge.left || edge.top {
                let idx = idx + hint;
                let i = (idx / (self.width + 1)) as N;
                let j = (idx % (self.width + 1)) as N;
                self.edge_hint.replace_with(|_| idx);
                return Some(Position {
                    i,
                    j,
                    dir: if edge.top {
                        Direction::Right
                    } else {
                        Direction::Up
                    },
                });
            }
        }
        self.edge_hint.replace_with(|_| self.edges.len());
        None
    }
}

fn bits_to_edge_graph(bits: &[bool], width: usize, height: usize) -> Graph {
    let mut graph = Graph {
        edges: vec![
            Edge {
                left: false,
                top: false
            };
            (width + 1) * (height + 1)
        ],
        edge_hint: RefCell::new(0),
        width,
        height,
    };

    let _: N = (graph.width + 1).try_into().expect("width overflow");
    let _: N = (graph.height + 1).try_into().expect("height overflow");

    let mut edge_hint = None;

    for i in 0..height {
        for j in 0..width {
            let idx = i * width + j;
            if !bits[idx] {
                continue;
            }
            let cell = i * (width + 1) + j;
            edge_hint.get_or_insert(cell);
            if j == 0 || !bits[idx - 1] {
                // left
                graph.edges[cell].left = true;
            }
            if i == 0 || !bits[idx - width] {
                // top
                graph.edges[cell].top = true;
            }
            if j == width - 1 || !bits[idx + 1] {
                // right
                graph.edges[cell + 1].left = true;
            }
            if i == height - 1 || !bits[idx + width] {
                // bottom
                graph.edges[cell + (width + 1)].top = true;
            }
        }
    }
    *graph.edge_hint.get_mut() = edge_hint.unwrap_or_else(|| graph.edges.len());
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
                Edge {
                    left: true,
                    top: true
                },
                Edge {
                    left: true,
                    top: false
                },
                Edge {
                    left: false,
                    top: false
                },
                Edge {
                    left: true,
                    top: false
                },
                Edge {
                    left: false,
                    top: true
                },
                Edge {
                    left: true,
                    top: false
                },
                Edge {
                    left: false,
                    top: true
                },
                Edge {
                    left: false,
                    top: true
                },
                Edge {
                    left: false,
                    top: false
                },
            ],
            edge_hint: RefCell::new(0),
            width: 2,
            height: 2,
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

#[test]
fn empty() {
    let bm = Bitmap::new(vec![false; 6], 2);
    assert_eq!(bm.path(), vec![]);
}

#[test]
fn edge_hint() {
    let bm = Bitmap {
        bits: vec![false, false, true, true, true, true],
        width: 3,
    };
    let graph = bits_to_edge_graph(&bm.bits, bm.width(), bm.height());
    assert_eq!(*graph.edge_hint.borrow(), 2);
}
