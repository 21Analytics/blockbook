pub use bitcoin::hash_types::BlockHash;
pub use reqwest::Error as ReqwestError;
pub use url::ParseError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    RequestError(reqwest::Error),
    UrlError(url::ParseError),
}

type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::RequestError(e) => e.fmt(f),
            Error::UrlError(e) => e.fmt(f),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::RequestError(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::UrlError(e)
    }
}

pub struct Blockbook {
    base_url: url::Url,
    client: reqwest::Client,
}

impl Blockbook {
    pub fn new(base_url: url::Url) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:107.0) Gecko/20100101 Firefox/107.0")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        Self { base_url, client }
    }

    fn url(&self, endpoint: impl AsRef<str>) -> Result<url::Url> {
        Ok(self.base_url.join(endpoint.as_ref())?)
    }

    async fn query<T: serde::de::DeserializeOwned>(&self, path: impl AsRef<str>) -> Result<T> {
        Ok(self
            .client
            .get(self.url(path.as_ref())?)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    // https://github.com/trezor/blockbook/blob/95eb699ccbaeef0ec6d8fd0486de3445b8405e8a/docs/api.md#get-block-hash
    pub async fn block_hash(&self, height: u32) -> Result<BlockHash> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct BlockHashObject {
            block_hash: BlockHash,
        }
        Ok(self
            .query::<BlockHashObject>(format!("/api/v2/block-index/{height}"))
            .await?
            .block_hash)
    }
}
