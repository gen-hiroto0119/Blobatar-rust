use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use serde_json::Value;
use tower::ServiceExt;

const FIXTURE: &str = include_str!("fixtures/api.json");

#[tokio::test]
async fn serves_the_frozen_root_help_response() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let case = &fixture["cases"][70];
    let request = Request::builder()
        .method(case["method"].as_str().unwrap())
        .uri(case["url"].as_str().unwrap())
        .body(Body::empty())
        .unwrap();
    let response = blobatar_server::router().oneshot(request).await.unwrap();

    assert_eq!(
        response.status().as_u16(),
        case["status"].as_u64().unwrap() as u16
    );
    for (name, value) in case["responseHeaders"].as_object().unwrap() {
        assert_eq!(
            response.headers().get(name).unwrap().to_str().unwrap(),
            value.as_str().unwrap(),
            "header {name}"
        );
    }
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&body).unwrap(),
        case["body"].as_str().unwrap()
    );
}

#[tokio::test]
async fn serves_openapi_from_the_request_host_not_forwarded_headers() {
    let request = Request::builder()
        .uri("/openapi.json")
        .header("host", "api.example.test:8443")
        .header("x-forwarded-host", "untrusted.example")
        .header("x-forwarded-proto", "https")
        .body(Body::empty())
        .unwrap();
    let response = blobatar_server::router().oneshot(request).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json; charset=utf-8"
    );
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "*"
    );
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "public, max-age=3600"
    );
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let expected = include_str!("../src/reference/openapi.json")
        .replace("http://127.0.0.1:3000", "http://api.example.test:8443");
    assert_eq!(std::str::from_utf8(&body).unwrap(), expected);
}

#[tokio::test]
async fn replays_all_frozen_http_cases() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();

    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let mut request = Request::builder()
            .method(case["method"].as_str().unwrap())
            .uri(case["url"].as_str().unwrap());
        for (name, value) in case["headers"].as_object().unwrap() {
            request = request.header(name, value.as_str().unwrap());
        }
        let response = blobatar_server::router()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(
            response.status().as_u16() as u64,
            case["status"].as_u64().unwrap(),
            "case {index}: {} {}",
            case["method"].as_str().unwrap(),
            case["url"].as_str().unwrap()
        );
        for (name, value) in case["responseHeaders"].as_object().unwrap() {
            assert_eq!(
                response.headers().get(name).unwrap().to_str().unwrap(),
                value.as_str().unwrap(),
                "case {index}, header {name}"
            );
        }
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            std::str::from_utf8(&body).unwrap(),
            case["body"].as_str().unwrap(),
            "case {index} body"
        );
    }
}
