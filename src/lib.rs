pub use reqwest::Error as RewestError;
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

#[allow(unused)]
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

    #[allow(unused)]
    fn url(&self, endpoint: impl AsRef<str>) -> Result<url::Url> {
        Ok(self.base_url.join(endpoint.as_ref())?)
    }
}
