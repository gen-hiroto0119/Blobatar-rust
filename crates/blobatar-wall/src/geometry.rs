use serde::{Deserialize, Serialize};

pub const CHUNK: i32 = 32;
pub const CAPACITY: i32 = CHUNK * CHUNK;
pub const REACH: i32 = 32;
pub const REGION: i32 = 8;
pub const FIRST: Cell = Cell { x: 0, y: 0 };

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Chunk {
    pub cx: i32,
    pub cy: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Region {
    pub rx: i32,
    pub ry: i32,
}

pub type Occupied = dyn Fn(i32, i32) -> bool;

pub fn chunk_of(x: i32, y: i32) -> Chunk {
    Chunk {
        cx: x.div_euclid(CHUNK),
        cy: y.div_euclid(CHUNK),
    }
}

pub fn region_of(chunk: Chunk) -> Region {
    Region {
        rx: chunk.cx.div_euclid(REGION),
        ry: chunk.cy.div_euclid(REGION),
    }
}

pub fn cell_index(x: i32, y: i32) -> i32 {
    y.rem_euclid(CHUNK) * CHUNK + x.rem_euclid(CHUNK)
}

pub fn cell_at(chunk: Chunk, index: i32) -> Cell {
    Cell {
        x: chunk.cx * CHUNK + index.rem_euclid(CHUNK),
        y: chunk.cy * CHUNK + index.div_euclid(CHUNK),
    }
}

pub fn cell_key(x: i32, y: i32) -> String {
    format!("{x},{y}")
}

pub fn chunk_key(chunk: Chunk) -> String {
    format!("{}_{}", chunk.cx, chunk.cy)
}

pub fn parse_chunk_key(key: &str) -> Option<Chunk> {
    let (cx, cy) = key.split_once('_')?;
    if !valid_component(cx) || !valid_component(cy) {
        return None;
    }
    Some(Chunk {
        cx: cx.parse().ok()?,
        cy: cy.parse().ok()?,
    })
}

fn valid_component(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.len() <= 7 && digits.bytes().all(|byte| byte.is_ascii_digit())
}

pub fn chunks_covering(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Chunk> {
    let from = chunk_of(x0.min(x1), y0.min(y1));
    let to = chunk_of(x0.max(x1), y0.max(y1));
    let mut chunks = Vec::new();
    for cy in from.cy..=to.cy {
        for cx in from.cx..=to.cx {
            chunks.push(Chunk { cx, cy });
        }
    }
    chunks
}

fn distance_squared(ax: i32, ay: i32, bx: i32, by: i32) -> i64 {
    let dx = i64::from(ax) - i64::from(bx);
    let dy = i64::from(ay) - i64::from(by);
    dx * dx + dy * dy
}

fn ring(center: Cell, radius: i32) -> Vec<Cell> {
    if radius == 0 {
        return vec![center];
    }
    let mut cells = Vec::with_capacity((radius * 8) as usize);
    for d in -radius..=radius {
        cells.push(Cell {
            x: center.x + d,
            y: center.y - radius,
        });
        cells.push(Cell {
            x: center.x + d,
            y: center.y + radius,
        });
    }
    for d in (-radius + 1)..=(radius - 1) {
        cells.push(Cell {
            x: center.x - radius,
            y: center.y + d,
        });
        cells.push(Cell {
            x: center.x + radius,
            y: center.y + d,
        });
    }
    cells
}

pub fn within_reach(x: i32, y: i32, occupied: &Occupied) -> bool {
    for radius in 0..=REACH {
        if ring(Cell { x, y }, radius).into_iter().any(|cell| {
            distance_squared(x, y, cell.x, cell.y) <= i64::from(REACH * REACH)
                && occupied(cell.x, cell.y)
        }) {
            return true;
        }
    }
    false
}

pub fn is_placeable(x: i32, y: i32, occupied: &Occupied, populated: bool) -> bool {
    if occupied(x, y) {
        return false;
    }
    if !populated {
        return Cell { x, y } == FIRST;
    }
    within_reach(x, y, occupied)
}

pub fn nearest_placeable(
    target: Cell,
    occupied: &Occupied,
    populated: bool,
    max_search: i32,
) -> Option<Cell> {
    if !populated {
        return (!occupied(FIRST.x, FIRST.y)).then_some(FIRST);
    }
    if is_placeable(target.x, target.y, occupied, true) {
        return Some(target);
    }

    let mut anchor = None;
    'search: for radius in 0..=max_search {
        for cell in ring(target, radius) {
            if occupied(cell.x, cell.y) {
                anchor = Some(cell);
                break 'search;
            }
        }
    }
    let anchor = anchor?;
    let dx = target.x - anchor.x;
    let dy = target.y - anchor.y;
    let dx_squared = i64::from(dx) * i64::from(dx);
    let dy_squared = i64::from(dy) * i64::from(dy);
    let distance = ((dx_squared + dy_squared) as f64).sqrt();
    let from = if distance > f64::from(REACH) {
        let scale = f64::from(REACH) / distance;
        Cell {
            x: anchor.x + js_round(f64::from(dx) * scale),
            y: anchor.y + js_round(f64::from(dy) * scale),
        }
    } else {
        target
    };

    for radius in 0..=REACH {
        for cell in ring(from, radius) {
            if !occupied(cell.x, cell.y)
                && distance_squared(cell.x, cell.y, anchor.x, anchor.y) <= i64::from(REACH * REACH)
            {
                return Some(cell);
            }
        }
    }
    None
}

fn js_round(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}
