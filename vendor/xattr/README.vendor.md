# Vendored xattr 0.2.3

This is upstream [`xattr` 0.2.3](https://github.com/Stebalien/xattr), from
commit `e42c1da499b72103a9c9ef5693ff31f998ad31e6`. It is distributed under MIT
or Apache-2.0; see `LICENSE-MIT` and `LICENSE-APACHE`.

Local source patches:

- `src/sys/mod.rs` maps missing extended attributes to `libc::ENODATA` on Linux
  and Android, retaining `libc::ENOATTR` on macOS and BSD.
- `src/sys/linux_macos/mod.rs` and `src/sys/bsd.rs` clear destination buffers
  before copying them in `XAttrs::clone_from`.
- `src/sys/bsd.rs` keeps the system namespace on the final system attribute.
