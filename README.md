# Atlas

The package index for [leash](https://github.com/Leashify/Leash).

Atlas contains the metadata needed for `leash` to download packages and
resolve their dependencies across npm, crates.io, PyPI, Go modules, and
GitHub releases.

The index is available at:

- `https://leashify.github.io/Atlas/` as a set of static files
- `https://github.com/Leashify/Atlas` as a git repository

Each ecosystem has its own directory:

- `npm/` — mirrored from the npm registry
- `cargo/` — mirrored from crates.io
- `pypi/` — mirrored from PyPI
- `go/` — mirrored from the Go module proxy
- `github/` — mirrored from GitHub releases

Every package is one file. Every version is one line of JSON.

## License

MIT
