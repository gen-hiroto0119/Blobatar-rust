use blobatar_wall::{
    Cell, Chunk, ChunkBody, ChunkPlacement, FIRST, cell_at, cell_index, check_expression,
    check_name, chunk_of, encode_chunk, is_placeable, nearest_placeable, parse_chunk_key,
};
use serde_json::Value;

const FIXTURE: &str = include_str!("fixtures/reference.json");

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).expect("frozen wall fixture is valid JSON")
}

#[test]
fn coordinates_match_the_pinned_reference() {
    for value in fixture()["cells"].as_array().unwrap() {
        let x = value["x"].as_i64().unwrap() as i32;
        let y = value["y"].as_i64().unwrap() as i32;
        let chunk = chunk_of(x, y);
        assert_eq!(chunk.cx, value["chunk"]["cx"].as_i64().unwrap() as i32);
        assert_eq!(chunk.cy, value["chunk"]["cy"].as_i64().unwrap() as i32);
        assert_eq!(cell_index(x, y), value["index"].as_i64().unwrap() as i32);
        assert_eq!(cell_at(chunk, cell_index(x, y)), Cell { x, y });
    }
}

#[test]
fn keys_match_the_pinned_reference() {
    for value in fixture()["keys"].as_array().unwrap() {
        let parsed = parse_chunk_key(value["key"].as_str().unwrap());
        match value["parsed"].as_object() {
            Some(expected) => assert_eq!(
                parsed,
                Some(Chunk {
                    cx: expected["cx"].as_i64().unwrap() as i32,
                    cy: expected["cy"].as_i64().unwrap() as i32,
                })
            ),
            None => assert!(parsed.is_none()),
        }
    }
}

#[test]
fn reach_and_nearest_match_the_pinned_reference() {
    for value in fixture()["reach"].as_array().unwrap() {
        let occupied_cells: Vec<Cell> = value["occupied"]
            .as_array()
            .unwrap()
            .iter()
            .map(|cell| Cell {
                x: cell["x"].as_i64().unwrap() as i32,
                y: cell["y"].as_i64().unwrap() as i32,
            })
            .collect();
        let occupied = move |x: i32, y: i32| occupied_cells.contains(&Cell { x, y });
        let x = value["x"].as_i64().unwrap() as i32;
        let y = value["y"].as_i64().unwrap() as i32;
        assert_eq!(
            is_placeable(x, y, &occupied, value["populated"].as_bool().unwrap()),
            value["placeable"].as_bool().unwrap()
        );
        let expected = value["nearest"].as_object().map(|cell| Cell {
            x: cell["x"].as_i64().unwrap() as i32,
            y: cell["y"].as_i64().unwrap() as i32,
        });
        assert_eq!(
            nearest_placeable(
                Cell { x, y },
                &occupied,
                value["populated"].as_bool().unwrap(),
                128,
            ),
            expected
        );
    }
    assert_eq!(FIRST, Cell { x: 0, y: 0 });
}

#[test]
fn moderation_and_expression_match_the_pinned_reference() {
    for value in fixture()["names"].as_array().unwrap() {
        let raw = value["raw"].as_str();
        let actual = check_name(raw, Some("cafe"));
        let expected = &value["result"];
        match expected["ok"].as_bool().unwrap() {
            true => assert_eq!(actual.unwrap(), expected["name"].as_str().unwrap()),
            false => {
                let actual = actual.unwrap_err();
                assert_eq!(
                    format!("{actual:?}").to_lowercase(),
                    expected["why"].as_str().unwrap()
                );
            }
        }
    }
    for value in fixture()["expressions"].as_array().unwrap() {
        assert_eq!(
            check_expression(value["raw"].as_str()),
            value["valid"].as_bool().unwrap()
        );
    }
}

#[test]
fn wire_encoding_matches_the_pinned_reference() {
    for value in fixture()["bodies"].as_array().unwrap() {
        let body = &value["body"];
        let cells = body["cells"]
            .as_array()
            .unwrap()
            .iter()
            .map(|cell| ChunkPlacement {
                index: cell["index"].as_i64().unwrap() as i32,
                seed: cell["seed"].as_str().unwrap().to_owned(),
                expression: cell["expression"].as_str().unwrap().to_owned(),
                at: cell["at"].as_i64().unwrap(),
            })
            .collect();
        let body = ChunkBody {
            key: body["key"].as_str().unwrap().to_owned(),
            version: body["version"].as_i64().unwrap(),
            cells,
        };
        assert_eq!(encode_chunk(&body), value["encoded"].as_str().unwrap());
    }
}
