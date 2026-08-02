# ADR 0001: crates.io package identity

Status: Accepted

## Context

The `themis` crates.io package is owned by an unrelated project. Cargo does
not permit this repository to publish `themis 0.10.1`, and consumers would
otherwise resolve the unrelated implementation.

## Decision

Publish this repository as `themis-topology` and set the library target name
to `themis`. Consumers declare
`themis = { package = "themis-topology", ... }`, preserving Rust paths while
binding Cargo resolution to the correct package.

## Alternatives

Requesting ownership transfer does not provide a bounded release path and
depends on unrelated owners. Retaining the occupied name cannot publish the
crate and is rejected.

## Verification

`cargo package --locked` must normalize the package as
`themis-topology 0.10.1`; a clean-checkout verification build must resolve
`melinoe` from crates.io.
