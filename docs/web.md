# GPUI Web editor and private My Wall

The browser runs the same Rust/GPUI editor as native. Native keeps GPUI 0.2.2;
the browser target uses pinned gpui-pre 0.3.8 with gpui-base 0.7.1. JavaScript is
limited to browser file/clipboard operations and the managed authentication/data
SDK; there is no React/Vue UI or separate web reimplementation.

## Local build

Use Node 24.19.0 and run `bash tools/build-web.sh` from the repository root. It
installs pinned Rust/Trunk if necessary, installs the locked Node dependencies,
bundles the cloud bridge, and emits `apps/web/dist`. Then run:

```sh
cd apps/web
RUSTUP_TOOLCHAIN=nightly-2026-10-05 trunk serve
```

The development URL is `http://127.0.0.1:8082`. If editing the JavaScript bridge,
run `npm run build` in `apps/web` before rebuilding/serving the Rust application.
The SVG/PNG/JSON downloads and JSON import do not require cloud configuration.
PNG is a 512px still image, not an animation. File import is limited to 1 MiB.

## Cloud architecture

```text
Vercel static assets → GPUI Web / Wasm
                        ├─ GitHub sign-in → Neon Managed Better Auth
                        └─ authenticated HTTPS → Neon Data API → PostgreSQL RLS
```

Use Neon **Free**, AWS **Singapore (`aws-ap-southeast-1`)**. Managed Better Auth
is Beta; this is an explicit product choice. The SDK is pinned to
`@neondatabase/neon-js` 0.7.0-beta. No Firebase service is used.

The browser only receives a public HTTPS endpoint and its own authentication
session. Never put a Postgres connection string, Neon API key, GitHub client
secret, or privileged database credential into the frontend. There are no
Vercel Functions in this architecture: auth and the Data API run on Neon.

## Provisioning checklist (manual, not performed by this code)

1. Create the Free Neon project in Singapore, optionally through the Vercel
   Marketplace integration. Confirm the plan/region before creating it; moving
   regions later requires migration.
2. Enable Managed Better Auth and GitHub OAuth. Create a GitHub OAuth App with
   callback `{NEON_AUTH_BASE_URL}/callback/github`. Enter its client ID and secret
   **in Neon**, not in this repository. Do not request `repo` permissions. Disable
   unneeded sign-in methods in Neon rather than relying on the UI to hide them.
3. Add the exact production origin and explicit development origins to the Auth
   trusted-domain allowlist. Do not allow all `*.vercel.app` hosts.
4. Enable the Data API for the chosen branch/database, with Managed Better Auth
   as the JWT provider. Leave automatic broad public-schema grants disabled;
   this migration grants access only to the application's table.
5. Apply `database/001_my_wall.sql` as the database owner, once, before enabling
   user traffic. It is transactional and intentionally not a re-runnable reset.
   Refresh the Data API schema cache and run Neon's Data API security advisors.
6. Set `NEON_PUBLIC_DATABASE_URL` in Vercel's **Production** build environment to
   the public HTTPS database URL from Neon, for example the form
   `https://ep-example.ap-southeast-1.aws.neon.tech/neondb`. Use the real URL shown
   by Neon; do not invent a hostname from this example. This is not `DATABASE_URL`
   and must not contain credentials or query parameters. Rebuild after changes.
7. Keep Preview cloud configuration unset initially. Never expose production
   data/auth to untrusted PR deployments. Dedicated preview branches need their
   own schema, trusted origins and GitHub OAuth callback configuration.

Neon validates JWTs, then Postgres grants and row-level policies restrict every
operation to the authenticated owner. Anonymous requests have no table grants.
Clients may write only `settings`; IDs, ownership and timestamps are assigned
server-side. Updates/deletes include the loaded revision to reject stale writes.

## Usage and recovery

- Open My Wall, sign in with GitHub, return to the editor and save your work.
- Open a saved card to edit it, update it, or save a separate copy.
- Deleting a card requires a second confirmation click. There is no recycle bin.
- The list is paginated in groups of 12. Export settings JSON for a portable backup.
- An OAuth redirect temporarily preserves the current settings in sessionStorage.
  It is not a durable cloud backup; download important drafts before clearing
  browser data. Unapplied advanced-JSON text is not part of saved settings.
- My Wall is a private collection, not the native spatial wall or a shared feed.
- Free-tier capacity, availability, rate limits and retention still apply. There
  is currently no application-level per-user storage quota or automated backup.

## Deployment and release gates

Import this repository into Vercel with the repository root as Root Directory,
Framework Preset **Other**, Node **24.x**, and the checked-in build/output settings.
`vercel.json` adds COOP/COEP and the Wasm content type. A cold Vercel build may be
significantly slower than a cached local build; hosted build limits, startup time,
headers, and the public URL must be verified on a real deployment before release.
Do not weaken COOP/COEP just to bypass an authentication or asset error.

Required live checks: GitHub redirect/return, session restoration, sign-out,
create/reload/update/copy/delete across two devices, and two different accounts
plus an unauthenticated client proving ownership isolation. Local bridge mocks
and the RLS SQL test do not prove the real OAuth/JWT integration.

## Focused checks

```sh
npm --prefix apps/web run check
npm --prefix apps/web test
cargo +nightly-2026-10-05 clippy --locked -p blobatar-web --target wasm32-unknown-unknown -- -D warnings
bash tools/build-web.sh
```

Run `psql -v ON_ERROR_STOP=1 -f database/tests/ownership.sql` only against a
disposable local Postgres instance as its administrator. It creates mock auth
roles and a test-only `auth.user_id()` function, then verifies the real migration's
grants/RLS with two users and an anonymous role. Never run that test on Neon or an
existing application database.

References: [Neon OAuth](https://neon.com/docs/auth/guides/setup-oauth),
[Data API](https://neon.com/docs/data-api/get-started),
[Access control](https://neon.com/docs/data-api/access-control).
