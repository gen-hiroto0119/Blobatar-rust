use std::{future::Future, pin::Pin, sync::Arc};

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use blobatar_server::{VerifyError, WallAuthContext, WallService, WallVerifier, router_with_wall};
use blobatar_wall::{SQLiteStore, WallStore};
use serde_json::Value;
use tower::ServiceExt;

fn local_service(store: Arc<SQLiteStore>) -> WallService {
    WallService::new(store)
        .with_local_secret("a".repeat(64))
        .with_secure_cookie(false)
        .with_clock(Arc::new(|| 86_399))
}

fn request(method: Method, uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "127.0.0.1:3000")
        .header(header::ORIGIN, "http://127.0.0.1:3000")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

async fn body(
    response: axum::response::Response<Body>,
) -> (StatusCode, String, axum::http::HeaderMap) {
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, 20 * 1024).await.unwrap();
    (
        parts.status,
        String::from_utf8(bytes.to_vec()).unwrap(),
        parts.headers,
    )
}

#[tokio::test]
async fn wall_is_read_only_without_an_injected_writer() {
    let store = Arc::new(SQLiteStore::in_memory().unwrap());
    let response = router_with_wall(WallService::new(store.clone()))
        .oneshot(request(Method::POST, "/wall/place", r#"{"x":0,"y":0}"#))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        store
            .region(blobatar_wall::Region { rx: 0, ry: 0 })
            .unwrap()
            .placements,
        0
    );
}

#[tokio::test]
async fn wall_routing_keeps_avatar_routes_and_hides_wrong_methods() {
    let app = router_with_wall(local_service(Arc::new(SQLiteStore::in_memory().unwrap())));
    let avatar = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/avatar/alain")
                .header(header::HOST, "127.0.0.1:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(avatar.status(), StatusCode::OK);
    let wrong_method = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/wall/place")
                .header(header::HOST, "127.0.0.1:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wrong_method.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn local_place_sets_cookie_and_cache_routes_match_version_semantics() {
    let store = Arc::new(SQLiteStore::in_memory().unwrap());
    let app = router_with_wall(local_service(store.clone()));
    let response = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/wall/place",
            r#"{"x":0,"y":0,"seed":" Alex ","expression":"thinking"}"#,
        ))
        .await
        .unwrap();
    let (status, response_body, headers) = body(response).await;
    assert_eq!(status, StatusCode::CREATED);
    let placed: Value = serde_json::from_str(&response_body).unwrap();
    assert_eq!(placed["seed"], "Alex");
    let cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert!(cookie.starts_with("wall="));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(!cookie.contains("Secure"));
    let token = cookie
        .strip_prefix("wall=")
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    assert!(blobatar_wall::is_token(&token));

    let region = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/wall/r/0_0")
                .header(header::HOST, "127.0.0.1:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        region.headers().get(header::CACHE_CONTROL).unwrap(),
        "public, max-age=30"
    );
    assert!(
        region
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );
    let region: Value =
        serde_json::from_slice(&to_bytes(region.into_body(), 1024).await.unwrap()).unwrap();
    assert_eq!(region["n"], 1);
    assert_eq!(region["v"]["0_0"], 1);

    let stale = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/wall/c/0_0/0")
                .header(header::HOST, "127.0.0.1:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::OK);
    assert_eq!(
        stale.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );
    let stale_body = to_bytes(stale.into_body(), 1024).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&stale_body).unwrap(),
        r#"{"k":"0_0","v":1,"c":[[0,"Alex","thinking",86399]]}"#
    );

    let current = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/wall/c/0_0/1")
                .header(header::HOST, "127.0.0.1:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        current.headers().get(header::CACHE_CONTROL).unwrap(),
        "public, max-age=31536000, immutable"
    );

    let mine = app
        .oneshot(
            Request::builder()
                .uri("/wall/mine")
                .header(header::HOST, "127.0.0.1:3000")
                .header(header::COOKIE, format!("wall={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mine.headers().get(header::VARY).unwrap(), "Cookie");
    let mine: Value =
        serde_json::from_slice(&to_bytes(mine.into_body(), 1024).await.unwrap()).unwrap();
    assert_eq!(mine["cells"][0]["seed"], "Alex");
}

#[tokio::test]
async fn malformed_bodies_and_local_csrf_fail_safely() {
    let app = router_with_wall(local_service(Arc::new(SQLiteStore::in_memory().unwrap())));
    for payload in ["", "null", "[]", "{"] {
        let response = app
            .clone()
            .oneshot(request(Method::POST, "/wall/place", payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{payload:?}");
    }

    let no_json = Request::builder()
        .method(Method::POST)
        .uri("/wall/place")
        .header(header::HOST, "127.0.0.1:3000")
        .header(header::ORIGIN, "http://127.0.0.1:3000")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(no_json).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );

    let foreign_origin = Request::builder()
        .method(Method::POST)
        .uri("/wall/place")
        .header(header::HOST, "127.0.0.1:3000")
        .header(header::ORIGIN, "https://evil.example")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"x":0,"y":0,"seed":"Alex","expression":"thinking"}"#,
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(foreign_origin).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );

    let too_large = Request::builder()
        .method(Method::POST)
        .uri("/wall/place")
        .header(header::HOST, "127.0.0.1:3000")
        .header(header::ORIGIN, "http://127.0.0.1:3000")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            r#"{{"x":0,"y":0,"seed":"{}"}}"#,
            "a".repeat(17 * 1024)
        )))
        .unwrap();
    assert_eq!(
        app.oneshot(too_large).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn validation_errors_and_injected_challenge_rejection_keep_statuses() {
    let app = router_with_wall(local_service(Arc::new(SQLiteStore::in_memory().unwrap())));
    for (payload, expected) in [
        (
            r#"{"seed":" ","expression":"?","x":0,"y":0}"#,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            r#"{"seed":"Alex","expression":"?","x":0,"y":0}"#,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            r#"{"seed":"Alex","expression":"thinking","x":1000001,"y":0}"#,
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(Method::POST, "/wall/place", payload))
                .await
                .unwrap()
                .status(),
            expected
        );
    }

    struct RejectingVerifier;
    impl WallVerifier for RejectingVerifier {
        fn verify<'a>(
            &'a self,
            _context: WallAuthContext<'a>,
        ) -> Pin<
            Box<dyn Future<Output = Result<blobatar_wall::IdentityHash, VerifyError>> + Send + 'a>,
        > {
            Box::pin(std::future::ready(Err(VerifyError::Rejected)))
        }
    }

    let remote = router_with_wall(
        WallService::new(Arc::new(SQLiteStore::in_memory().unwrap()))
            .with_verifier(Arc::new(RejectingVerifier)),
    );
    let response = remote
        .oneshot(request(
            Method::POST,
            "/wall/place",
            r#"{"seed":"Alex","expression":"thinking","x":0,"y":0}"#,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn forwarded_identity_headers_are_ignored_and_admin_route_is_hidden() {
    let store = Arc::new(SQLiteStore::in_memory().unwrap());
    let app = router_with_wall(local_service(store.clone()));
    let first = Request::builder()
        .method(Method::POST)
        .uri("/wall/place")
        .header(header::HOST, "127.0.0.1:3000")
        .header(header::ORIGIN, "http://127.0.0.1:3000")
        .header(header::CONTENT_TYPE, "application/json")
        .header("cf-connecting-ip", "192.0.2.1")
        .header("x-forwarded-for", "192.0.2.1")
        .body(Body::from(
            r#"{"x":0,"y":0,"seed":"Alex","expression":"thinking"}"#,
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(first).await.unwrap().status(),
        StatusCode::CREATED
    );
    let second = Request::builder()
        .method(Method::POST)
        .uri("/wall/place")
        .header(header::HOST, "127.0.0.1:3000")
        .header(header::ORIGIN, "http://127.0.0.1:3000")
        .header(header::CONTENT_TYPE, "application/json")
        .header("cf-connecting-ip", "198.51.100.2")
        .header("x-forwarded-for", "198.51.100.2")
        .body(Body::from(
            r#"{"x":1,"y":0,"seed":"Taylor","expression":"thinking"}"#,
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(second).await.unwrap().status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        app.oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/wall/p/0_0")
                .header(header::HOST, "127.0.0.1:3000")
                .header(header::AUTHORIZATION, "Bearer guess")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn configured_admin_token_can_remove_a_wall_cell() {
    let store = Arc::new(SQLiteStore::in_memory().unwrap());
    store
        .place(blobatar_wall::PlaceInput {
            cell: blobatar_wall::Cell { x: 0, y: 0 },
            seed: "Alex".to_owned(),
            expression: "thinking".to_owned(),
            now: 1,
            identity: blobatar_wall::hash_identity("loopback", "1970-01-01", "secret"),
            token: blobatar_wall::hash_token("test-cookie"),
        })
        .unwrap();
    let app = router_with_wall(
        local_service(store.clone()).with_admin_token(Some("test-admin-token".to_owned())),
    );

    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/wall/p/0_0")
                .header(header::HOST, "127.0.0.1:3000")
                .header(header::AUTHORIZATION, "Bearer incorrect")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);

    let removed = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/wall/p/0_0")
                .header(header::HOST, "127.0.0.1:3000")
                .header(header::AUTHORIZATION, "Bearer test-admin-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(removed.status(), StatusCode::OK);
    assert_eq!(
        store.cell(blobatar_wall::Cell { x: 0, y: 0 }).unwrap(),
        None
    );
}
