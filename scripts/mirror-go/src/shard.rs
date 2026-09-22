use std::path::PathBuf;

/// Go module paths are preserved in full.
///
///   github.com/gin-gonic/gin -> go/github.com/gin-gonic/gin
///   golang.org/x/net         -> go/golang.org/x/net
///   gopkg.in/yaml.v3         -> go/gopkg.in/yaml.v3
pub fn shard_path(path: &str) -> PathBuf {
    PathBuf::from(format!("go/{}", path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_full_path() {
        assert_eq!(
            shard_path("github.com/gin-gonic/gin"),
            PathBuf::from("go/github.com/gin-gonic/gin")
        );
    }

    #[test]
    fn preserves_short_path() {
        assert_eq!(
            shard_path("gopkg.in/yaml.v3"),
            PathBuf::from("go/gopkg.in/yaml.v3")
        );
    }
}
