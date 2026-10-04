mod geometry;
mod identity;
mod moderation;
mod sqlite;
mod wire;

pub use geometry::{
    CAPACITY, CHUNK, Cell, Chunk, FIRST, Occupied, REACH, REGION, Region, cell_at, cell_index,
    cell_key, chunk_key, chunk_of, chunks_covering, is_placeable, nearest_placeable,
    parse_chunk_key, region_of, within_reach,
};
pub use identity::{
    IdentityHash, TokenHash, day_of, hash_identity, hash_token, is_token, new_token, next_midnight,
    same_secret, token_from_cookie,
};
pub use moderation::{MAX_NAME, NameRefusal, check_expression, check_name, fold_name};
pub use sqlite::SQLiteStore;
pub use wire::{ChunkBody, ChunkPlacement, ChunkState, RegionIndex, encode_chunk};

use std::{error::Error, fmt};

#[derive(Debug)]
pub enum WallError {
    Database(String),
    InvalidName(NameRefusal),
    InvalidExpression,
    InvalidCoordinates,
    Cooldown { until: i64 },
    Unplaceable { nearest: Option<Cell> },
    Taken,
    NotFound,
}

impl fmt::Display for WallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(message) => write!(formatter, "wall database error: {message}"),
            Self::InvalidName(reason) => write!(formatter, "invalid wall name: {reason:?}"),
            Self::InvalidExpression => formatter.write_str("invalid wall expression"),
            Self::InvalidCoordinates => formatter.write_str("invalid wall coordinates"),
            Self::Cooldown { until } => write!(formatter, "wall cooldown until {until}"),
            Self::Unplaceable { .. } => formatter.write_str("wall cell is unplaceable"),
            Self::Taken => formatter.write_str("wall cell is occupied"),
            Self::NotFound => formatter.write_str("wall cell is empty"),
        }
    }
}

impl Error for WallError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub seed: String,
    pub expression: String,
    pub at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceInput {
    pub cell: Cell,
    pub seed: String,
    pub expression: String,
    pub now: i64,
    pub identity: IdentityHash,
    pub token: TokenHash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub placement: Placement,
    pub chunk: Chunk,
    pub version: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Removed {
    pub placement: Placement,
    pub version: i64,
}

pub trait WallStore: Send + Sync {
    fn region(&self, region: Region) -> Result<RegionIndex, WallError>;
    fn chunk(&self, chunk: Chunk) -> Result<ChunkBody, WallError>;
    fn mine(&self, token: &TokenHash, limit: usize) -> Result<Vec<Placement>, WallError>;
    fn place(&self, input: PlaceInput) -> Result<Placed, WallError>;
    fn remove(&self, cell: Cell) -> Result<Removed, WallError>;
    fn cell(&self, cell: Cell) -> Result<Option<Placement>, WallError>;
}
