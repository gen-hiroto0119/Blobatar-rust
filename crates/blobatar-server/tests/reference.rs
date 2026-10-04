use axum::{
    body::{Body, to_bytes},
    http::{HeaderValue, Request, header},
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
async fn review_invalid_hosts_return_negotiated_errors_for_api_routes() {
    let invalid_hosts = [
        "user@api.example.test",
        "api.example.test:bad",
        "api.example.test:65536",
        "api.example.test:",
        "api.example.test/path",
        "api.example.test\\path",
        "api.example.test?query=1",
        "api.example.test#fragment",
        "api example.test",
    ];
    for host in invalid_hosts {
        for path in ["/openapi.json", "/avatar/alain"] {
            let request = Request::builder()
                .uri(path)
                .header(header::HOST, host)
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap();
            let response = blobatar_server::router().oneshot(request).await.unwrap();

            assert_eq!(response.status().as_u16(), 400, "{host} on {path}");
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                "no-store"
            );
            assert_eq!(response.headers().get(header::VARY).unwrap(), "accept");
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let envelope: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(envelope["error"]["code"], "invalid_host");
        }
    }
}

#[tokio::test]
async fn review_invalid_host_head_has_empty_body() {
    let request = Request::builder()
        .method("HEAD")
        .uri("/avatar/alain")
        .header(header::HOST, "api.example.test/path")
        .header(header::ACCEPT, "application/json")
        .body(Body::empty())
        .unwrap();
    let response = blobatar_server::router().oneshot(request).await.unwrap();

    assert_eq!(response.status().as_u16(), 400);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json; charset=utf-8"
    );
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        ""
    );
}

#[tokio::test]
async fn review_duplicate_host_and_invalid_host_on_absolute_uri_are_rejected() {
    let mut duplicate = Request::builder()
        .uri("/openapi.json")
        .body(Body::empty())
        .unwrap();
    duplicate
        .headers_mut()
        .append(header::HOST, HeaderValue::from_static("first.example.test"));
    duplicate.headers_mut().append(
        header::HOST,
        HeaderValue::from_static("second.example.test"),
    );
    duplicate
        .headers_mut()
        .insert(header::ACCEPT, HeaderValue::from_static("application/json"));

    for request in [
        duplicate,
        Request::builder()
            .uri("https://origin.example.test/openapi.json")
            .header(header::HOST, "other.example.test/path")
            .header(header::ACCEPT, "application/json")
            .body(Body::empty())
            .unwrap(),
    ] {
        let response = blobatar_server::router().oneshot(request).await.unwrap();
        assert_eq!(response.status().as_u16(), 400);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let envelope: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(envelope["error"]["code"], "invalid_host");
    }
}

#[tokio::test]
async fn review_valid_ipv6_and_absolute_https_origins_are_preserved() {
    let ipv6 = Request::builder()
        .uri("/openapi.json")
        .header(header::HOST, "[2001:db8::1]:8443")
        .body(Body::empty())
        .unwrap();
    let response = blobatar_server::router().oneshot(ipv6).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let document: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(document["servers"][0]["url"], "http://[2001:db8::1]:8443");

    let absolute = Request::builder()
        .uri("https://origin.example.test/openapi.json")
        .header(header::HOST, "other.example.test")
        .header("x-forwarded-host", "untrusted.example.test")
        .header("x-forwarded-proto", "http")
        .body(Body::empty())
        .unwrap();
    let response = blobatar_server::router().oneshot(absolute).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let document: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(document["servers"][0]["url"], "https://origin.example.test");
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
