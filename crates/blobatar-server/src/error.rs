use std::{collections::HashMap, sync::OnceLock};

use axum::{
    body::Body,
    http::{HeaderMap, Response, StatusCode, header},
};
use serde::Serialize;

#[derive(Debug)]
pub(crate) struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl ApiError {
    pub(crate) fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
        }
    }

    pub(crate) fn not_found(path: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: format!("no route for {path}"),
        }
    }

    pub(crate) fn method_not_allowed(method: &str) -> Self {
        Self {
            status: StatusCode::METHOD_NOT_ALLOWED,
            code: "method_not_allowed",
            message: format!("{method} not allowed"),
        }
    }
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
    hint: &'a str,
    status: u16,
    documentation: &'static str,
}

const ERROR_HINTS: &str = include_str!("reference/errors.json");
const DOCUMENTATION: &str = "https://blobatar.dev/docs";
const CONTENT_SECURITY_POLICY: &str = "default-src 'none'; style-src 'unsafe-inline'; sandbox";

static HINTS: OnceLock<HashMap<String, String>> = OnceLock::new();

fn hint(code: &str) -> &'static str {
    HINTS
        .get_or_init(|| {
            serde_json::from_str(ERROR_HINTS).expect("frozen error hints are valid JSON")
        })
        .get(code)
        .map(String::as_str)
        .unwrap_or_default()
}

pub(crate) fn response(error: &ApiError, headers: &HeaderMap, usage: &str) -> Response<Body> {
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let json = accept.contains("application/json") || accept.contains("+json");

    let body = if json {
        let envelope = ErrorEnvelope {
            error: ErrorBody {
                code: error.code,
                message: &error.message,
                hint: hint(error.code),
                status: error.status.as_u16(),
                documentation: DOCUMENTATION,
            },
        };
        let mut body =
            serde_json::to_string_pretty(&envelope).expect("error envelope serializes as JSON");
        body.push('\n');
        body
    } else {
        match error.status {
            StatusCode::BAD_REQUEST => format!("{}\n\n{usage}", error.message),
            StatusCode::NOT_FOUND => usage.to_owned(),
            _ => error.message.clone(),
        }
    };

    let content_type = if json {
        "application/json; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    };
    let mut builder = Response::builder()
        .status(error.status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::VARY, "accept")
        .header("content-security-policy", CONTENT_SECURITY_POLICY)
        .header("x-content-type-options", "nosniff");
    if error.status == StatusCode::METHOD_NOT_ALLOWED {
        builder = builder.header(header::ALLOW, "GET, HEAD");
    }
    builder
        .body(Body::from(body))
        .expect("valid error response")
}
