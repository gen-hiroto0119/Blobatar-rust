# Blobatar Rust

Independent Rust implementation of [Blobatar](https://blobatar.dev/), targeting
native GPUI on macOS. This repository is under active development, **not a
complete port**. Runtime components must not depend on JavaScript, a WebView,
or an upstream service.

## Compatibility baseline

- Generation 2: `blobatar 2.7.0`, commit
  [`a7fd546`](https://github.com/Alain00/blobatar/tree/a7fd546ebede49d0a9fa638945b9e534489782a2).
- Complete historical Flutter vectors from 2.4.0 are retained, including
  expression data. They do not prove compatibility with all 2.7.0 behavior.
- Generation 1 will use the separately frozen `blobatar@1.0.0` implementation.
- Provenance is recorded in [docs/upstream-lock.json](docs/upstream-lock.json).
- Original upstream copyright and MIT terms are preserved in
  [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Development

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

## Delivery stages

| Stage | Scope | Status |
| --- | --- | --- |
| A/B | Workspace, generation 2 geometry, traits, colors, static SVG/URI, reference comparisons | Static core comparisons pass; integrated into native demo |
| C | GPUI vector component and macOS static image comparison | Ten shapes, fourteen static expressions and the 400-avatar size/background/surface matrix compared on macOS; see [comparison notes](docs/render-matrix.md) |
| D | 14 expressions, morph, idle motion, gaze projection | Native expression/Always/reduced-motion controls exercised on macOS; gaze adapter wired, native gaze/hover verification pending |
| E | Editor, exports/settings, reusable showcase components | Not implemented |
| F | Generation 1, avatar API, SQLite wall/API/native wall | Not implemented |
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