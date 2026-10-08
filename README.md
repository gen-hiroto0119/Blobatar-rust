# Blobatar Rust

Independent Rust implementation of [Blobatar](https://blobatar.dev/), targeting
native GPUI on macOS. This repository is under active development, **not a
complete port**. Native runtime components must not depend on JavaScript, a WebView,
or an upstream service.

## Web / Worker compatibility probes (P0)

Experimental, independent workspaces in [`probes/`](probes/README.md) test
gpui-base with GPUI Web and a workers-rs SVG endpoint using the existing Rust
generation core. They do not replace the native editor or wall and are not
deployed. Browser/Worker builds use their platform's generated JavaScript glue;
the native application remains Rust/GPUI only.

## Experimental Web editor

`apps/web` runs the shared GPUI editor in the browser. Build and serve it locally:

```sh
bash tools/build-web.sh
cd apps/web
RUSTUP_TOOLCHAIN=nightly-2026-10-05 trunk serve
```

Trunk serves the editor at `http://127.0.0.1:8082/`. The Vercel configuration
declares a static build command, output directory and cross-origin isolation
headers, but a cold hosted build has not been verified. The optional private My
Wall uses GitHub sign-in through Neon Managed Better Auth and the Neon Data API.
Follow [the cloud setup guide](docs/web.md) to enable it; without configuration,
the editor and local file downloads still work. The integration requires live
OAuth/Neon validation before public release.

## Generation 1 / 2 editor

Open **エディターを開く / Open editor** in the demo, or run:

```sh
cargo run -p blobatar-demo -- --editor
```

The editor keeps the existing motion/gaze demo separate. It provides the pinned
25-axis control set, generation-specific shape/tone candidate sets, lock/unlock, name shuffle, reset,
neutral-layout eye-fit readback, seven-name crowd, backgrounds, expressions and
motion modes. All-selected **retains the candidate array** (uniform candidate
selection); Auto removes the override (the original weighted distribution).
Shape-limited controls use the union of recognized candidate silhouettes.
Advanced JSON edits the entire trait map without dropping unknown trait keys.
Unapplied JSON remains a draft across other control changes; successful Apply,
Reset and settings load replace it with the canonical state.

GPUI text input implements selection, clipboard actions, grapheme navigation and
IME composition. Tab moves between controls; sliders also accept arrows and
Home/End. Native macOS interaction and accessibility acceptance remain pending.

SVG and PNG exports are static; PNG defaults to transparent 512×512 unless a
background is selected. Save dialogs can be cancelled. Export writes a temporary
file in the destination directory and atomically replaces the chosen file only
after writing succeeds. Settings JSON is versioned and restores all Generation 2
options, expressions, candidate arrays, motion and manual reduced motion.
Both generations can be previewed, saved and restored. Generation 1 uses its
frozen six-shape layout rather than Generation 2 thresholds; expression and motion
controls apply to that layout. Switching generation preserves numeric overrides
(the same shape value can select a different silhouette). Unsupported generations,
schemas and invalid JSON are rejected without replacing the current editor state.

Code generation supports **Rust/GPUI only**, as requested. The editor displays
one Rust snippet and a **Rustコードをコピー / Copy Rust code** button, without
framework or HTTP snippet selectors. Rust/GPUI output and settings JSON carry
the full configuration. The snippet belongs in a fallible function with a GPUI
context (see the generator example). SVG/PNG/settings exports and the standalone
avatar HTTP API are separate from code generation and remain in scope.
Runtime rendering, export and snippet generation use Rust only.
The native code panel uses monospace Rust syntax highlighting for keywords,
strings (including raw JSON literals), comments, types, calls and numbers.
Highlighting changes presentation only; Copy still copies the original plain Rust.

### Editor appearance

The editor adopts the [Plexer design foundations](https://www.figma.com/design/Z20UsjBdlp8nv3IBet8Dfz/Plexer?node-id=41-205),
not its product-specific navigation or authentication flows. It starts in Light;
the header's **ダーク / Dark** or **ライト / Light** button switches the editor's
appearance without resetting the name, overrides, unapplied JSON or animation.
Appearance is local to the editor window and is not part of avatar settings JSON.
The wall, component gallery and motion demo keep their existing appearance.

Reusable tokens and button/panel styling live in `blobatar_ui::theme`. Controls
use 8px corners, panels 12px, buttons 36px and inputs 40px. Hover, keyboard focus,
selection and disabled file actions have distinct styling; invalid traits mark
the JSON field until edited or replaced. Below 980 logical pixels, the preview
and controls stack in a single scrollable column instead of clipping sideways.
The code panel deliberately stays dark in both themes to retain the existing
syntax colors; long code lines scroll horizontally and Copy uses the original
source. Its copied label resets when the generated source changes.

Inter, Noto Sans JP and IBM Plex Mono are bundled under the SIL OFL 1.1, so the
editor does not depend on fonts installed on the host or download fonts at
runtime. The font sources, hashes and license notices are in
`crates/blobatar-ui/assets/fonts/`. Japanese headings/code use Noto Sans JP as a
fallback. This is the first editor/theme milestone, not a redesign of all screens.

Frozen editor tests cover 10,624 picker transitions and 270 historical
nonempty-name readback states. The 45 pinned empty-name readback records remain
unchanged but are excluded from readback equality for the intentional
`blobatar`-seed improvement; a focused test compares empty and literal
`blobatar` names in both generations.
The fixture retains 720 historical upstream web snippets as reference data;
those formats are no longer generated or tested as supported outputs. Rust
snippet tests check preview parity and name/motion/reduced-motion preservation.
Regenerate only against the pinned, clean checkout:

```sh
bun tools/generate-editor-reference.ts ../blobatar-reference --write
# Omit --write to verify the frozen fixture without modifying it.
cargo test -p blobatar-ui --test reference
cargo test -p blobatar-export
```

## Reusable native components

Open **再利用部品を開く / Open components** in the original demo, or run
`cargo run -p blobatar-demo -- --components`. The gallery uses the public
`blobatar_ui::components` views with sample data, not a second implementation.

- `ProfileAvatar`: optional image with a deterministic missing/failed-image fallback.
- `PresenceAvatar`: online/away/offline/thinking, expression transition and unread
  badges (zero hidden, counts above 99 shown as `99+`).
- `AgentList`: caller-provided agents with unique stable IDs, working count,
  ID-based selection and `AgentSelected` events; click or Enter/Space selects a
  row. Selection survives reorder and rename while its ID remains present.
- `UserTable`: caller-provided rows, ID-based static avatars and a bounded drawing
  cache. GPUI's uniform list renders visible rows; the gallery supplies 10,000
  sample users. Rename changes the display name without changing the avatar seed.
- `GroupChat`: consecutive-sender grouping, first-message headings/times,
  member stack, overflow count and typing indicator.
- `PasswordField`: masked text, Unicode/IME-aware caret geometry and scroll/resize
  updates. Focused/hidden follows the caret, unfocused follows the pointer,
  visible uses Sleepy with Rest gaze. Its input is in-memory only, never included
  in settings, logs, exports, fixtures or demo network requests. Copy/cut do not
  put secret text on the clipboard, even while revealed. `PasswordEdited` carries
  no text; applications must explicitly read `value` if they need it.

These views reuse `AnimatedBlobatar`'s system reduced-motion behavior. Keyboard
focus is explicit, but GPUI 0.2.2 has no integrated screen-reader semantics in this
implementation: labels alone are **not** VoiceOver support. OS accessibility,
native interaction acceptance and measured large-table performance remain open.
Use sample strings, not real credentials, when testing the password demo.

## Standalone avatar HTTP API

Run the frozen avatar API locally:

```sh
cargo run -p blobatar-server
# Optional bind override:
BLOBATAR_BIND=127.0.0.1:3001 cargo run -p blobatar-server
```

The default listener is loopback-only at `127.0.0.1:3000`; the server is not a
public hosted service and makes no upstream requests. Ctrl-C shuts it down.
`GET` and `HEAD /` and `/avatar/` return help, `/openapi.json` serves the frozen
schema with its `servers` URL set from the request origin, and unknown paths
return 404. `/avatar/<name>` accepts only `GET` and `HEAD`; other methods return
405 with `Allow: GET, HEAD`.
Malformed or duplicate `Host` authorities return negotiated 400 errors. Absolute
HTTP(S) request origins take precedence over a valid `Host` header.

Avatar names strip only literal `.svg`, `.png`, `.jpg`, `.jpeg`, `.gif` and
`.webp` suffixes before percent decoding. Literal slashes are rejected;
percent-encoded slashes are allowed. Names and titles are limited by UTF-16
code units (256 and 128 respectively). Query parameters use form decoding and
the first value for each key wins; `s` takes precedence over `size`. Size uses
finite ECMAScript number syntax, clamps to 8–1024 and rounds like JavaScript.
Hue is 0–360, tone is 0–1, backgrounds are `none`, `square`, `circle` or
`squircle`, and expressions are the fourteen names listed in `/openapi.json`.
Unversioned avatars use generation 2; `gen=1` or `gen=2` pins a generation and
uses immutable caching. Exact ETag matches return 304. Errors negotiate
pretty-printed JSON when `Accept` contains `application/json` or `+json`;
otherwise they return plain text.

`Avatar::new` remains generation 2. The public
`Avatar::with_generation(name, &options, Generation::One)` factory selects the
separately frozen Generation 1 layout. Generation 1 settings in the editor and
exports are supported. An empty editor name consistently uses the `blobatar`
seed for preview, readback, export and snippets; this intentionally improves
the upstream editor's empty-name readback behavior. The SQLite wall backend
and optional server routes are implemented; standalone remote writes remain
disabled by default. A GPUI wall client is separate.

## Compatibility baseline

- Generation 2: `blobatar 2.7.0`, commit
  [`a7fd546`](https://github.com/Alain00/blobatar/tree/a7fd546ebede49d0a9fa638945b9e534489782a2).
- Complete historical Flutter vectors from 2.4.0 are retained, including
  expression data. They do not prove compatibility with all 2.7.0 behavior.
- Generation 1 uses the separately frozen `blobatar@1.0.0` implementation.
  Its archive integrity is
  `sha512-Xl122ZzoiW18Z5sLDHwcZXtHGCQwB1hN9LyUYEdjqBLJltk8h0Ap//1RWq612eABNLE957tii3aBP7hzeRXdsQ==`.
- Provenance is recorded in [docs/upstream-lock.json](docs/upstream-lock.json).
- Original upstream copyright and MIT terms are preserved in
  [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Development

See [CI checks and local reproduction](docs/ci.md) for automatic Linux/macOS,
Web and Worker gates, change-based selection, and remaining manual UI checks.

Rust is pinned by `rust-toolchain.toml`. Core logic uses `f64`, with no GPUI
dependency. Native rendering converts to `f32` only at the drawing boundary.

```sh
cargo metadata --no-deps --format-version 1
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
cargo run -p blobatar-demo
# Static comparison gallery: five sizes × four backgrounds × two host surfaces.
cargo run -p blobatar-demo -- --matrix 256 squircle dark
cargo run -p blobatar-core --example export -- Alice > avatar.svg
```

Tests consume committed expectations; they do not need Bun or network access.
To deliberately regenerate expectations from the fixed TypeScript reference,
install Bun and use a clean checkout:

```sh
git clone https://github.com/Alain00/blobatar.git .reference
git -C .reference checkout a7fd546ebede49d0a9fa638945b9e534489782a2
bun tools/generate-reference.ts .reference --write
# Verify reproducibility without overwriting:
bun tools/generate-reference.ts .reference
bun tools/generate-motion-reference.ts .reference --write
bun tools/generate-motion-reference.ts .reference
bun tools/generate-expression-reference.ts .reference --write
bun tools/generate-expression-reference.ts .reference
bun tools/generate-transform-reference.ts .reference --write
bun tools/generate-transform-reference.ts .reference
# Generation 1 and HTTP API references also verify the archived v1 package.
bun tools/generate-generation1-reference.ts .reference ../blobatar-v1-reference
bun tools/generate-generation1-reference.ts .reference ../blobatar-v1-reference --write
bun tools/generate-api-reference.ts .reference ../blobatar-v1-reference
bun tools/generate-api-reference.ts .reference ../blobatar-v1-reference --write
# Deterministic DOM/clock harness; no browser or production JS dependency.
bun tools/generate-driver-reference.ts .reference --write
bun tools/generate-driver-reference.ts .reference
bun tools/generate-gaze-transform-reference.ts .reference --write
bun tools/generate-gaze-transform-reference.ts .reference
# Requires an existing Chrome debugging endpoint. Used only for test generation.
CDP_URL=http://localhost:29229 bun tools/generate-survey-reference.ts .reference --write
CDP_URL=http://localhost:29229 bun tools/generate-survey-reference.ts .reference
# Freeze matching reference pages and their SVG inputs into a local output folder.
bun tools/generate-render-matrix.ts .reference ./render-matrix-reference
```

Review any fixture diff rather than accepting new output to hide a regression.
Hash streams, shape names, palette hex and serialized paths are compared exactly.
Layout floats use `abs(a-b) <= max(1e-12, 1e-9 * max(abs(a),abs(b)))`.
The HTTP fixture contains 244 exact request/response cases, including error
precedence, quoted/control-character values and `/avatar/` method handling; its
help, errors and OpenAPI assets are frozen from upstream commit `a7fd546`. The
Generation 1 fixture contains 1,543 cases and verifies the archive integrity
above.

## Native wall

Open **壁を開く / Open wall** in the demo, or run
`cargo run -p blobatar-demo -- --wall`. The desktop wall stores placements in a
local SQLite database and does not post to an external service; see
[docs/wall.md](docs/wall.md) for configuration and controls. The backend and
native client are implemented; native interaction and performance acceptance
remain pending.

## Delivery stages

| Stage | Scope | Status |
| --- | --- | --- |
| A/B | Workspace, generation 2 geometry, traits, colors, static SVG/URI, reference comparisons | Static core comparisons pass; integrated into native demo |
| C | GPUI vector component and macOS static image comparison | Ten shapes, fourteen static expressions and the 400-avatar size/background/surface matrix compared on macOS; see [comparison notes](docs/render-matrix.md) |
| D | 14 expressions, morph, idle motion, gaze projection | Native expression/Always/reduced-motion controls exercised on macOS; gaze adapter wired, native gaze/hover verification pending |
| E | Editor, exports/settings, reusable showcase components | Generation 2 editor, exports/settings and six reusable native views implemented; native interaction acceptance pending |
| F | Generation 1 core/editor/exports, avatar HTTP API and SQLite wall backend/API | Generation 1 core/editor/exports and frozen avatar HTTP API verified; SQLite wall backend/routes and native client implemented, native acceptance pending |
| G | Accessibility, performance evidence, packaging and user/API documentation | Not complete |

Static SVG output is an intermediate milestone, not the desktop demo or final
application. Linux test success is not proof of a macOS build or launch.

Static expressions use `Options { expression: Some(Expression::Happy), ..Default::default() }`.
The expression reference corpus covers ten shapes × fourteen expressions × four
Unicode seeds × six tone bands (3,360 cases), plus custom-palette SVG/URI cases.
Palette strings are XML-escaped in SVG attributes, a deliberate security
hardening over upstream for inputs containing markup. Valid palette output is
unchanged. URI encoding otherwise preserves upstream's raw-Unicode behavior.

`AnimatedBlobatar` retains neutral paths and interpolates the pose and fill instead
of regenerating traits on each frame. The demo exposes all fourteen expression
targets, off/hover/always modes, and an explicit reduced-motion toggle. Static and
reduced-motion modes use the baked drawing and do not schedule continuous frames.
On macOS, an app-wide NSWorkspace notification observer tracks the OS Reduce
Motion setting without polling. OS or explicit reduced motion wins over Always
and immediately finishes the current morph. Other platforms currently have only
the explicit toggle. Offscreen canvases skip motion sampling and frame requests;
window-pointer state still updates without notifying invisible views. Keyboard
navigation and performance measurements remain incomplete. Native OS-setting
and offscreen validation is pending.

The transform corpus contains 6,048 fixed-time cases: three Unicode seeds, three
shapes, fourteen expressions, sixteen times (including blink boundaries), and
three amplitudes. It compares native affine composition with the pinned
upstream SVG transform lists, including seeded eye lean and scale order.

The face fitter has 120 cases replaying bounds and fill-query results recorded
from the pinned upstream in Chrome. Its outputs and query coordinates use the
same strict floating-point tolerance as the other math tests. This verifies
the sixteen-ray fit and eye insets **given identical renderer measurements**;
it does not establish native silhouette-measurement parity. The new native
`f64` outline/survey implementation still differs from browser `getBBox` and
`isPointInFill` at rounding boundaries. That comparison remains open; the
acceptance tolerance has not been relaxed.

`GazeDriver` adds a platform-independent event/clock controller: Pointer, Point,
Element bounds, Rest and None, pursuit parking, layout remeasurement, geometry
replacement, enable/disable and permanent stop. The host owns element identity,
event subscriptions and fresh window bounds.
The driver fixture has 12 sequences / 3,384 snapshots from the actual pinned
driver with a deterministic DOM/clock harness and frozen survey measurements.
It checks internal f64 state, rounded output channels and requested-frame state.
The pinned implementation's pointer-leave sentinel aims far upper-left despite
its comment saying centre; this behavior is preserved rather than silently fixed.

The GPUI adapter installs weak window-level pointer listeners during paint,
remeasures its own bounds during prepaint (including scroll/resize), and composes
the cached per-eye projections with pose, hover, glance and blink. Its 672-case
composition fixture folds `gaze.css` arithmetic into the pinned, quantized SVG
transform lists; it is not a browser computed-style or pixel-parity claim.
Gaze holds suppress glance seeds only; breathe, bob and blink continue.

Use `AnimatedBlobatar::new(name, &options).gaze_travel(2.5)` to opt into travel
(100-unit viewBox, zero by default), then call `look_at(GazeTarget::Pointer, cx)`.
Point and Element bounds are window-local logical pixels. Element identity is
host-owned: supply updated bounds through `remeasure_gaze_target`, including zero
bounds for a detached target. The demo demonstrates moving/hidden/reappearing
element bounds; a reusable automatic element-observation wrapper is not supplied.
`None` eases home and releases idle glance; `Rest` holds it off. `stop_gaze`
discards the controller and disables its listeners; a later `look_at` installs a
fresh controller. Off and reduced motion disable gaze alongside other motion.
Native adapter interaction validation is still pending.

The native preview exposes pause/play and 0.25×/0.5×/1×/2× playback. The pure
`PlaybackClock` uses absolute wall-time anchors; changing rate or pausing does
not jump the virtual timeline. `seek_idle(Duration, cx)` accepts arbitrary
times, pauses, settles the selected expression/hover, and resets pursuit. The
demo has 0/1234/3000ms presets. Seeking is **not** a rewind of morph or gaze
event history; deterministic pursuit inspection requires replaying its input
sequence. OS/explicit reduced motion still takes precedence over playback.