# Atlas

The metadata index for [leash](https://github.com/Leashify/Leash).

Atlas mirrors package metadata from npm, crates.io, PyPI, Go modules, and
GitHub releases into one uniform format. `leash` reads Atlas to resolve
dependencies. Artifacts themselves are downloaded directly from the
original registries.

## What's here

- **`config.json`** — registry endpoints and sharding rules
- **`schema/`** — JSON schemas for every file format
- **`scripts/`** — mirror jobs, one per ecosystem
- **`npm/`, `cargo/`, `pypi/`, `go/`, `github/`** — the index itself
- **`keys/`** — signing keys.
