# Atrium OAuth security patch

This directory contains `atrium-oauth` 0.1.7 from crates.io, corresponding to
[upstream commit 1762e5b](https://github.com/atrium-rs/atrium/tree/1762e5bcf00fb2a038fa4a94777acdd9cb6d4b70/atrium-oauth).
The upstream MIT license is retained in `LICENSE`. Source (including unit tests), README and
changelog are retained; the normalized crate manifest is used without its generated
header. Upstream's stable rustfmt settings are retained locally.

The local patch upgrades P-256 signing from p256 0.13 / ecdsa 0.16 /
elliptic-curve 0.13 to p256 0.14 and its current RustCrypto dependencies. This removes
crypto-bigint 0.5, affected by
[RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387).
The published Atrium OAuth 0.2.0 also retains the affected dependencies, so a release
upgrade alone cannot fix this finding.

`jose-jwk` remains responsible for the existing serialized data types, with its
optional crypto features disabled. `src/p256_jwk.rs` adapts those types to the new
P-256 API, validates lengths, scalar range, curve and public/private consistency,
and preserves the existing EC/P-256 `x`, `y`, `d` representation. Malformed saved
keys now fail explicitly. Valid saved keys retain their identity and token binding.
JWT signing still emits ES256's fixed-width 64-byte signature. OAuth transport,
claims, session persistence, token refresh and nonce retry remain upstream code.

Changed integration points are `utils.rs`, `jose/signing.rs`, `keyset.rs` and
`http_client/dpop.rs`, plus the new adapter and its module declaration. Upstream
PEM tests use the new P-256 API. The standalone tutorial examples and their
Hickory dependency are excluded; Threadline supplies its own transport and resolvers. Added regression
tests cover saved/generated keys, invalid keys, independently verified JWTs,
public-only headers and nonce retries. The app workspace includes this crate so
CI runs its checks and tests too.

Remove this patch when an upstream release uses a non-vulnerable crypto stack.
First verify `cargo audit`, the full workspace tests, independent JWT verification
and saved-session compatibility, then remove the Cargo patch and workspace member.
