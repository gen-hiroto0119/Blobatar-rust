use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use blobatar_wall::{
    Cell, IdentityHash, PlaceInput, SQLiteStore, TokenHash, WallError, WallStore, hash_identity,
};

fn path() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("blobatar-wall-{suffix}.sqlite"))
}

fn input(cell: Cell, identity: &IdentityHash, token: &TokenHash, now: i64) -> PlaceInput {
    PlaceInput {
        cell,
        seed: "Alex".to_owned(),
        expression: "thinking".to_owned(),
        now,
        identity: identity.clone(),
        token: token.clone(),
    }
}

fn identity(value: &str) -> IdentityHash {
    hash_identity(value, "2026-01-01", "test-secret")
}

fn token(value: &str) -> TokenHash {
    blobatar_wall::hash_token(value)
}

#[test]
fn persistence_negative_coordinates_and_origin_survive_reopen() {
    let database = path();
    let store = SQLiteStore::open(&database).unwrap();
    let id = identity("one");
    let placed = store
        .place(input(Cell { x: 0, y: 0 }, &id, &token("one"), 1))
        .unwrap();
    assert_eq!(placed.version, 1);
    let negative = store
        .place(input(
            Cell { x: -1, y: 0 },
            &identity("two"),
            &token("two"),
            2,
        ))
        .unwrap();
    assert_eq!(negative.chunk.cx, -1);
    drop(store);

    let reopened = SQLiteStore::open(&database).unwrap();
    assert_eq!(
        reopened.cell(Cell { x: 0, y: 0 }).unwrap().unwrap().seed,
        "Alex"
    );
    assert_eq!(
        reopened
            .region(blobatar_wall::Region { rx: 0, ry: 0 })
            .unwrap()
            .placements,
        2
    );
    let _ = std::fs::remove_file(database);
}

#[test]
fn region_coordinates_outside_integer_bounds_fail_without_panicking() {
    let store = SQLiteStore::in_memory().unwrap();
    assert!(matches!(
        store.region(blobatar_wall::Region {
            rx: i32::MAX,
            ry: 0
        }),
        Err(WallError::InvalidCoordinates)
    ));
}

#[test]
fn diagonal_reach_and_nearest_suggestion_are_euclidean() {
    let store = SQLiteStore::in_memory().unwrap();
    store
        .place(input(
            Cell { x: 0, y: 0 },
            &identity("one"),
            &token("one"),
            1,
        ))
        .unwrap();
    let too_far = store.place(input(
        Cell { x: 32, y: 32 },
        &identity("two"),
        &token("two"),
        2,
    ));
    assert!(matches!(
        too_far,
        Err(WallError::Unplaceable { nearest: Some(_) })
    ));
}

#[test]
fn removal_does_not_refund_quota_and_versions_are_monotonic() {
    let store = SQLiteStore::in_memory().unwrap();
    let first_id = identity("one");
    store
        .place(input(Cell { x: 0, y: 0 }, &first_id, &token("one"), 86_399))
        .unwrap();
    let second = store
        .place(input(
            Cell { x: 1, y: 0 },
            &identity("two"),
            &token("two"),
            86_399,
        ))
        .unwrap();
    assert_eq!(second.version, 2);
    let removed = store.remove(Cell { x: 1, y: 0 }).unwrap();
    assert_eq!(removed.version, 3);
    let refunded = store.place(input(Cell { x: 1, y: 0 }, &first_id, &token("one"), 86_399));
    assert!(matches!(refunded, Err(WallError::Cooldown { .. })));
    let placed_next_day = store
        .place(input(Cell { x: 1, y: 0 }, &first_id, &token("one"), 86_400))
        .unwrap();
    assert_eq!(placed_next_day.version, 4);
}

#[test]
fn separate_connections_serialize_same_cell_and_identity_races() {
    let database = path();
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for value in ["one", "two"] {
        let database = database.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let store = SQLiteStore::open(database).unwrap();
            barrier.wait();
            (
                value,
                store.place(input(
                    Cell { x: 0, y: 0 },
                    &identity(value),
                    &token(value),
                    1,
                )),
            )
        }));
    }
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(
        outcomes.iter().filter(|(_, result)| result.is_ok()).count(),
        1
    );
    let loser = outcomes
        .iter()
        .find(|(_, result)| result.is_err())
        .unwrap()
        .0;
    let retry = SQLiteStore::open(&database).unwrap().place(input(
        Cell { x: 1, y: 0 },
        &identity(loser),
        &token(loser),
        1,
    ));
    assert!(retry.is_ok(), "failed same-cell race must not spend quota");
    drop(outcomes);

    let same_identity = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for x in [3, 4] {
        let database = database.clone();
        let barrier = Arc::clone(&same_identity);
        handles.push(thread::spawn(move || {
            let store = SQLiteStore::open(database).unwrap();
            barrier.wait();
            store.place(input(
                Cell { x, y: 0 },
                &identity("three"),
                &token("three"),
                86_400,
            ))
        }));
    }
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| matches!(result, Err(WallError::Cooldown { .. })))
            .count(),
        1
    );
    let _ = std::fs::remove_file(database);
}
