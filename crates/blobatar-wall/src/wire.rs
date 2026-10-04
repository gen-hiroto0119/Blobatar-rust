use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkPlacement {
    pub index: i32,
    pub seed: String,
    pub expression: String,
    pub at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkBody {
    pub key: String,
    pub version: i64,
    pub cells: Vec<ChunkPlacement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkState {
    pub key: String,
    pub version: i64,
    pub count: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionIndex {
    pub chunks: Vec<ChunkState>,
    pub placements: i64,
}

struct Wire<'a>(&'a ChunkPlacement);

impl Serialize for Wire<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (&self.0.index, &self.0.seed, &self.0.expression, &self.0.at).serialize(serializer)
    }
}

pub fn encode_chunk(body: &ChunkBody) -> String {
    #[derive(Serialize)]
    struct Encoded<'a> {
        k: &'a str,
        v: i64,
        c: Vec<Wire<'a>>,
    }
    serde_json::to_string(&Encoded {
        k: &body.key,
        v: body.version,
        c: body.cells.iter().map(Wire).collect(),
    })
    .expect("wall wire values are serializable")
}
