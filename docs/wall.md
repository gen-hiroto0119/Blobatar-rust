# Wall backend

The wall is a durable SQLite-backed placement service exposed by
`blobatar-server`. The Rust implementation does not run JavaScript at
runtime. `blobatar-wall` owns the geometry, moderation, identity helpers,
wire format, `WallStore` trait, and SQLite implementation.

## Running locally

The server still binds to `127.0.0.1:3000` by default. The relevant
environment variables are:

* `BLOBATAR_BIND` selects a parsed `SocketAddr`.
* `BLOBATAR_WALL_DB` selects the SQLite database (default:
  `blobatar-wall.sqlite3`).
* `BLOBATAR_WALL_LOCAL_IDENTITY` enables local writes. This is rejected unless
  `BLOBATAR_BIND` is a loopback address. A random secret is persisted beside
  the database in a private `<database>.wall-secret` file; the secret is never
  stored in SQLite or logged.
* `BLOBATAR_WALL_BLOCKLIST` adds comma-separated moderation terms.
* `BLOBATAR_WALL_ADMIN_TOKEN` enables the administrative delete route. Leave it
  unset to keep that route hidden.

Without an injected verifier or local identity mode, the standalone server is
read-only and wall writes return `503`. An embedding application can implement
`blobatar_server::wall_adapter::WallVerifier`. It receives trusted connection
information and the challenge value, rather than caller-supplied forwarded or
Cloudflare identity headers, and returns a date-salted `IdentityHash`.

Local HTTP writes intentionally omit the cookie `Secure` attribute so a
browser can use them over `http://127.0.0.1`. Deployments using HTTPS should
leave secure cookies enabled. Wall cookies are `wall=` followed by 64 lowercase
hexadecimal characters, with `HttpOnly` and `SameSite=Lax`.

## Routes and caching

* `GET /wall/r/<region>` returns a region index. It is public-cacheable for
  30 seconds.
* `GET /wall/c/<chunk>/<version>` returns compact `{k,v,c}` chunk JSON.
  Current versions are immutable for one year; stale requested versions return
  the current body with `no-store`.
* `GET /wall/mine` reads the caller's cookie and sends `Vary: Cookie`.
* `POST /wall/place` validates JSON, moderation, coordinates, reach, quota, and
  identity before one atomic SQLite transaction.
* `DELETE /wall/p/<cell>` is hidden with `404` unless an admin bearer token is
  configured.

Write and error responses use `no-store`; wall responses do not enable blanket
CORS. Browser local writes require an `application/json` content type and a
loopback `Origin`. Request bodies are bounded to 16 KiB.

SQLite stores only SHA-256 identity and token hashes. Identity hashes use
`SHA-256(secret:YYYY-MM-DD:trusted-identity)`, so daily quota identifiers are
not reusable across UTC days; token hashes use unsalted SHA-256 because the
cookie tokens contain 256 bits of randomness. Daily quota, placement insertion,
chunk version/count, and total placement count are committed together. Removal
increments the chunk version and decrements counts but never refunds quota.
Separate connections use SQLite busy timeouts and immediate write
transactions, while region and chunk reads use one snapshot each.
