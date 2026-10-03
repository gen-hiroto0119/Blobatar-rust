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
```

Review any fixture diff rather than accepting new output to hide a regression.
Hash streams, shape names, palette hex and serialized paths are compared exactly.
Layout floats use `abs(a-b) <= max(1e-12, 1e-9 * max(abs(a),abs(b)))`.

## Delivery stages

| Stage | Scope | Status |
| --- | --- | --- |
| A/B | Workspace, generation 2 geometry, traits, colors, static SVG/URI, reference comparisons | Static core comparisons pass; integrated into native demo |
| C | GPUI vector component and macOS static image comparison | Ten shapes and fourteen static expressions verified on macOS; full size/surface matrix pending |
| D | 14 expressions, morph, idle motion, gaze projection | Native morph/idle controller added; interactive visual verification and gaze input pending |
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
OS reduced-motion detection, gaze input, offscreen scheduling, keyboard navigation,
and performance measurements remain incomplete.

The transform corpus contains 6,048 fixed-time cases: three Unicode seeds, three
shapes, fourteen expressions, sixteen times (including blink boundaries), and
three amplitudes. It compares native affine composition with the pinned
upstream SVG transform lists, including seeded eye lean and scale order.