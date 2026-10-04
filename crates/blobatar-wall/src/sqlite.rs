use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::{
    Cell, Chunk, ChunkBody, ChunkPlacement, ChunkState, PlaceInput, Placed, Placement, Region,
    RegionIndex, Removed, TokenHash, WallError, WallStore, cell_index, chunk_key, chunk_of, day_of,
    is_placeable, nearest_placeable,
};

const MAX_COORDINATE: i32 = 1_000_000;
const SUGGESTION_RADIUS: i32 = 128;
const SUGGESTION_HALO: i32 = SUGGESTION_RADIUS + crate::REACH;

#[derive(Clone)]
pub struct SQLiteStore {
    connection: Arc<Mutex<Connection>>,
}

impl SQLiteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WallError> {
        let connection = Connection::open(path).map_err(database)?;
        Self::from_connection(connection)
    }

    pub fn in_memory() -> Result<Self, WallError> {
        Self::from_connection(Connection::open_in_memory().map_err(database)?)
    }

    fn from_connection(connection: Connection) -> Result<Self, WallError> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(database)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 PRAGMA journal_mode = WAL;
                 CREATE TABLE IF NOT EXISTS placements (
                   x INTEGER NOT NULL,
                   y INTEGER NOT NULL,
                   cx INTEGER NOT NULL,
                   cy INTEGER NOT NULL,
                   seed TEXT NOT NULL,
                   expression TEXT NOT NULL,
                   at INTEGER NOT NULL,
                   ip_hash TEXT NOT NULL,
                   token_hash TEXT NOT NULL,
                   PRIMARY KEY (x, y)
                 );
                 CREATE INDEX IF NOT EXISTS placements_by_chunk ON placements (cx, cy);
                 CREATE INDEX IF NOT EXISTS placements_by_token ON placements (token_hash);
                 CREATE TABLE IF NOT EXISTS quota (
                   ip_hash TEXT NOT NULL,
                   day TEXT NOT NULL,
                   PRIMARY KEY (ip_hash, day)
                 );
                 CREATE TABLE IF NOT EXISTS chunks (
                   cx INTEGER NOT NULL,
                   cy INTEGER NOT NULL,
                   version INTEGER NOT NULL CHECK (version >= 1),
                   count INTEGER NOT NULL CHECK (count >= 0 AND count <= 1024),
                   PRIMARY KEY (cx, cy)
                 );
                 CREATE TABLE IF NOT EXISTS meta (
                   k TEXT NOT NULL PRIMARY KEY,
                   v INTEGER NOT NULL CHECK (v >= 0)
                 );
                 INSERT INTO meta (k, v) VALUES ('placements', 0)
                   ON CONFLICT (k) DO NOTHING;",
            )
            .map_err(database)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    ) -> Result<T, WallError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| WallError::Database("connection mutex poisoned".to_owned()))?;
        operation(&connection).map_err(database)
    }

    fn with_transaction<T>(
        &self,
        behavior: TransactionBehavior,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, WallError>,
    ) -> Result<T, WallError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| WallError::Database("connection mutex poisoned".to_owned()))?;
        let transaction = connection
            .transaction_with_behavior(behavior)
            .map_err(database)?;
        let result = operation(&transaction)?;
        transaction.commit().map_err(database)?;
        Ok(result)
    }
}

impl WallStore for SQLiteStore {
    fn region(&self, region: Region) -> Result<RegionIndex, WallError> {
        self.with_transaction(TransactionBehavior::Deferred, |transaction| {
            let Some(cx0) = region.rx.checked_mul(crate::REGION) else {
                return Err(WallError::InvalidCoordinates);
            };
            let Some(cy0) = region.ry.checked_mul(crate::REGION) else {
                return Err(WallError::InvalidCoordinates);
            };
            let Some(cx1) = cx0.checked_add(crate::REGION - 1) else {
                return Err(WallError::InvalidCoordinates);
            };
            let Some(cy1) = cy0.checked_add(crate::REGION - 1) else {
                return Err(WallError::InvalidCoordinates);
            };
            let mut statement = transaction
                .prepare(
                    "SELECT cx, cy, version, count FROM chunks
                     WHERE cx BETWEEN ?1 AND ?2 AND cy BETWEEN ?3 AND ?4
                     ORDER BY cy, cx",
                )
                .map_err(database)?;
            let chunks = statement
                .query_map(params![cx0, cx1, cy0, cy1], |row| {
                    let chunk = Chunk {
                        cx: row.get(0)?,
                        cy: row.get(1)?,
                    };
                    Ok(ChunkState {
                        key: chunk_key(chunk),
                        version: row.get(2)?,
                        count: row.get(3)?,
                    })
                })
                .map_err(database)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database)?;
            let placements = transaction
                .query_row("SELECT v FROM meta WHERE k = 'placements'", [], |row| {
                    row.get(0)
                })
                .map_err(database)?;
            Ok(RegionIndex { chunks, placements })
        })
    }

    fn chunk(&self, chunk: Chunk) -> Result<ChunkBody, WallError> {
        self.with_transaction(TransactionBehavior::Deferred, |transaction| {
            let state = transaction
                .query_row(
                    "SELECT version FROM chunks WHERE cx = ?1 AND cy = ?2",
                    params![chunk.cx, chunk.cy],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(database)?;
            let version = state.unwrap_or(0);
            let mut statement = transaction
                .prepare(
                    "SELECT x, y, seed, expression, at FROM placements
                     WHERE cx = ?1 AND cy = ?2 ORDER BY at, x, y",
                )
                .map_err(database)?;
            let cells = statement
                .query_map(params![chunk.cx, chunk.cy], |row| {
                    let x: i32 = row.get(0)?;
                    let y: i32 = row.get(1)?;
                    Ok(ChunkPlacement {
                        index: cell_index(x, y),
                        seed: row.get(2)?,
                        expression: row.get(3)?,
                        at: row.get(4)?,
                    })
                })
                .map_err(database)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database)?;
            Ok(ChunkBody {
                key: chunk_key(chunk),
                version,
                cells,
            })
        })
    }

    fn mine(&self, token: &TokenHash, limit: usize) -> Result<Vec<Placement>, WallError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT x, y, seed, expression, at FROM placements
                 WHERE token_hash = ?1 ORDER BY at DESC, x, y LIMIT ?2",
            )?;
            statement
                .query_map(params![token.as_str(), limit as i64], placement_from_row)?
                .collect()
        })
    }

    fn place(&self, input: PlaceInput) -> Result<Placed, WallError> {
        let input = validate_input(input)?;
        let day = day_of(input.now);
        self.with_transaction(TransactionBehavior::Immediate, |transaction| {
            let size: i64 = transaction
                .query_row("SELECT v FROM meta WHERE k = 'placements'", [], |row| {
                    row.get(0)
                })
                .map_err(database)?;
            let spent_today = transaction
                .query_row(
                    "SELECT EXISTS (
                       SELECT 1 FROM quota WHERE ip_hash = ?1 AND day = ?2
                     )",
                    params![input.identity.as_str(), day],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(database)?;
            if spent_today {
                return Err(WallError::Cooldown {
                    until: crate::next_midnight(input.now),
                });
            }
            let nearby = cells_in(
                transaction,
                input.cell.x - crate::REACH,
                input.cell.y - crate::REACH,
                input.cell.x + crate::REACH,
                input.cell.y + crate::REACH,
            )?;
            let occupied: HashSet<_> = nearby.into_iter().collect();
            let occupied_lookup = move |x: i32, y: i32| occupied.contains(&Cell { x, y });
            if !is_placeable(input.cell.x, input.cell.y, &occupied_lookup, size > 0) {
                let wide = cells_in(
                    transaction,
                    input.cell.x - SUGGESTION_HALO,
                    input.cell.y - SUGGESTION_HALO,
                    input.cell.x + SUGGESTION_HALO,
                    input.cell.y + SUGGESTION_HALO,
                )?;
                let wide: HashSet<_> = wide.into_iter().collect();
                let lookup = move |x: i32, y: i32| wide.contains(&Cell { x, y });
                let nearest = nearest_placeable(input.cell, &lookup, size > 0, SUGGESTION_RADIUS);
                return Err(WallError::Unplaceable { nearest });
            }

            let quota_inserted = transaction
                .execute(
                    "INSERT INTO quota (ip_hash, day) VALUES (?1, ?2)
                     ON CONFLICT (ip_hash, day) DO NOTHING",
                    params![input.identity.as_str(), day],
                )
                .map_err(database)?;
            if quota_inserted == 0 {
                return Err(WallError::Cooldown {
                    until: crate::next_midnight(input.now),
                });
            }

            let chunk = chunk_of(input.cell.x, input.cell.y);
            let placement_inserted = transaction
                .execute(
                    "INSERT INTO placements
                       (x, y, cx, cy, seed, expression, at, ip_hash, token_hash)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT (x, y) DO NOTHING",
                    params![
                        input.cell.x,
                        input.cell.y,
                        chunk.cx,
                        chunk.cy,
                        input.seed,
                        input.expression,
                        input.now,
                        input.identity.as_str(),
                        input.token.as_str()
                    ],
                )
                .map_err(database)?;
            if placement_inserted == 0 {
                return Err(WallError::Taken);
            }

            transaction
                .execute(
                    "INSERT INTO chunks (cx, cy, version, count) VALUES (?1, ?2, 1, 1)
                     ON CONFLICT (cx, cy) DO UPDATE
                       SET version = version + 1, count = count + 1",
                    params![chunk.cx, chunk.cy],
                )
                .map_err(database)?;
            transaction
                .execute(
                    "INSERT INTO meta (k, v) VALUES ('placements', 1)
                     ON CONFLICT (k) DO UPDATE SET v = v + 1",
                    [],
                )
                .map_err(database)?;
            let version = transaction
                .query_row(
                    "SELECT version FROM chunks WHERE cx = ?1 AND cy = ?2",
                    params![chunk.cx, chunk.cy],
                    |row| row.get(0),
                )
                .map_err(database)?;
            Ok(Placed {
                placement: Placement {
                    x: input.cell.x,
                    y: input.cell.y,
                    seed: input.seed.clone(),
                    expression: input.expression.clone(),
                    at: input.now,
                },
                chunk,
                version,
            })
        })
    }

    fn remove(&self, cell: Cell) -> Result<Removed, WallError> {
        self.with_transaction(TransactionBehavior::Immediate, |transaction| {
            let placement = transaction
                .query_row(
                    "SELECT x, y, seed, expression, at FROM placements WHERE x = ?1 AND y = ?2",
                    params![cell.x, cell.y],
                    placement_from_row,
                )
                .optional()
                .map_err(database)?
                .ok_or(WallError::NotFound)?;
            let chunk = chunk_of(cell.x, cell.y);
            transaction
                .execute(
                    "DELETE FROM placements WHERE x = ?1 AND y = ?2",
                    params![cell.x, cell.y],
                )
                .map_err(database)?;
            transaction
                .execute(
                    "UPDATE chunks SET version = version + 1, count = MAX(count - 1, 0)
                     WHERE cx = ?1 AND cy = ?2",
                    params![chunk.cx, chunk.cy],
                )
                .map_err(database)?;
            transaction
                .execute(
                    "UPDATE meta SET v = MAX(v - 1, 0) WHERE k = 'placements'",
                    [],
                )
                .map_err(database)?;
            let version = transaction
                .query_row(
                    "SELECT version FROM chunks WHERE cx = ?1 AND cy = ?2",
                    params![chunk.cx, chunk.cy],
                    |row| row.get(0),
                )
                .map_err(database)?;
            Ok(Removed { placement, version })
        })
    }

    fn cell(&self, cell: Cell) -> Result<Option<Placement>, WallError> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT x, y, seed, expression, at FROM placements WHERE x = ?1 AND y = ?2",
                    params![cell.x, cell.y],
                    placement_from_row,
                )
                .optional()
        })
    }
}

fn validate_input(mut input: PlaceInput) -> Result<PlaceInput, WallError> {
    if i64::from(input.cell.x).abs() > i64::from(MAX_COORDINATE)
        || i64::from(input.cell.y).abs() > i64::from(MAX_COORDINATE)
    {
        return Err(WallError::InvalidCoordinates);
    }
    input.seed = crate::check_name(Some(&input.seed), None).map_err(WallError::InvalidName)?;
    if !crate::check_expression(Some(&input.expression)) {
        return Err(WallError::InvalidExpression);
    }
    Ok(input)
}

fn cells_in(
    transaction: &Transaction<'_>,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> Result<Vec<Cell>, WallError> {
    let mut statement = transaction
        .prepare(
            "SELECT x, y FROM placements
             WHERE x BETWEEN ?1 AND ?2 AND y BETWEEN ?3 AND ?4",
        )
        .map_err(database)?;
    statement
        .query_map(params![x0, x1, y0, y1], |row| {
            Ok(Cell {
                x: row.get(0)?,
                y: row.get(1)?,
            })
        })
        .map_err(database)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database)
}

fn placement_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Placement> {
    Ok(Placement {
        x: row.get(0)?,
        y: row.get(1)?,
        seed: row.get(2)?,
        expression: row.get(3)?,
        at: row.get(4)?,
    })
}

fn database(error: rusqlite::Error) -> WallError {
    WallError::Database(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_database_failure_rolls_back_quota_and_is_not_taken() {
        let store = SQLiteStore::in_memory().unwrap();
        let input = PlaceInput {
            cell: Cell { x: 0, y: 0 },
            seed: "Alex".to_owned(),
            expression: "thinking".to_owned(),
            now: 86_399,
            identity: crate::hash_identity("peer", "1970-01-01", "secret"),
            token: crate::hash_token(&"a".repeat(64)),
        };
        store
            .connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_placement BEFORE INSERT ON placements
                 BEGIN SELECT RAISE(ABORT, 'forced failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            store.place(input.clone()),
            Err(WallError::Database(_))
        ));
        store
            .connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_placement")
            .unwrap();
        assert!(store.place(input).is_ok());
    }
}
