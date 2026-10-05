MIT License

Copyright (c) 2026 Alain

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR OTHER DEALINGS IN THE SOFTWARE.

## Bundled editor fonts

Inter (The Inter Project Authors), Noto Sans JP (Adobe and Noto contributors) and
IBM Plex Mono (IBM Corp.) are redistributed unmodified under SIL Open Font
License 1.1. Full copyright/license notices and original download URLs with
SHA-256 hashes are in `crates/blobatar-ui/assets/fonts/`. They are embedded for
offline native text rendering; they are not part of generated avatar exports.

## Rust code highlighting

`rustc_lexer 0.1.0` by The Rust Project Developers supplies Rust tokenization
for the native code preview. It is licensed under MIT or Apache-2.0 and comes
from [rust-lang/rust](https://github.com/rust-lang/rust/). It is used only for
presentation; generated source and clipboard data are not rewritten.

## GPUI text input

`crates/blobatar-ui/src/text_input.rs` adapts the GPUI 0.2.2
`examples/input.rs` implementation from Zed Industries under Apache-2.0.
The original notice is preserved in `crates/blobatar-ui/LICENSE-GPUI`.
Local changes integrate the editor, scope shortcuts, handle IME selection offsets,
and correct point-to-character lookup.

## xattr 0.2.3

Vendored from [Stebalien/xattr](https://github.com/Stebalien/xattr), upstream
commit `e42c1da499b72103a9c9ef5693ff31f998ad31e6`. The crate is dual-licensed
under MIT or Apache-2.0; see `vendor/xattr/LICENSE-MIT` and
`vendor/xattr/LICENSE-APACHE`. Local fixes map Linux and Android
missing-attribute errors to `libc::ENODATA` while macOS and BSD retain
`libc::ENOATTR`, clear destination buffers in `XAttrs::clone_from`, and
preserve the BSD system namespace for its final system attribute.

## Frozen avatar API reference assets

`crates/blobatar-server/src/reference/` and
`crates/blobatar-server/tests/fixtures/api.json` are frozen, upstream-derived
reference assets from Blobatar commit
`a7fd546ebede49d0a9fa638945b9e534489782a2`. Generation 1 reference cases use
the separately integrity-checked `blobatar@1.0.0` archive documented in the
README. These assets preserve the upstream MIT-licensed API contract; the
server has no JavaScript runtime or external-service dependency.
