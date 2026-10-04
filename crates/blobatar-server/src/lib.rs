mod error;
mod parse;
pub mod wall_adapter;

use axum::{
    Router,
    body::Body,
    extract::connect_info::ConnectInfo,
    http::{Method, Request, Response, StatusCode, header, uri::Authority},
};
use blobatar_core::Avatar;
use error::ApiError;
use std::net::SocketAddr;
use url::Url;

pub use wall_adapter::{VerifyError, WallAuthContext, WallService, WallVerifier};

const USAGE: &str = include_str!("reference/usage.txt");
const OPENAPI: &str = include_str!("reference/openapi.json");
const CONTENT_SECURITY_POLICY: &str = "default-src 'none'; style-src 'unsafe-inline'; sandbox";

pub fn router() -> Router {
    Router::new().fallback(handle)
}

pub fn router_with_wall(service: WallService) -> Router {
    Router::new().fallback(move |request: Request<Body>| async move {
        handle_with_wall(request, service.clone()).await
    })
}

async fn handle(request: Request<Body>) -> Response<Body> {
    let method = request.method().clone();
    let mut response = match request_url(&request) {
        Ok(url) => avatar_route(request, method.clone(), url),
        Err(error) => error::response(&error, request.headers(), USAGE),
    };

    if method == Method::HEAD {
        let (parts, _) = response.into_parts();
        response = Response::from_parts(parts, Body::empty());
    }
    response
}

async fn handle_with_wall(request: Request<Body>, service: WallService) -> Response<Body> {
    let method = request.method().clone();
    let mut response = match request_url(&request) {
        Ok(url) if url.path().starts_with("/wall/") => {
            let peer = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|info| info.0);
            service.handle(request, &url, peer).await
        }
        Ok(url) => avatar_route(request, method.clone(), url),
        Err(error) => error::response(&error, request.headers(), USAGE),
    };
    if method == Method::HEAD {
        let (parts, _) = response.into_parts();
        response = Response::from_parts(parts, Body::empty());
    }
    response
}

fn avatar_route(request: Request<Body>, method: Method, url: Url) -> Response<Body> {
    let path = url.path();
    if path == "/openapi.json" {
        openapi_response(&url.origin().ascii_serialization())
    } else if path == "/" {
        help_response()
    } else if let Some(raw_name) = path.strip_prefix("/avatar/") {
        match avatar_response(&request, &method, &url, raw_name) {
            Ok(response) => response,
            Err(error) => error::response(&error, request.headers(), USAGE),
        }
    } else {
        let error = ApiError::not_found(path);
        error::response(&error, request.headers(), USAGE)
    }
}
fn avatar_response(
    request: &Request<Body>,
    method: &Method,
    url: &Url,
    raw_name: &str,
) -> Result<Response<Body>, ApiError> {
    if method != Method::GET && method != Method::HEAD {
        return Err(ApiError::method_not_allowed(method.as_str()));
    }
    if raw_name.is_empty() {
        return Ok(help_response());
    }

    let parsed = parse::parse_options(url.query())?;
    let name = parse::parse_name(raw_name)?;
    let avatar = Avatar::with_generation(&name, &parsed.options, parsed.generation);
    let svg = avatar.svg(&parsed.options);
    let etag = etag(&svg);
    let not_modified = request
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == etag);
    let status = if not_modified {
        StatusCode::NOT_MODIFIED
    } else {
        StatusCode::OK
    };
    let cache_control = if parsed.generation_pinned {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=86400, stale-while-revalidate=2592000"
    };

    Ok(Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CACHE_CONTROL, cache_control)
        .header("content-security-policy", CONTENT_SECURITY_POLICY)
        .header("x-content-type-options", "nosniff")
        .header(header::ETAG, etag)
        .body(Body::from(if not_modified { String::new() } else { svg }))
        .expect("valid avatar response"))
}

fn request_url(request: &Request<Body>) -> Result<Url, ApiError> {
    let uri = request.uri();
    let supplied_host = request_host(request)?;
    if uri.scheme().is_some() {
        let authority = uri.authority().ok_or_else(invalid_host)?;
        validate_authority(authority.as_str())?;
        let url = Url::parse(&uri.to_string()).map_err(|_| invalid_host())?;
        if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
            return Err(invalid_host());
        }
        return Ok(url);
    }

    let authority = supplied_host
        .or_else(|| uri.authority().map(Authority::as_str))
        .unwrap_or("127.0.0.1:3000");
    let authority = validate_authority(authority)?;
    let path_and_query = uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");
    Url::parse(&format!("http://{}{path_and_query}", authority.as_str()))
        .map_err(|_| invalid_host())
}

fn request_host(request: &Request<Body>) -> Result<Option<&str>, ApiError> {
    let mut hosts = request.headers().get_all(header::HOST).iter();
    let Some(host) = hosts.next() else {
        return Ok(None);
    };
    if hosts.next().is_some() {
        return Err(invalid_host());
    }
    let host = host.to_str().map_err(|_| invalid_host())?;
    validate_authority(host)?;
    Ok(Some(host))
}

fn validate_authority(value: &str) -> Result<Authority, ApiError> {
    if value.is_empty()
        || value.ends_with(':')
        || value.chars().any(|character| {
            character.is_whitespace() || matches!(character, '/' | '\\' | '?' | '#' | '@')
        })
    {
        return Err(invalid_host());
    }
    let authority = value.parse::<Authority>().map_err(|_| invalid_host())?;
    if authority.host().is_empty()
        || authority
            .port()
            .is_some_and(|_| authority.port_u16().is_none())
    {
        return Err(invalid_host());
    }
    let url = Url::parse(&format!("http://{value}/")).map_err(|_| invalid_host())?;
    if url.host().is_none()
        || url.username() != ""
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid_host());
    }
    Ok(authority)
}

fn invalid_host() -> ApiError {
    ApiError::bad_request("invalid_host", "Invalid request host")
}

fn help_response() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .header("content-security-policy", CONTENT_SECURITY_POLICY)
        .header("x-content-type-options", "nosniff")
        .body(Body::from(USAGE))
        .expect("valid help response")
}

fn openapi_response(origin: &str) -> Response<Body> {
    let mut document: serde_json::Value =
        serde_json::from_str(OPENAPI).expect("frozen OpenAPI document is valid JSON");
    document["servers"][0]["url"] = serde_json::Value::String(origin.to_owned());
    let mut body =
        serde_json::to_string_pretty(&document).expect("OpenAPI document serializes as JSON");
    body.push('\n');
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(body))
        .expect("valid OpenAPI response")
}

fn etag(svg: &str) -> String {
    let hash = svg.encode_utf16().fold(0x811c9dc5_u32, |hash, unit| {
        (hash ^ u32::from(unit)).wrapping_mul(0x01000193)
    });
    format!("\"{}\"", base36(hash))
}

fn base36(mut value: u32) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let mut digits = Vec::new();
    while value > 0 {
        let digit = (value % 36) as u8;
        digits.push(if digit < 10 {
            char::from(b'0' + digit)
        } else {
            char::from(b'a' + digit - 10)
        });
        value /= 36;
    }
    digits.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use blobatar_core::Expression;
    use serde_json::Value;

    use crate::{
        OPENAPI,
        parse::{
            BACKGROUND_NAMES, HUE_MAX, HUE_MIN, NAME_MAX_UTF16, QUERY_PARAMETER_NAMES, SIZE_MAX,
            SIZE_MIN, TITLE_MAX_UTF16, TONE_MAX, TONE_MIN,
        },
    };

    #[test]
    fn parser_parameters_enums_and_limits_match_the_frozen_openapi_schema() {
        let document: Value = serde_json::from_str(OPENAPI).unwrap();
        let parameters = document["paths"]["/avatar/{name}"]["get"]["parameters"]
            .as_array()
            .unwrap();
        let by_name: BTreeMap<_, _> = parameters
            .iter()
            .map(|parameter| (parameter["name"].as_str().unwrap(), parameter))
            .collect();
        let query_names: BTreeSet<_> = parameters
            .iter()
            .filter(|parameter| parameter["in"] == "query")
            .map(|parameter| parameter["name"].as_str().unwrap())
            .collect();
        let parser_names: BTreeSet<_> = QUERY_PARAMETER_NAMES.iter().copied().collect();
        assert_eq!(query_names, parser_names);

        let name = &by_name["name"]["schema"];
        assert_eq!(name["minLength"].as_u64(), Some(1));
        assert_eq!(name["maxLength"].as_u64(), Some(NAME_MAX_UTF16 as u64));

        for name in ["size", "s"] {
            let schema = &by_name[name]["schema"];
            assert_eq!(schema["minimum"].as_f64(), Some(SIZE_MIN));
            assert_eq!(schema["maximum"].as_f64(), Some(SIZE_MAX));
        }
        for (name, minimum, maximum) in [("hue", HUE_MIN, HUE_MAX), ("tone", TONE_MIN, TONE_MAX)] {
            let schema = &by_name[name]["schema"];
            assert_eq!(schema["minimum"].as_f64(), Some(minimum));
            assert_eq!(schema["maximum"].as_f64(), Some(maximum));
        }
        assert_eq!(
            by_name["background"]["schema"]["enum"],
            serde_json::json!(BACKGROUND_NAMES)
        );
        let expressions: Vec<_> = Expression::ALL.into_iter().map(Expression::name).collect();
        assert_eq!(
            by_name["expression"]["schema"]["enum"],
            serde_json::json!(expressions)
        );
        assert_eq!(
            by_name["gen"]["schema"]["enum"],
            serde_json::json!(["1", "2"])
        );
        assert_eq!(
            by_name["title"]["schema"]["maxLength"].as_u64(),
            Some(TITLE_MAX_UTF16 as u64)
        );
    }
}
