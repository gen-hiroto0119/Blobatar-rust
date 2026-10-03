# Vendored xattr 0.2.3

This is upstream [`xattr` 0.2.3](https://github.com/Stebalien/xattr), from
commit `e42c1da499b72103a9c9ef5693ff31f998ad31e6`. It is distributed under MIT
or Apache-2.0; see `LICENSE-MIT` and `LICENSE-APACHE`.

The only source patch is in `src/sys/mod.rs`: Linux and Android use
`libc::ENODATA` for a missing extended attribute, while macOS and BSD continue
to use `libc::ENOATTR`.
