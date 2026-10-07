# P0: GPUI Base / Workers compatibility probes

These are small, non-production experiments, not the migrated Blobatar editor.
Each has its own Cargo workspace and lockfile. The root workspace, GPUI 0.2.2,
native UI initialization, local SQLite wall and native server remain unchanged.
Only `blobatar-core` is shared with the product in this phase.

## Pinned dependency set

- UI: `gpui-base = 0.7.1`, `gpui-pre` / `gpui-pre-platform = 0.3.8`.
  Base's published manifest pins this GPUI snapshot; its web platform resolves
  to `gpui-pre-web = 0.3.8`. Do not mix it with GPUI 0.2.2 entities or types.
- Worker: `worker = 0.8.0`, `worker-build = 0.8.0`, Wrangler `4.148.0`.
- Native / Worker: the root Rust 1.97.1 toolchain.
- Browser: `nightly-2026-10-05`, `wasm32-unknown-unknown`, Trunk `0.21.14`.

The UI now uses the upstream **published snapshots**, rather than the Git-only
dependency set in the initial architecture proposal. Matching versions and both
lockfiles are kept explicitly. Neither probe uses gpui-component or a WebView.

## GPUI Base: one screen, two targets

```sh
cd probes/gpui-base
cargo run --locked
# Separate command, in the same directory:
RUSTUP_TOOLCHAIN=nightly-2026-10-05 trunk serve --no-autoreload
# Optimized browser build:
RUSTUP_TOOLCHAIN=nightly-2026-10-05 trunk build --release
```

The native and browser entry points render the same Rust view. Base owns the
InputState/Input, Button activation and Root focus traversal. Application code
owns the Plexer-derived spacing/colors and calls the existing core for SVGs.
The existing Noto Sans JP asset is reused with its existing OFL license.
The preview uses GPUI's SVG image loader, **not** the existing animated GPUI
path renderer: neither animation nor renderer parity is established here.

The input is a draft. Generate or unmodified Enter in the input applies it;
Generation toggles 1/2 and applies
the current input. Empty input becomes `blobatar`, without trimming nonempty
names. Activations counts generation actions so duplicate Enter/Space handling
is observable. Copy SVG copies the applied SVG, not the pending name. It uses
GPUI's clipboard abstraction and makes no claim that browser permission was
granted; production clipboard failure feedback remains a P2 requirement.
Light/Dark switches Plexer-derived application colors and Base's semantic theme.

This first probe uses `single_threaded_web()` as the upstream Base example does.
It does not prove the multithreaded configuration or either renderer backend
individually. Trunk supplies COOP/COEP headers locally so later threaded probes
can retain the same hosting boundary. The HTML contains no separate UI: Trunk
only bootstraps the GPUI canvas.

Manual acceptance: edit/select/copy/paste a name; Generate; Tab/Shift+Tab across
input and buttons; Enter/Space activate exactly once; switch both generations;
empty input; Japanese text; Copy SVG into a local plain-text document; resize.
Japanese text entry alone is not proof of IME composition or accessibility.

## workers-rs: core SVGs in workerd

Prerequisites: Node.js >=22/npm, Python 3, and:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.97.1
cargo install worker-build --version 0.8.0 --locked
cd probes/worker
npm ci
npm run dev
# In another terminal, in probes/worker:
npm run smoke
```

`wrangler dev --local` binds loopback; no Cloudflare account, secret, binding,
login or deployment is needed. Do not run deploy/login/provisioning commands.
`workers_dev` and preview URLs are disabled; no deployment script is provided.
The build tool emits minimal JS runtime glue around the Rust Worker Wasm.
Wrangler 4.148.0's Miniflare dependency currently pins vulnerable sharp 0.35.4.
The scoped npm override pins it to 0.35.5 per
[GHSA-wq5f-xc86-pv6w](https://github.com/advisories/GHSA-wq5f-xc86-pv6w);
remove the override once Wrangler/Miniflare pins sharp at 0.35.5 or newer.

Supported **probe-only** contract:

| Route | Behavior |
| --- | --- |
| `GET /healthz` | Plain-text probe identification |
| `GET /probe/avatar.svg?name=...&generation=2` | Existing core SVG, generation 1 or 2 |
| `HEAD` on these routes | Same status/type, empty body |

Missing/empty name uses `blobatar`; generation defaults to 2. Names are limited
to 256 UTF-8 bytes. Unknown/duplicate parameters and invalid generations return
400, unsupported methods on known routes return 405 with Allow, unknown paths
return 404. All responses are no-store and nosniff. There are no cookies,
storage writes, CORS grants, wall operations or secrets. URL query decoding uses
the standard `url` crate; the complete native API's query/error/cache semantics
are **not** implemented. The distinct `/probe/` prefix prevents confusion with
the existing `/avatar/*` contract.

## Checks

Run from each probe directory, respectively:

```sh
# gpui-base
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
RUSTUP_TOOLCHAIN=nightly-2026-10-05 trunk build --release

# worker
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --target wasm32-unknown-unknown -- -D warnings
worker-build --release
# With local workerd running:
python3 smoke.py
```

Existing core regression gate, from the repository root:
`cargo test --locked -p blobatar-core`.

## Gates before P1/P2 or publication

- Run the pinned UI on native macOS and in a real browser; builds alone are not
  acceptance. Record any blocked target explicitly.
- Move existing GPUI adapters and UI together to a single snapshot, then check
  `gpui_base::init` and `blobatar_ui::init` interactions. **Coexistence has not
  been established by these isolated workspaces.**
- Extract platform-specific file/clipboard/clock/random/settings/wall access;
  connect the real editor, rather than growing this probe into a second editor.
- Establish browser file I/O, PNG export, IME, accessibility, animation, renderer
  fallback, clipboard rejection handling and native regressions.
- Measure the **actual editor's** release size/startup/memory. This probe's size
  is not a shipping budget; Static Assets currently limits one file to 25 MiB.
- Extract/test the complete API contract before implementing Worker `/avatar/*`.
  Wall storage atomicity/quota/reach/version/moderation and public identity remain
  separate gates; no Durable Object/D1 decision is finalized by the SVG probe.
- Cloudflare account/bindings/secrets/deployment require a separate approval.

References: [GPUI Base](https://gpui-kit.com/base/),
[published snapshot pins](https://docs.rs/crate/gpui-base/0.7.1/source/Cargo.toml),
[workers-rs](https://github.com/cloudflare/workers-rs),
[Cloudflare Rust guide](https://developers.cloudflare.com/workers/languages/rust/).
