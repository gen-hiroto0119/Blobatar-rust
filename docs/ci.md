# Continuous integration

The `CI` GitHub Actions workflow runs on pull requests, pushes to `main`, and
manual dispatches. It does not deploy, publish packages, access Cloudflare, or
change branch protection. Actions are pinned by commit, Rust comes from
`rust-toolchain.toml`, and Cargo/npm builds use committed lockfiles.

| Check | Scope |
| --- | --- |
| Native (Ubuntu 24.04 / macOS 15) | Workspace fmt, tests, clippy, build; the GPUI Base probe's native checks when present and affected |
| GPUI Web build | Pinned nightly `nightly-2026-10-05`, Trunk `0.21.14`, release Wasm build |
| Worker tests and HTTP smoke | Pinned Node `24.19.0`, worker-build `0.8.0`, native tests, native/Wasm clippy, release build, real local workerd HTTP smoke |
| CI result | Stable aggregate gate: every selected job must succeed; a selected job being skipped or cancelled is a failure |

The probe jobs activate when their workspaces are present. This allows CI to be
merged independently of the P0 probe PR. Until then their absence is reported in
the selection job summary, **not** represented as successful probe verification.
Merging the probes triggers their checks on the resulting `main` revision.

## Cost and check selection

- Changes to native crates, apps, vendored code, root Cargo files or the pinned
  upstream provenance select native checks. Core and root `Cargo.toml` changes
  also select both probes; root `Cargo.lock` changes select native checks only
  because each probe has its own lockfile.
- GPUI probe or bundled-font changes select native probe checks and the Web build.
- Worker changes select only Worker checks unless another affected path matches.
- CI or Rust toolchain/configuration changes select every available workspace.
- Documentation-only changes skip builds. Manual dispatch checks every available
  workspace, regardless of changed paths.
- Cargo dependencies/build artifacts and installed Rust tools are cached separately
  by job/platform. npm's download cache is restored before `npm ci`.
- A new commit cancels an older run for the same PR/ref. Jobs have time limits and
  do not upload large build artifacts by default.

`CI result` is the intended required check if maintainers later enable branch
protection. This PR does not configure repository policies. Fork PRs use the
ordinary `pull_request` event with read-only permissions, not
`pull_request_target`; no credentials or deployment secrets are supplied.

## Local reproduction and limits

Native checks (Linux requires the development libraries listed in the workflow):

```sh
cargo fmt --all --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked --workspace
```

When the probes are present, their README documents the matching local commands.
The workflow runs the locked Worker release build, then starts Wrangler locally;
Wrangler runs its configured custom build again before serving. CI waits for
`/healthz`, then runs the HTTP smoke script with a timeout. Its log is printed on
both failure and success; no Cloudflare account or token is needed.

CI compilation/tests are not proof of real UI rendering, Japanese IME,
VoiceOver, WebGPU support, or performance. Those remain separate acceptance
checks. Cold GPUI builds can be expensive, especially on hosted macOS runners;
monitor actual run time before expanding the matrix.
