/// Index feed that lists newly published module versions.
pub const INDEX_URL: &str = "https://index.golang.org/index";

/// HTTP proxy that serves module metadata and archives.
pub const PROXY_URL: &str = "https://proxy.golang.org";

/// Checksum database. Authoritative source of module hashes.
pub const SUMDB_URL: &str = "https://sum.golang.org";

/// Maximum events fetched per index page.
pub const PAGE_LIMIT: usize = 2000;

/// Concurrent module mirrors.
pub const CONCURRENCY: usize = 8;

/// Cursor file. Holds the timestamp of the last processed event.
pub const CURSOR_FILE: &str = ".go-cursor";

/// User agent sent on every request.
pub const USER_AGENT: &str = "atlas-mirror-go/0.1.0 (https://github.com/Leashify/Atlas)";
