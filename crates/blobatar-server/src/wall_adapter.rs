use std::{
    future::Future,
    net::SocketAddr,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{Body, to_bytes},
    http::{HeaderMap, Request, Response, StatusCode, header},
};
use blobatar_wall::{
    Cell, IdentityHash, PlaceInput, Region, WallError, WallStore, check_expression, check_name,
    chunk_key, day_of, encode_chunk, hash_identity, hash_token, is_token, new_token,
    parse_chunk_key, same_secret, token_from_cookie,
};
use serde_json::{Value, json};
use url::Url;

const BODY_LIMIT: usize = 16 * 1024;
const A_YEAR: &str = "public, max-age=31536000, immutable";
const COOKIE_MAX_AGE: i64 = 31_536_000;

pub trait WallVerifier: Send + Sync {
    fn verify<'a>(
        &'a self,
        context: WallAuthContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<IdentityHash, VerifyError>> + Send + 'a>>;
}

#[derive(Clone, Copy, Debug)]
pub struct WallAuthContext<'a> {
    pub peer: Option<SocketAddr>,
    pub challenge: Option<&'a str>,
    pub now: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyError {
    Missing,
    Rejected,
}

#[derive(Clone)]
pub struct WallService {
    store: Arc<dyn WallStore>,
    verifier: Option<Arc<dyn WallVerifier>>,
    local_secret: Option<String>,
    blocklist: Option<String>,
    admin_token: Option<String>,
    secure_cookie: bool,
    clock: Arc<dyn Fn() -> i64 + Send + Sync>,
}

impl WallService {
    pub fn new(store: Arc<dyn WallStore>) -> Self {
        Self {
            store,
            verifier: None,
            local_secret: None,
            blocklist: None,
            admin_token: None,
            secure_cookie: true,
            clock: Arc::new(now),
        }
    }

    pub fn with_verifier(mut self, verifier: Arc<dyn WallVerifier>) -> Self {
        self.verifier = Some(verifier);
        self
    }

    pub fn with_local_secret(mut self, secret: String) -> Self {
        self.local_secret = Some(secret);
        self
    }

    pub fn with_blocklist(mut self, blocklist: Option<String>) -> Self {
        self.blocklist = blocklist;
        self
    }

    pub fn with_admin_token(mut self, token: Option<String>) -> Self {
        self.admin_token = token.filter(|token| !token.is_empty());
        self
    }

    pub fn with_secure_cookie(mut self, secure: bool) -> Self {
        self.secure_cookie = secure;
        self
    }

    pub fn with_clock(mut self, clock: Arc<dyn Fn() -> i64 + Send + Sync>) -> Self {
        self.clock = clock;
        self
    }

    pub fn has_local_identity(&self) -> bool {
        self.local_secret.is_some()
    }

    pub async fn handle(
        &self,
        request: Request<Body>,
        url: &Url,
        peer: Option<SocketAddr>,
    ) -> Response<Body> {
        let path = url.path();
        let Some(rest) = path.strip_prefix("/wall/") else {
            return json_response(json!({"error": "no such thing"}), StatusCode::NOT_FOUND, []);
        };
        let segments: Vec<_> = rest.split('/').collect();
        match segments.as_slice() {
            ["r", key] if request.method() == axum::http::Method::GET => self.region(key).await,
            ["c", key, version] if request.method() == axum::http::Method::GET => {
                self.chunk(key, version).await
            }
            ["mine"] if request.method() == axum::http::Method::GET => {
                let cookie = request
                    .headers()
                    .get(header::COOKIE)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);
                self.mine(cookie).await
            }
            ["place"] if request.method() == axum::http::Method::POST => {
                self.place(request, url, peer).await
            }
            ["p", key] if request.method() == axum::http::Method::DELETE => {
                let authorization = request
                    .headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);
                self.remove(authorization, key).await
            }
            _ => json_response(json!({"error": "no such thing"}), StatusCode::NOT_FOUND, []),
        }
    }

    async fn region(&self, key: &str) -> Response<Body> {
        let Some(parsed) = parse_chunk_key(key) else {
            return json_response(json!({"error": "bad region"}), StatusCode::BAD_REQUEST, []);
        };
        let store = self.store.clone();
        match run_store(move || {
            store.region(Region {
                rx: parsed.cx,
                ry: parsed.cy,
            })
        })
        .await
        {
            Ok(index) => {
                let versions: serde_json::Map<_, _> = index
                    .chunks
                    .iter()
                    .map(|state| (state.key.clone(), json!(state.version)))
                    .collect();
                let full: Vec<_> = index
                    .chunks
                    .iter()
                    .filter(|state| state.count >= i64::from(blobatar_wall::CAPACITY))
                    .map(|state| state.key.clone())
                    .collect();
                json_response_with_headers(
                    json!({"r": key, "n": index.placements, "v": versions, "f": full}),
                    StatusCode::OK,
                    [("cache-control", "public, max-age=30")],
                )
            }
            Err(error) => store_error(error),
        }
    }

    async fn chunk(&self, key: &str, requested: &str) -> Response<Body> {
        let Some(parsed) = parse_chunk_key(key) else {
            return json_response(json!({"error": "bad chunk"}), StatusCode::BAD_REQUEST, []);
        };
        if requested.is_empty()
            || requested.len() > 9
            || !requested.bytes().all(|byte| byte.is_ascii_digit())
        {
            return json_response(json!({"error": "bad chunk"}), StatusCode::BAD_REQUEST, []);
        }
        let store = self.store.clone();
        let mut body = match run_store(move || store.chunk(parsed)).await {
            Ok(body) => body,
            Err(error) => return store_error(error),
        };
        body.key = key.to_owned();
        let current = requested.parse::<i64>().ok() == Some(body.version);
        let cache = if current { A_YEAR } else { "no-store" };
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .header(header::CACHE_CONTROL, cache)
            .body(Body::from(encode_chunk(&body)))
            .expect("valid wall chunk response")
    }

    async fn mine(&self, cookie: Option<String>) -> Response<Body> {
        let placements = match cookie.as_deref().and_then(token_from_cookie) {
            None => Vec::new(),
            Some(token) => {
                let token = hash_token(token);
                let store = self.store.clone();
                match run_store(move || store.mine(&token, 8)).await {
                    Ok(placements) => placements,
                    Err(error) => return store_error(error),
                }
            }
        };
        let cells: Vec<_> = placements
            .into_iter()
            .map(|placement| {
                json!({
                    "x": placement.x,
                    "y": placement.y,
                    "seed": placement.seed,
                    "at": placement.at
                })
            })
            .collect();
        json_response_with_headers(
            json!({"cells": cells}),
            StatusCode::OK,
            [("vary", "Cookie")],
        )
    }

    async fn place(
        &self,
        request: Request<Body>,
        url: &Url,
        peer: Option<SocketAddr>,
    ) -> Response<Body> {
        if self.local_secret.is_none() && self.verifier.is_none() {
            return json_response(
                json!({"error": "the wall is not configured"}),
                StatusCode::SERVICE_UNAVAILABLE,
                [],
            );
        }
        if !is_json_request(request.headers()) {
            return json_response(
                json!({"error": "json required"}),
                StatusCode::BAD_REQUEST,
                [],
            );
        }
        if self.local_secret.is_some() && !local_origin(request.headers(), url) {
            return json_response(json!({"error": "origin"}), StatusCode::FORBIDDEN, []);
        }
        let headers = request.headers().clone();
        let bytes = match to_bytes(request.into_body(), BODY_LIMIT).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return json_response(json!({"error": "bad request"}), StatusCode::BAD_REQUEST, []);
            }
        };
        let value: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => {
                return json_response(json!({"error": "bad request"}), StatusCode::BAD_REQUEST, []);
            }
        };
        let Some(object) = value.as_object() else {
            return json_response(json!({"error": "bad request"}), StatusCode::BAD_REQUEST, []);
        };
        let seed = match object.get("seed").and_then(Value::as_str) {
            Some(seed) => match check_name(Some(seed), self.blocklist.as_deref()) {
                Ok(seed) => seed,
                Err(reason) => {
                    return json_response(
                        json!({"error": "name", "why": format!("{reason:?}").to_lowercase()}),
                        StatusCode::UNPROCESSABLE_ENTITY,
                        [],
                    );
                }
            },
            None => {
                return json_response(
                    json!({"error": "name", "why": "empty"}),
                    StatusCode::UNPROCESSABLE_ENTITY,
                    [],
                );
            }
        };
        let expression = match object.get("expression").and_then(Value::as_str) {
            Some(expression) if check_expression(Some(expression)) => expression.to_owned(),
            _ => {
                return json_response(
                    json!({"error": "expression"}),
                    StatusCode::UNPROCESSABLE_ENTITY,
                    [],
                );
            }
        };
        let x = match integer_field(object.get("x")) {
            Some(value) => value,
            None => {
                return json_response(
                    json!({"error": "off the wall"}),
                    StatusCode::BAD_REQUEST,
                    [],
                );
            }
        };
        let y = match integer_field(object.get("y")) {
            Some(value) => value,
            None => {
                return json_response(
                    json!({"error": "off the wall"}),
                    StatusCode::BAD_REQUEST,
                    [],
                );
            }
        };
        if x.unsigned_abs() > 1_000_000 || y.unsigned_abs() > 1_000_000 {
            return json_response(
                json!({"error": "off the wall"}),
                StatusCode::BAD_REQUEST,
                [],
            );
        }
        let now = (self.clock)();
        let identity = match self
            .identity(peer, object.get("turnstile").and_then(Value::as_str), now)
            .await
        {
            Ok(identity) => identity,
            Err(VerifyError::Missing) => {
                return json_response(
                    json!({"error": "the wall is not configured"}),
                    StatusCode::SERVICE_UNAVAILABLE,
                    [],
                );
            }
            Err(VerifyError::Rejected) => {
                return json_response(json!({"error": "challenge"}), StatusCode::FORBIDDEN, []);
            }
        };
        let token = headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(token_from_cookie)
            .map(str::to_owned)
            .unwrap_or_else(new_token);
        let input = PlaceInput {
            cell: Cell { x, y },
            seed,
            expression,
            now,
            identity,
            token: hash_token(&token),
        };
        let store = self.store.clone();
        match run_store(move || store.place(input)).await {
            Ok(placed) => {
                let cookie = format!(
                    "wall={token}; Path=/; Max-Age={COOKIE_MAX_AGE}; HttpOnly;{} SameSite=Lax",
                    if self.secure_cookie { " Secure;" } else { "" }
                );
                json_response_with_headers(
                    json!({
                        "x": placed.placement.x,
                        "y": placed.placement.y,
                        "at": placed.placement.at,
                        "seed": placed.placement.seed,
                        "chunk": chunk_key(placed.chunk),
                        "version": placed.version
                    }),
                    StatusCode::CREATED,
                    [("set-cookie", cookie.as_str())],
                )
            }
            Err(error) => store_error(error),
        }
    }

    async fn remove(&self, authorization: Option<String>, key: &str) -> Response<Body> {
        let Some(expected) = self.admin_token.as_deref() else {
            return json_response(json!({"error": "no such thing"}), StatusCode::NOT_FOUND, []);
        };
        let offered = authorization
            .as_deref()
            .and_then(|value| value.strip_prefix("Bearer "));
        if offered.is_none_or(|value| !same_secret(value, expected)) {
            return json_response(json!({"error": "no such thing"}), StatusCode::NOT_FOUND, []);
        }
        let Some(parsed) = parse_cell_key(key) else {
            return json_response(json!({"error": "bad cell"}), StatusCode::BAD_REQUEST, []);
        };
        let store = self.store.clone();
        match run_store(move || store.remove(parsed)).await {
            Ok(removed) => json_response(
                json!({
                    "removed": {
                        "x": removed.placement.x,
                        "y": removed.placement.y,
                        "seed": removed.placement.seed
                    }
                }),
                StatusCode::OK,
                [],
            ),
            Err(WallError::NotFound) => {
                json_response(json!({"error": "nobody there"}), StatusCode::NOT_FOUND, [])
            }
            Err(error) => store_error(error),
        }
    }

    async fn identity(
        &self,
        peer: Option<SocketAddr>,
        challenge: Option<&str>,
        now: i64,
    ) -> Result<IdentityHash, VerifyError> {
        if let Some(secret) = &self.local_secret {
            let identity = peer
                .map(|peer| peer.ip().to_string())
                .unwrap_or_else(|| "loopback".to_owned());
            return Ok(hash_identity(&identity, &day_of(now), secret));
        }
        let Some(verifier) = &self.verifier else {
            return Err(VerifyError::Missing);
        };
        verifier
            .verify(WallAuthContext {
                peer,
                challenge,
                now,
            })
            .await
    }
}

async fn run_store<T, F>(operation: F) -> Result<T, WallError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, WallError> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|_| WallError::Database("wall store task failed".to_owned()))?
}

fn integer_field(value: Option<&Value>) -> Option<i32> {
    let number = value?.as_number()?;
    let value = number.as_i64().or_else(|| {
        let value = number.as_f64()?;
        (value.is_finite() && value.fract() == 0.0)
            .then_some(value as i64)
            .filter(|integer| *integer as f64 == value)
    })?;
    i32::try_from(value).ok()
}

fn parse_cell_key(key: &str) -> Option<Cell> {
    let parsed = parse_chunk_key(key)?;
    Some(Cell {
        x: parsed.cx,
        y: parsed.cy,
    })
}

fn is_json_request(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(header::CONTENT_TYPE).iter();
    let Some(value) = values.next() else {
        return false;
    };
    values.next().is_none()
        && value.to_str().ok().is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
        })
}

fn local_origin(headers: &HeaderMap, request_url: &Url) -> bool {
    let mut origins = headers.get_all(header::ORIGIN).iter();
    let Some(origin) = origins.next() else {
        return false;
    };
    if origins.next().is_some() {
        return false;
    }
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(origin) = Url::parse(origin) else {
        return false;
    };
    let local_host = match origin.host() {
        Some(url::Host::Domain(host)) => host == "localhost",
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    matches!(origin.scheme(), "http" | "https")
        && origin.scheme() == request_url.scheme()
        && local_host
        && origin.username().is_empty()
        && origin.password().is_none()
        && origin.path() == "/"
        && origin.query().is_none()
        && origin.fragment().is_none()
        && origin.port_or_known_default() == request_url.port_or_known_default()
}

fn store_error(error: WallError) -> Response<Body> {
    match error {
        WallError::InvalidName(reason) => json_response(
            json!({"error": "name", "why": format!("{reason:?}").to_lowercase()}),
            StatusCode::UNPROCESSABLE_ENTITY,
            [],
        ),
        WallError::InvalidExpression => json_response(
            json!({"error": "expression"}),
            StatusCode::UNPROCESSABLE_ENTITY,
            [],
        ),
        WallError::InvalidCoordinates => json_response(
            json!({"error": "off the wall"}),
            StatusCode::BAD_REQUEST,
            [],
        ),
        WallError::Cooldown { until } => json_response(
            json!({"error": "cooldown", "until": until}),
            StatusCode::TOO_MANY_REQUESTS,
            [],
        ),
        WallError::Unplaceable { nearest } => json_response(
            json!({"error": "unplaceable", "nearest": nearest}),
            StatusCode::CONFLICT,
            [],
        ),
        WallError::Taken => json_response(json!({"error": "taken"}), StatusCode::CONFLICT, []),
        WallError::Database(_) => json_response(
            json!({"error": "wall database"}),
            StatusCode::INTERNAL_SERVER_ERROR,
            [],
        ),
        WallError::NotFound => {
            json_response(json!({"error": "nobody there"}), StatusCode::NOT_FOUND, [])
        }
    }
}

fn json_response<const N: usize>(
    value: Value,
    status: StatusCode,
    headers: [(&str, &str); N],
) -> Response<Body> {
    json_response_with_headers(value, status, headers)
}

fn json_response_with_headers<const N: usize>(
    value: Value,
    status: StatusCode,
    headers: [(&str, &str); N],
) -> Response<Body> {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8");
    let cache_control_present = headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("cache-control"));
    if !cache_control_present {
        builder = builder.header(header::CACHE_CONTROL, "no-store");
    }
    for (name, value) in headers {
        builder = builder.header(name, value);
    }
    builder
        .body(Body::from(
            serde_json::to_string(&value).expect("wall response serializes"),
        ))
        .expect("valid wall JSON response")
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after Unix epoch")
        .as_secs() as i64
}

pub fn load_local_secret(database: &Path) -> Result<String, std::io::Error> {
    let path = local_secret_path(database);
    match std::fs::read_to_string(&path) {
        Ok(secret) => {
            let secret = validate_local_secret(secret)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            }
            Ok(secret)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let secret = new_token();
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(mut file) => {
                    use std::io::Write;
                    file.write_all(secret.as_bytes())?;
                    file.sync_all()?;
                    Ok(secret)
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let secret = std::fs::read_to_string(&path)?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
                    }
                    validate_local_secret(secret)
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

fn local_secret_path(database: &Path) -> PathBuf {
    let mut path = database.as_os_str().to_owned();
    path.push(".wall-secret");
    PathBuf::from(path)
}

fn validate_local_secret(secret: String) -> Result<String, std::io::Error> {
    let secret = secret.trim().to_owned();
    if is_token(&secret) {
        Ok(secret)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid wall local secret",
        ))
    }
}
