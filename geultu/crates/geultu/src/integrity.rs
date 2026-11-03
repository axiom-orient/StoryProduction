//! SHA256SUMS verification shared by the craft-pack and refinement-pack
//! loaders (H4/H5).
//!
//! The parser uses the plain `shasum -a 256 -c` record syntax:
//! each non-empty line is `<64 hex digest> SP <type char> <path>` where the
//! type char is a space (text mode, i.e. two spaces after the digest) or `*`
//! (binary mode). Digest comparison is case-insensitive, matching the Perl
//! `shasum` reference implementation. Everything else fails closed: malformed
//! lines, entries naming files that escape the pack directory (`..`,
//! absolute/prefixed paths, backslashes, NUL or non-normalized components), duplicate
//! paths, symlink/non-regular entries observed at admission and missing/modified files
//! are hard errors. Unlisted package files remain ignored; archives are streamed.
//! Text consumed by a loader is re-bound to its verified digest before parse.
//! A pack-local `SHA256SUMS` path uses the same admission rules; an explicitly
//! supplied external sums file remains a caller-selected input.
//! A pack-local `SHA256SUMS` path uses the same admission rules; an explicitly
//! supplied external sums file remains a caller-selected input.

mod parse;
mod verify;

pub(crate) use verify::read_verified_text;
pub use verify::verify_sums;
