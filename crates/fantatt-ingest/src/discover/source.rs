//! Where the walk reads its pages: the polite fetch client in production,
//! recorded fixtures in tests.

use async_trait::async_trait;

use crate::Fetched;
use crate::conditional::CacheValidators;
use crate::error::FetchError;
use crate::uri::Uri;

/// One page a source delivered.
#[derive(Debug, Clone)]
pub struct Page {
    pub bytes: Vec<u8>,
    pub validators: CacheValidators,
}

/// What a source answered for one URL.
#[derive(Debug, Clone)]
pub enum PageReply {
    /// The page body, freshly delivered.
    Fresh(Page),
    /// The page is unchanged since the validators the caller stored.
    Unchanged,
    /// The source has no such page: a 404, or a fixture that was never
    /// recorded. The walk prunes the branch instead of failing.
    Missing,
}

/// The page source of one sweep.
#[async_trait]
pub trait PageSource: Send + Sync {
    /// Read one page, honoring the validators the ledger holds for it.
    async fn page(&self, url: &Uri, validators: &CacheValidators) -> Result<PageReply, FetchError>;
}

/// The live source: the fetch facade of G1.2, which bounds, archives, and
/// conditions every request.
pub struct ClientSource<'a> {
    client: &'a crate::client::FetchClient,
}

impl<'a> ClientSource<'a> {
    pub fn new(client: &'a crate::client::FetchClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl PageSource for ClientSource<'_> {
    async fn page(&self, url: &Uri, validators: &CacheValidators) -> Result<PageReply, FetchError> {
        let target = reqwest::Url::parse(&url.as_url()).expect("a normalized URI parses as a URL");
        match self.client.fetch(&target, validators.clone()).await {
            Ok(Fetched::Fresh(page)) => Ok(PageReply::Fresh(Page {
                bytes: page.body,
                validators: page.validators,
            })),
            Ok(Fetched::NotModified) => Ok(PageReply::Unchanged),
            // why: a page the portal dropped prunes the branch; every other
            // definitive or transient failure fails the sweep
            Err(FetchError::Status {
                status: 404 | 410, ..
            }) => Ok(PageReply::Missing),
            Err(error) => Err(error),
        }
    }
}
