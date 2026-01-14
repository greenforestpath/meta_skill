# Bundle Format and Manifest

This document describes the `ms` bundle packaging format used by `ms bundle create` and `ms bundle install`.

## Bundle Layout

Bundles are stored as a single binary file with a deterministic layout:

```
Header: "MSBUNDLE1\0"
u64: manifest length (bytes, big-endian)
manifest: UTF-8 TOML (BundleManifest)
u64: blob count (N)
for each blob:
  u64: hash length
  hash bytes (e.g. "sha256:...")
  u64: blob length
  blob bytes
```

The manifest includes a checksum of the bundle contents and optional signature entries.

## Manifest Schema (TOML)

```toml
[bundle]
id = "rust-toolkit"
name = "Rust Toolkit"
version = "1.0.0"
description = "Shared Rust skills"
authors = ["Team <dev@example.com>"]
license = "MIT"
repository = "https://example.com/rust-toolkit"
keywords = ["rust", "tooling"]
ms_version = ">=0.1.0"

[[skills]]
name = "error-handling"
path = "skills/by-id/error-handling"
version = "1.2.0"
hash = "sha256:..."

[[skills]]
name = "async-patterns"
path = "skills/by-id/async-patterns"
version = "0.5.0"
hash = "sha256:..."
optional = true

[[dependencies]]
id = "core-utils"
version = "^1.0"
optional = true
```

## Checksums and Blobs

- Each blob is hashed with SHA-256 (`sha256:<hex>`).
- Bundle checksum is computed from the manifest (with checksum cleared) plus sorted blob hashes.
- `ms bundle install` verifies blob hashes and bundle checksum before unpacking.

## Signatures

Signatures are optional in the manifest. The current implementation includes a verification hook but
no concrete verifier. When a verifier is configured, the full bundle payload is verified before install.

## Install Paths

Bundle paths must be relative. During install, they are resolved under the git archive root (e.g. `~/.local/share/ms/archive`).
Absolute paths or `..` segments are rejected.
