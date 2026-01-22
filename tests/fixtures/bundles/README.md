# Bundle Test Fixtures

This directory contains pre-generated `.msb` bundle files for testing bundle parsing and installation without HTTP dependencies.

## Fixture Files

- `minimal.msb` - A single minimal skill bundle
- `multi_skill.msb` - Bundle with multiple skills
- `invalid_header.msb` - Bundle with corrupted header (for error testing)
- `invalid_checksum.msb` - Bundle with wrong checksum (for verification testing)

## Regenerating Fixtures

These fixtures are generated programmatically. To regenerate:

```bash
cargo test --test integration bundle_fixtures_generator -- --ignored --nocapture
```

## Format

Bundles follow the MSBUNDLE1 format:
- Header: `MSBUNDLE1\0`
- Manifest length (u64 BE)
- Manifest TOML bytes
- Blob count (u64 BE)
- For each blob: hash length, hash bytes, data length, data bytes

See `src/bundler/package.rs` for the full specification.
