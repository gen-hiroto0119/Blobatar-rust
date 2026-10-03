# Static rendering comparison

Native product revision: `116d80367cf50b3a36986dfbd67ef0a21aa8128d`.
Reference: Blobatar 2.7.0, `a7fd546ebede49d0a9fa638945b9e534489782a2`.

The same seed (`ひろと`) and ten shape overrides cover 24/40/64/128/256px,
none/circle/squircle/square backgrounds and light/dark surfaces: 40 pages and
400 avatars. This is the static size/surface matrix, not motion or gaze evidence.

Native captures used macOS 26.5.2, Xcode 26.6 / SDK 26.5, Rust 1.97.1 and GPUI
0.2.2. Full-desktop images are 1600×1200; window images are 1500×882, including
the titlebar. Chromium 137.0.7118.2 reference captures used a 1500×850 viewport
and device pixel ratio 1. The titlebar/fonts are excluded from avatar metrics.

All ten square-background pages independently give the same alignment:
native window pixels are reference pixels translated by `(0,37)`. No resizing,
rotation or independent per-avatar registration was used. Raw RGB palette
values match; this is not a color-calibrated display/ICC comparison.

For every avatar, the comparison includes a 3px surrounding guard. A reference
edge is any pixel differing from a horizontal/vertical neighbor. All changed
pixels fall on that band or its one-pixel expansion; there are **zero differences
farther away**, and **zero non-surface pixels outside the one-pixel outer guard**.
Maximum per-avatar mean absolute channel error, including the guard, is
2.517037 on a 0–255 scale. Full native samples were also inspected across every
size/background/surface category: no visible overlap, missing shapes or clipping.
The observed residual is consistent with edge rasterization differences, not
proof of bit-identical renderers. Core mathematical tolerances are unchanged.

## Reproduce

1. Run `bun tools/generate-render-matrix.ts PINNED_CHECKOUT REFERENCE_DIRECTORY`.
2. Capture all generated pages in Chromium at the dimensions/DPR above; record
   each SVG's `getBoundingClientRect()` as `bounds.json` (`[{id,bounds:[...]}]`).
3. On macOS, build/run `blobatar-demo --matrix SIZE BACKGROUND SURFACE` for all
   combinations. Capture whole windows as `window/SIZE-BACKGROUND-SURFACE.png`.
   Retain a `manifest.json` containing `commit` and a `files` array of
   `{path,sha256}` entries. Do not substitute Linux software-GPU captures.
4. With Python, Pillow and NumPy installed, run:

   ```sh
   python tools/compare-render-matrix.py --native NATIVE_DIRECTORY \
     --reference REFERENCE_DIRECTORY --out COMPARISON_DIRECTORY
   ```

The script verifies checksums and alignment anchors, writes all full-page pairs
and a 400-row `comparison.json`, and reports outliers. It is a diagnostic report,
not an automatic acceptance of arbitrary image differences. Inspect the images
and source geometry before accepting a new result. Captures and reports are
session attachments rather than source-controlled binaries.
