/// Trait for opening URLs.
pub(super) trait UrlOpener: Send + Sync {
    fn open(&self, url: &str) -> anyhow::Result<()>;
}

/// System URL opener implementation.
pub(super) struct SystemUrlOpener;

impl UrlOpener for SystemUrlOpener {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        open::that(url).map_err(anyhow::Error::from)
    }
}
