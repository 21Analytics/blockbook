//! # Rust Blockbook Library
//!
//! This crate provides REST and WebSocket clients to query various
//! information from a Blockbook server, which is a block explorer
//! backend [created and maintained](https://github.com/trezor/blockbook)
//! by SatoshiLabs.
//!
//! Note that this crate currently only exposes a Bitcoin-specific API,
//! even though Blockbook provides a unified API that supports
//! [multiple cryptocurrencies](https://github.com/trezor/blockbook#implemented-coins).
//!
//! The methods exposed in this crate make extensive use of types from the
//! [`bitcoin`](https://crates.io/crates/bitcoin) crate to provide strongly typed APIs.
//!
//! An example of how to use the [`REST client`]:
//!
//! ```ignore
//! # tokio_test::block_on(async {
//! # let url = format!("https://{}", std::env::var("BLOCKBOOK_SERVER").unwrap()).parse().unwrap();
//! let client = blockbook::Client::new(url).await?;
//!
//! // query the Genesis block hash
//! let genesis_hash = client
//!     .block_hash(&blockbook::Height::from_consensus(0).unwrap())
//!     .await?;
//! assert_eq!(
//!     genesis_hash.to_string(),
//!     "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
//! );
//!
//! // query the full block
//! let genesis = client.block_by_hash(&genesis_hash).await?;
//! assert_eq!(genesis.previous_block_hash, None);
//!
//! // inspect the first coinbase transaction
//! let tx = genesis.txs.get(0).unwrap();
//! assert!((tx.vout.get(0).unwrap().value.to_btc() - 50.0).abs() < f64::EPSILON);
//! # Ok::<_,blockbook::Error>(())
//! # });
//! ```
//!
//! For an example of how to use the WebSocket client, see [`its documentation`].
//!
//! ## Supported Blockbook Version
//!
//! The currently supported version of Blockbook is commit [`95ee9b5b`](https://github.com/trezor/blockbook/commit/95ee9b5b).
//!
//! [`REST client`]: Client
//! [`its documentation`]: websocket::Client

#![forbid(unsafe_code)]

mod external {
    pub use bitcoin::address::Address;
    pub use bitcoin::address::NetworkUnchecked;
    pub use bitcoin::amount::Amount;
    pub use bitcoin::bip32::DerivationPath;
    pub use bitcoin::blockdata::locktime::absolute::{Height, LockTime, Time};
    pub use bitcoin::blockdata::script::ScriptBuf;
    pub use bitcoin::blockdata::witness::Witness;
    pub use bitcoin::hash_types::{BlockHash, TxMerkleNode, Txid, Wtxid};
    pub use bitcoin::hashes;
    pub use bitcoin::Sequence;
    pub use bitcoin::Transaction as BitcoinTransaction;
    pub use reqwest::Error as ReqwestError;
    pub use url::ParseError;
}

#[doc(hidden)]
pub use external::*;

/// The WebSocket client.
pub mod websocket;

/// The errors emitted by the REST client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error during a network request.
    #[error("error occurred during the network request: {0}")]
    RequestError(#[from] reqwest::Error),
    /// An error while parsing a URL.
    #[error("invalid url: {0}")]
    UrlError(#[from] url::ParseError),
    /// The Blockbook version run on the provided server does not match the version required.
    #[error("Blockbook version {client} required but server runs {server}.")]
    VersionMismatch {
        client: semver::Version,
        server: semver::Version,
    },
}

type Result<T> = std::result::Result<T, Error>;

/// A REST client that can query a Blockbook server.
///
/// Provides a set methods that allow strongly typed
/// access to the APIs available from a Blockbook server.
///
/// See the [`module documentation`] for some concrete examples
/// of how to use call these APIs.
///
/// [`module documentation`]: crate
pub struct Client {
    base_url: url::Url,
    client: reqwest::Client,
}

impl Client {
    /// Constructs a new client for a given server `base_url`.
    ///
    /// `base_url` should not contain the `/api/v2/` path fragment.
    ///
    /// # Errors
    ///
    /// If the server status could not be retreived or the Blockbook
    /// version run on the server does not match the version required.
    pub async fn new(base_url: url::Url) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let client = Self {
            base_url,
            client: reqwest::Client::builder()
                .default_headers(headers)
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        };
        let client_version = semver::Version::new(0, 4, 0);
        let server_version = client.status().await?.blockbook.version;
        if server_version != client_version {
            return Err(Error::VersionMismatch {
                client: client_version,
                server: server_version,
            });
        }
        Ok(client)
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

    /// Queries [information](https://github.com/trezor/blockbook/blob/95eb699ccbaeef0ec6d8fd0486de3445b8405e8a/docs/api.md#status-page) about the Blockbook server status.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn status(&self) -> Result<Status> {
        self.query("/api/v2").await
    }

    /// Retrieves the [`BlockHash`] of a block of the given `height`.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn block_hash(&self, height: &Height) -> Result<BlockHash> {
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

    /// Retrieves [information](https://github.com/trezor/blockbook/blob/95eb699ccbaeef0ec6d8fd0486de3445b8405e8a/docs/api.md#get-transaction)
    /// about a transaction with a given `txid`.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn transaction(&self, txid: &Txid) -> Result<Transaction> {
        self.query(format!("/api/v2/tx/{txid}")).await
    }

    /// Retrieves [information](https://github.com/trezor/blockbook/blob/95eb699ccbaeef0ec6d8fd0486de3445b8405e8a/docs/api.md#get-transaction-specific)
    /// about a transaction with a given `txid` as reported by the Bitcoin Core backend.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn transaction_specific(&self, txid: &Txid) -> Result<TransactionSpecific> {
        self.query(format!("/api/v2/tx-specific/{txid}")).await
    }

    /// Retrieves [information](https://github.com/trezor/blockbook/blob/86ff5a9538dba6b869f53850676f9edfc3cb5fa8/docs/api.md#get-block)
    /// about a block of the specified `height`.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn block_by_height(&self, height: &Height) -> Result<Block> {
        self.query(format!("/api/v2/block/{height}")).await
    }

    /// Retrieves [information](https://github.com/trezor/blockbook/blob/86ff5a9538dba6b869f53850676f9edfc3cb5fa8/docs/api.md#get-block)
    /// about a block with the specified `hash`.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn block_by_hash(&self, hash: &BlockHash) -> Result<Block> {
        self.query(format!("/api/v2/block/{hash}")).await
    }

    /// Retrieves a list of available price tickers close to a given `timestamp`.
    ///
    /// The API will return a tickers list that is as close as possible
    /// to the specified `timestamp`.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn tickers_list(&self, timestamp: &Time) -> Result<TickersList> {
        self.query(format!("/api/v2/tickers-list/?timestamp={timestamp}"))
            .await
    }

    /// Retrieves the exchange rate for a given `currency`.
    ///
    /// The API will return a ticker that is as close as possible to the provided
    /// `timestamp`. If `timestamp` is `None`, the latest available ticker will be
    /// returned.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn ticker(&self, currency: &Currency, timestamp: Option<&Time>) -> Result<Ticker> {
        let mut query_pairs = url::form_urlencoded::Serializer::new(String::new());
        query_pairs.append_pair("currency", &format!("{currency:?}"));
        if let Some(ts) = timestamp {
            query_pairs.append_pair("timestamp", &ts.to_string());
        }
        self.query(format!("/api/v2/tickers?{}", query_pairs.finish()))
            .await
    }

    /// Retrieves the exchange rates for all available currencies.
    ///
    /// The API will return tickers that are as close as possible to the provided
    /// `timestamp`. If `timestamp` is `None`, the latest available tickers will be
    /// returned.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn tickers(&self, timestamp: Option<&Time>) -> Result<Ticker> {
        self.query(format!(
            "/api/v2/tickers/{}",
            timestamp.map_or(String::new(), |ts| format!("?timestamp={ts}"))
        ))
        .await
    }

    /// Retrieves [basic aggregated information](https://github.com/trezor/blockbook/blob/211aeff22d6f9ce59b26895883aa85905bba566b/docs/api.md#get-address)
    /// about a provided `address`.
    ///
    /// If an `also_in` [`Currency`] is specified, the total balance will also be returned in terms of that currency.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn address_info_specific_basic(
        &self,
        address: &Address,
        also_in: Option<&Currency>,
    ) -> Result<AddressInfoBasic> {
        let mut query_pairs = url::form_urlencoded::Serializer::new(String::new());
        query_pairs.append_pair("details", "basic");
        if let Some(currency) = also_in {
            query_pairs.append_pair("secondary", &format!("{currency:?}"));
        }
        self.query(format!(
            "/api/v2/address/{address}?{}",
            query_pairs.finish()
        ))
        .await
    }

    /// Retrieves [basic aggregated information](https://github.com/trezor/blockbook/blob/211aeff22d6f9ce59b26895883aa85905bba566b/docs/api.md#get-address)
    /// as well as a list of [`Txid`]s for a given `address`.
    ///
    /// The [`txids`] field of the response will be paged if the `address` was
    /// involved in many transactions. In this case, use [`address_info_specific`]
    /// to control the pagination.
    ///
    /// [`txids`]: AddressInfo::txids
    /// [`address_info_specific`]: Client::address_info_specific
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn address_info(&self, address: &Address) -> Result<AddressInfo> {
        self.query(format!("/api/v2/address/{address}")).await
    }

    /// Retrieves [basic aggregated information](https://github.com/trezor/blockbook/blob/211aeff22d6f9ce59b26895883aa85905bba566b/docs/api.md#get-address)
    /// as well as a paginated list of [`Txid`]s for a given `address`.
    ///
    /// If an `also_in` [`Currency`] is specified, the total balance will also be returned in terms of that currency.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn address_info_specific(
        &self,
        address: &Address,
        page: Option<&std::num::NonZeroU32>,
        pagesize: Option<&std::num::NonZeroU16>,
        from: Option<&Height>,
        to: Option<&Height>,
        also_in: Option<&Currency>,
    ) -> Result<AddressInfo> {
        let mut query_pairs = url::form_urlencoded::Serializer::new(String::new());
        if let Some(p) = page {
            query_pairs.append_pair("page", &p.to_string());
        }
        if let Some(ps) = pagesize {
            query_pairs.append_pair("pageSize", &ps.to_string());
        }
        if let Some(start_block) = from {
            query_pairs.append_pair("from", &start_block.to_string());
        }
        if let Some(end_block) = to {
            query_pairs.append_pair("to", &end_block.to_string());
        }
        if let Some(currency) = also_in {
            query_pairs.append_pair("secondary", &format!("{currency:?}"));
        }
        self.query(format!(
            "/api/v2/address/{address}?{}",
            query_pairs.finish()
        ))
        .await
    }

    /// Retrieves [basic aggregated information](https://github.com/trezor/blockbook/blob/211aeff22d6f9ce59b26895883aa85905bba566b/docs/api.md#get-address)
    /// as well as a paginated list of [`Tx`] objects for a given `address`.
    ///
    /// The `details` parameter specifies how much information should
    /// be returned for the transactions in question:
    /// - [`TxDetail::Light`]: A list of [`Tx::Light`] abbreviated transaction information
    /// - [`TxDetail::Full`]: A list of [`Tx::Ordinary`] detailed transaction information
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    #[allow(clippy::too_many_arguments)]
    pub async fn address_info_specific_detailed(
        &self,
        address: &Address,
        page: Option<&std::num::NonZeroU32>,
        pagesize: Option<&std::num::NonZeroU16>,
        from: Option<&Height>,
        to: Option<&Height>,
        details: &TxDetail,
        also_in: Option<&Currency>,
    ) -> Result<AddressInfo> {
        let mut query_pairs = url::form_urlencoded::Serializer::new(String::new());
        query_pairs.append_pair("details", details.as_str());
        if let Some(p) = page {
            query_pairs.append_pair("page", &p.to_string());
        }
        if let Some(ps) = pagesize {
            query_pairs.append_pair("pageSize", &ps.to_string());
        }
        if let Some(start_block) = from {
            query_pairs.append_pair("from", &start_block.to_string());
        }
        if let Some(end_block) = to {
            query_pairs.append_pair("to", &end_block.to_string());
        }
        if let Some(currency) = also_in {
            query_pairs.append_pair("secondary", &format!("{currency:?}"));
        }
        self.query(format!(
            "/api/v2/address/{address}?{}",
            query_pairs.finish()
        ))
        .await
    }

    /// Retrieves [information](https://github.com/trezor/blockbook/blob/78cf3c264782e60a147031c6ae80b3ab1f704783/docs/api.md#get-utxo)
    /// about unspent transaction outputs (UTXOs) that a given address controls.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn utxos_from_address(
        &self,
        address: &Address,
        confirmed_only: bool,
    ) -> Result<Vec<Utxo>> {
        self.query(format!("/api/v2/utxo/{address}?confirmed={confirmed_only}"))
            .await
    }

    /// Retrieves [information](https://github.com/trezor/blockbook/blob/78cf3c264782e60a147031c6ae80b3ab1f704783/docs/api.md#get-utxo)
    /// about unspent transaction outputs (UTXOs) that are controlled by addresses that
    /// can be derived from the given [`extended public key`].
    ///
    /// For details of how Blockbook attempts to derive addresses, see the
    /// [`xpub_info_basic`] documentation.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    ///
    /// [`extended public key`]: bitcoin::bip32::Xpub
    /// [`xpub_info_basic`]: Client::xpub_info_basic
    pub async fn utxos_from_xpub(&self, xpub: &str, confirmed_only: bool) -> Result<Vec<Utxo>> {
        self.query(format!("/api/v2/utxo/{xpub}?confirmed={confirmed_only}"))
            .await
    }

    /// Retrieves a paginated list of [information](https://github.com/trezor/blockbook/blob/78cf3c264782e60a147031c6ae80b3ab1f704783/docs/api.md#balance-history)
    /// about the balance history of a given `address`.
    ///
    /// If a `currency` is specified, contemporary exchange rates
    /// will be included for each balance history event.
    ///
    /// The history can be aggregated into chunks of time of a desired
    /// length by specifying a `group_by` interval in seconds.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    ///
    /// [`extended public key`]: bitcoin::bip32::Xpub
    /// [`xpub_info_basic`]: Client::xpub_info_basic
    pub async fn balance_history(
        &self,
        address: &Address,
        from: Option<&Time>,
        to: Option<&Time>,
        currency: Option<&Currency>,
        group_by: Option<u32>,
    ) -> Result<Vec<BalanceHistory>> {
        let mut query_pairs = url::form_urlencoded::Serializer::new(String::new());
        if let Some(f) = from {
            query_pairs.append_pair("from", &f.to_string());
        }
        if let Some(t) = to {
            query_pairs.append_pair("to", &t.to_string());
        }
        if let Some(t) = currency {
            query_pairs.append_pair(
                "fiatcurrency",
                serde_json::to_value(t).unwrap().as_str().unwrap(),
            );
        }
        if let Some(gb) = group_by {
            query_pairs.append_pair("groupBy", &gb.to_string());
        }
        self.query(format!(
            "/api/v2/balancehistory/{address}?{}",
            query_pairs.finish()
        ))
        .await
    }

    /// Broadcasts a transaction to the network, returning its [`Txid`].
    ///
    /// If you already have a serialized transaction, you can use this
    /// API as follows:
    /// ```no_run
    /// # tokio_test::block_on(async {
    /// # let raw_tx = vec![0_u8];
    /// # let client = blockbook::Client::new("dummy:".parse().unwrap()).await?;
    /// // Assuming you have a hex serialization of a transaction:
    /// // let raw_tx = hex::decode(raw_tx_hex).unwrap();
    /// let tx: bitcoin::Transaction = bitcoin::consensus::deserialize(&raw_tx).unwrap();
    /// client.broadcast_transaction(&tx).await?;
    /// # Ok::<_,blockbook::Error>(())
    /// # });
    /// ```
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    pub async fn broadcast_transaction(&self, tx: &BitcoinTransaction) -> Result<Txid> {
        #[derive(serde::Deserialize)]
        struct Response {
            result: Txid,
        }
        Ok(self
            .query::<Response>(format!(
                "/api/v2/sendtx/{}",
                bitcoin::consensus::encode::serialize_hex(tx)
            ))
            .await?
            .result)
    }

    /// Retrieves [information](https://github.com/trezor/blockbook/blob/211aeff22d6f9ce59b26895883aa85905bba566b/docs/api.md#get-xpub)
    /// about the funds held by addresses from public keys derivable from an [`extended public key`].
    ///
    /// See the above link for more information about how the Blockbook server will
    /// try to derive public keys and addresses from the extended public key.
    /// Briefly, the extended key is expected to be derived at `m/purpose'/coin_type'/account'`,
    /// and Blockbook will derive `change` and `index` levels below that, subject to a
    /// gap limit of unused indices.
    ///
    /// In addition to the aggregated amounts, per-address indicators can
    /// also be retrieved (Blockbook calls them `tokens`) by setting
    /// `include_token_list`. The [`AddressFilter`] enum then allows selecting
    /// the addresses holding a balance, addresses having been used, or all
    /// addresses.
    ///
    /// If an `also_in` [`Currency`] is specified, the total balance will also be returned
    /// in terms of that currency.
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    ///
    /// [`extended public key`]: bitcoin::bip32::Xpub
    pub async fn xpub_info_basic(
        &self,
        xpub: &str,
        include_token_list: bool,
        address_filter: Option<&AddressFilter>,
        also_in: Option<&Currency>,
    ) -> Result<XPubInfoBasic> {
        let mut query_pairs = url::form_urlencoded::Serializer::new(String::new());
        query_pairs.append_pair(
            "details",
            if include_token_list {
                "tokenBalances"
            } else {
                "basic"
            },
        );
        if let Some(address_property) = address_filter {
            query_pairs.append_pair("tokens", address_property.as_str());
        }
        if let Some(currency) = also_in {
            query_pairs.append_pair("secondary", &format!("{currency:?}"));
        }
        self.query(format!("/api/v2/xpub/{xpub}?{}", query_pairs.finish()))
            .await
    }

    /// Retrieves [information](https://github.com/trezor/blockbook/blob/211aeff22d6f9ce59b26895883aa85905bba566b/docs/api.md#get-xpub)
    /// about the funds held by addresses from public keys derivable from an [`extended public key`],
    /// as well as a paginated list of [`Txid`]s or [`Transaction`]s that affect addresses derivable from
    /// the extended public key.
    ///
    /// [`Txid`]s or [`Transaction`]s are included in the returned [`XPubInfo`] based
    /// on whether `entire_txs` is set to `false` or `true` respectively.
    ///
    /// For the other arguments, see the documentation of [`xpub_info_basic`].
    ///
    /// # Errors
    ///
    /// If the underlying network request fails, if the server returns a
    /// non-success response, or if the response body is of unexpected format.
    ///
    /// [`extended public key`]: bitcoin::bip32::Xpub
    /// [`xpub_info_basic`]: Client::xpub_info_basic
    #[allow(clippy::too_many_arguments)]
    pub async fn xpub_info(
        &self,
        xpub: &str,
        page: Option<&std::num::NonZeroU32>,
        pagesize: Option<&std::num::NonZeroU16>,
        from: Option<&Height>,
        to: Option<&Height>,
        entire_txs: bool,
        address_filter: Option<&AddressFilter>,
        also_in: Option<&Currency>,
    ) -> Result<XPubInfo> {
        let mut query_pairs = url::form_urlencoded::Serializer::new(String::new());
        if let Some(p) = page {
            query_pairs.append_pair("page", &p.to_string());
        }
        if let Some(ps) = pagesize {
            query_pairs.append_pair("pageSize", &ps.to_string());
        }
        if let Some(start_block) = from {
            query_pairs.append_pair("from", &start_block.to_string());
        }
        if let Some(end_block) = to {
            query_pairs.append_pair("to", &end_block.to_string());
        }
        query_pairs.append_pair("details", if entire_txs { "txs" } else { "txids" });
        if let Some(address_property) = address_filter {
            query_pairs.append_pair("tokens", address_property.as_str());
        }
        if let Some(currency) = also_in {
            query_pairs.append_pair("secondary", &format!("{currency:?}"));
        }
        self.query(format!("/api/v2/xpub/{xpub}?{}", query_pairs.finish()))
            .await
    }
}

/// Aggregated information about funds held in addresses derivable from an [`extended public key`].
///
/// [`extended public key`]: bitcoin::bip32::Xpub
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct XPubInfoBasic {
    pub address: String,
    #[serde(with = "amount")]
    pub balance: Amount,
    #[serde(with = "amount")]
    pub total_received: Amount,
    #[serde(with = "amount")]
    pub total_sent: Amount,
    #[serde(with = "amount")]
    pub unconfirmed_balance: Amount,
    pub unconfirmed_txs: u32,
    pub txs: u32,
    #[serde(rename = "addrTxCount")]
    pub used_addresses_count: usize,
    pub used_tokens: u32,
    pub secondary_value: Option<f64>,
    pub tokens: Option<Vec<Token>>,
}

/// Information about funds at a Bitcoin address derived from an [`extended public key`].
///
/// [`extended public key`]: bitcoin::bip32::Xpub
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub r#type: String,
    #[serde(rename = "name")]
    #[serde(deserialize_with = "deserialize_address")]
    pub address: Address,
    pub path: DerivationPath,
    pub transfers: u32,
    pub decimals: u8,
    #[serde(with = "amount")]
    pub balance: Amount,
    #[serde(with = "amount")]
    pub total_received: Amount,
    #[serde(with = "amount")]
    pub total_sent: Amount,
}

/// Detailed information about funds held in addresses derivable from an [`extended public key`],
///
/// [`extended public key`]: bitcoin::bip32::Xpub
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct XPubInfo {
    #[serde(flatten)]
    pub paging: AddressInfoPaging,
    #[serde(flatten)]
    pub basic: XPubInfoBasic,
    pub txids: Option<Vec<Txid>>,
    pub transactions: Option<Vec<Transaction>>,
}

/// Used to select which addresses to consider when deriving from
/// [`extended public keys`].
///
/// [`extended public keys`]: bitcoin::bip32::Xpub
pub enum AddressFilter {
    NonZero,
    Used,
    Derived,
}

impl AddressFilter {
    fn as_str(&self) -> &'static str {
        match self {
            AddressFilter::NonZero => "nonzero",
            AddressFilter::Used => "used",
            AddressFilter::Derived => "derived",
        }
    }
}

/// Used to select the level of detail for [`address info`] transactions.
///
/// [`address info`]: Client::address_info_specific_detailed
pub enum TxDetail {
    Light,
    Full,
}

impl TxDetail {
    fn as_str(&self) -> &'static str {
        match self {
            TxDetail::Light => "txslight",
            TxDetail::Full => "txs",
        }
    }
}

fn to_u32_option<'de, D>(deserializer: D) -> std::result::Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: i32 = serde::Deserialize::deserialize(deserializer)?;
    Ok(u32::try_from(value).ok())
}

/// Paging information.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressInfoPaging {
    pub page: u32,
    /// The `total_pages` is unknown and hence set to `None` when
    /// a block height filter is set and the number of transactions
    /// is higher than the `pagesize` (default: 1000).
    #[serde(deserialize_with = "to_u32_option")]
    pub total_pages: Option<u32>,
    pub items_on_page: u32,
}

/// Information about the transactional activity of an address.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressInfo {
    #[serde(flatten)]
    pub paging: AddressInfoPaging,
    #[serde(flatten)]
    pub basic: AddressInfoBasic,
    pub txids: Option<Vec<Txid>>,
    pub transactions: Option<Vec<Tx>>,
}

fn deserialize_address<'de, D>(deserializer: D) -> std::result::Result<Address, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let unchecked_address: Address<NetworkUnchecked> =
        serde::Deserialize::deserialize(deserializer)?;
    unchecked_address
        .require_network(bitcoin::Network::Bitcoin)
        .map_err(|error| {
            serde::de::Error::custom(
                if let bitcoin::address::Error::NetworkValidation { found, .. } = error {
                    format!("invalid address: network {found} is not supported")
                } else {
                    format!("unexpected error: {error}")
                },
            )
        })
}

fn deserialize_optional_address<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Address>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    struct Helper(#[serde(deserialize_with = "deserialize_address")] Address);

    let helper_option: Option<Helper> = serde::Deserialize::deserialize(deserializer)?;
    Ok(helper_option.map(|helper| helper.0))
}

/// Information about the funds moved from or to an address.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressInfoBasic {
    #[serde(deserialize_with = "deserialize_address")]
    pub address: Address,
    #[serde(with = "amount")]
    pub balance: Amount,
    #[serde(with = "amount")]
    pub total_received: Amount,
    #[serde(with = "amount")]
    pub total_sent: Amount,
    #[serde(with = "amount")]
    pub unconfirmed_balance: Amount,
    pub unconfirmed_txs: u32,
    pub txs: u32,
    pub secondary_value: Option<f64>,
}

/// The variants for the transactions contained in [`AddressInfo::transactions`].
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Tx {
    Ordinary(Transaction),
    Light(BlockTransaction),
}

/// Information about an unspent transaction outputs.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Utxo {
    pub txid: Txid,
    pub vout: u32,
    #[serde(with = "amount")]
    pub value: Amount,
    pub height: Option<Height>,
    pub confirmations: u32,
    #[serde(rename = "lockTime")]
    pub locktime: Option<Time>,
    pub coinbase: Option<bool>,
    #[serde(deserialize_with = "deserialize_optional_address")]
    #[serde(default)]
    pub address: Option<Address>,
    pub path: Option<DerivationPath>,
}

/// A timestamp and a set of exchange rates for multiple currencies.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Ticker {
    #[serde(rename = "ts")]
    pub timestamp: Time,
    pub rates: std::collections::HashMap<Currency, f64>,
}

/// Information about a block.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub page: u32,
    pub total_pages: u32,
    pub items_on_page: u32,
    pub hash: BlockHash,
    pub previous_block_hash: Option<BlockHash>,
    pub next_block_hash: Option<BlockHash>,
    pub height: Height,
    pub confirmations: u32,
    pub size: u32,
    pub time: Time,
    pub version: bitcoin::blockdata::block::Version,
    pub merkle_root: TxMerkleNode,
    pub nonce: String,
    pub bits: String,
    pub difficulty: String,
    pub tx_count: u32,
    pub txs: Vec<BlockTransaction>,
}

/// Information about a transaction.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub struct BlockTransaction {
    pub txid: Txid,
    pub vsize: u32,
    pub vin: Vec<BlockVin>,
    pub vout: Vec<BlockVout>,
    pub block_hash: BlockHash,
    pub block_height: Height,
    pub confirmations: u32,
    pub block_time: Time,
    #[serde(with = "amount")]
    pub value: Amount,
    #[serde(with = "amount")]
    pub value_in: Amount,
    #[serde(with = "amount")]
    pub fees: Amount,
}

fn deserialize_optional_address_vector<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<Address>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    struct Helper(#[serde(deserialize_with = "deserialize_address_vector")] Vec<Address>);

    let helper_option: Option<Helper> = serde::Deserialize::deserialize(deserializer)?;
    Ok(helper_option.map(|helper| helper.0))
}

/// Information about a transaction input.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockVin {
    pub n: u16,
    /// Can be `None` or multiple addresses for a non-standard script,
    /// where the latter indicates a multisig input
    #[serde(deserialize_with = "deserialize_optional_address_vector")]
    #[serde(default)]
    pub addresses: Option<Vec<Address>>,
    /// Indicates a [standard script](https://github.com/trezor/blockbook/blob/0ebbf16f18551f1c73b59bec6cfcbbdc96ec47e8/bchain/coins/btc/bitcoinlikeparser.go#L193-L194)
    pub is_address: bool,
    #[serde(with = "amount")]
    pub value: Amount,
}

/// Information about a transaction output.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockVout {
    #[serde(with = "amount")]
    pub value: Amount,
    pub n: u16,
    pub spent: Option<bool>,
    pub addresses: Vec<AddressBlockVout>,
    /// Indicates the `addresses` vector to contain the `Address` `AddressBlockVout` variant
    pub is_address: bool,
}

/// Either an address or an [`OP_RETURN output`].
///
/// [`OP_RETURN output`]: OpReturn
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum AddressBlockVout {
    #[serde(deserialize_with = "deserialize_address")]
    Address(Address),
    OpReturn(OpReturn),
}

/// An `OP_RETURN` output.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OpReturn(pub String);

/// A cryptocurrency asset.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub enum Asset {
    Bitcoin,
}

/// Status and backend information of the Blockbook server.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Status {
    pub blockbook: StatusBlockbook,
    pub backend: Backend,
}

/// Status information of the Blockbook server.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
#[allow(clippy::struct_excessive_bools)]
pub struct StatusBlockbook {
    pub coin: Asset,
    pub host: String,
    pub version: semver::Version,
    pub git_commit: String,
    pub build_time: chrono::DateTime<chrono::Utc>,
    pub sync_mode: bool,
    #[serde(rename = "initialSync")]
    pub is_initial_sync: bool,
    #[serde(rename = "inSync")]
    pub is_in_sync: bool,
    pub best_height: crate::Height,
    pub last_block_time: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "inSyncMempool")]
    pub is_in_sync_mempool: bool,
    pub last_mempool_time: chrono::DateTime<chrono::Utc>,
    pub mempool_size: u32,
    pub decimals: u8,
    pub db_size: u64,
    pub about: String,
    pub has_fiat_rates: bool,
    pub current_fiat_rates_time: chrono::DateTime<chrono::Utc>,
    pub historical_fiat_rates_time: chrono::DateTime<chrono::Utc>,
}

/// The specific chain (mainnet, testnet, ...).
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub enum Chain {
    #[serde(rename = "main")]
    Main,
}

/// Information about the full node backing the Blockbook server.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Backend {
    pub chain: Chain,
    pub blocks: crate::Height,
    pub headers: u32,
    pub best_block_hash: crate::BlockHash,
    pub difficulty: String,
    pub size_on_disk: u64,
    #[serde(flatten)]
    pub version: Version,
    pub protocol_version: String,
}

/// Version information about the full node.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Version {
    pub version: String,
    pub subversion: String,
}

mod amount {
    struct AmountVisitor;

    impl<'de> serde::de::Visitor<'de> for AmountVisitor {
        type Value = super::Amount;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid Bitcoin amount")
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if let Ok(amount) = super::Amount::from_btc(value) {
                Ok(amount)
            } else {
                Err(E::custom("invalid Bitcoin amount"))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if let Ok(amount) =
                super::Amount::from_str_in(value, bitcoin::amount::Denomination::Satoshi)
            {
                Ok(amount)
            } else {
                Err(E::custom("invalid Bitcoin amount"))
            }
        }
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<super::Amount, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(AmountVisitor)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(super) fn serialize<S>(amount: &super::Amount, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&amount.to_sat().to_string())
    }
}

/// Information about the available exchange rates at a given timestamp.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct TickersList {
    #[serde(rename = "ts")]
    pub timestamp: Time,
    pub available_currencies: Vec<Currency>,
}

/// The supported currencies.
#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Currency {
    Aed,
    Ars,
    Aud,
    Bch,
    Bdt,
    Bhd,
    Bits,
    Bmd,
    Bnb,
    Brl,
    Btc,
    Cad,
    Chf,
    Clp,
    Cny,
    Czk,
    Dkk,
    Dot,
    Eos,
    Eth,
    Eur,
    Gbp,
    Hkd,
    Huf,
    Idr,
    Ils,
    Inr,
    Jpy,
    Krw,
    Kwd,
    Link,
    Lkr,
    Ltc,
    Mmk,
    Mxn,
    Myr,
    Ngn,
    Nok,
    Nzd,
    Php,
    Pkr,
    Pln,
    Rub,
    Sar,
    Sats,
    Sek,
    Sgd,
    Thb,
    Try,
    Twd,
    Uah,
    Usd,
    Vef,
    Vnd,
    Xag,
    Xau,
    Xdr,
    Xlm,
    Xrp,
    Yfi,
    Zar,
}

fn maybe_block_height<'de, D>(deserializer: D) -> std::result::Result<Option<Height>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(to_u32_option(deserializer)?.and_then(|h| Height::from_consensus(h).ok()))
}

/// Information about a transaction.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Transaction {
    pub txid: Txid,
    pub version: bitcoin::blockdata::transaction::Version,
    pub lock_time: Option<Height>,
    pub vin: Vec<Vin>,
    pub vout: Vec<Vout>,
    pub size: u32,
    pub vsize: u32,
    /// `None` for unconfirmed transactions
    pub block_hash: Option<BlockHash>,
    /// `None` for unconfirmed transactions
    #[serde(deserialize_with = "maybe_block_height")]
    pub block_height: Option<Height>,
    pub confirmations: u32,
    pub block_time: Time,
    #[serde(with = "amount")]
    pub value: Amount,
    #[serde(with = "amount")]
    pub value_in: Amount,
    #[serde(with = "amount")]
    pub fees: Amount,
    #[serde(rename = "hex")]
    pub script: ScriptBuf,
}

fn deserialize_address_vector<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<Address>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    struct Helper(#[serde(deserialize_with = "deserialize_address")] Address);

    let helper_vector: Vec<Helper> = serde::Deserialize::deserialize(deserializer)?;
    Ok(helper_vector.into_iter().map(|helper| helper.0).collect())
}

/// Information about a transaction input.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vin {
    pub txid: Txid,
    pub vout: Option<u16>,
    pub sequence: Option<Sequence>,
    pub n: u16,
    #[serde(deserialize_with = "deserialize_address_vector")]
    pub addresses: Vec<Address>,
    pub is_address: bool,
    #[serde(with = "amount")]
    pub value: Amount,
}

/// Information about a transaction output.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Vout {
    #[serde(with = "amount")]
    pub value: Amount,
    pub n: u16,
    pub spent: Option<bool>,
    pub spent_tx_id: Option<Txid>,
    pub spent_height: Option<Height>,
    pub spent_index: Option<u16>,
    #[serde(rename = "hex")]
    pub script: ScriptBuf,
    #[serde(deserialize_with = "deserialize_address_vector")]
    pub addresses: Vec<Address>,
    pub is_address: bool,
    /// only present in an xpub context
    pub is_own: Option<bool>,
}

/// Detailed information about a transaction input.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct TransactionSpecific {
    pub txid: Txid,
    pub version: bitcoin::blockdata::transaction::Version,
    pub vin: Vec<VinSpecific>,
    pub vout: Vec<VoutSpecific>,
    pub blockhash: Option<BlockHash>,
    pub blocktime: Option<Time>,
    #[serde(rename = "hash")]
    pub wtxid: Wtxid,
    pub confirmations: Option<u32>,
    pub locktime: LockTime,
    #[serde(rename = "hex")]
    pub script: ScriptBuf,
    pub size: u32,
    pub time: Option<Time>,
    pub vsize: u32,
    pub weight: u32,
}

impl From<TransactionSpecific> for BitcoinTransaction {
    fn from(tx: TransactionSpecific) -> Self {
        BitcoinTransaction {
            version: tx.version,
            lock_time: tx.locktime,
            input: tx.vin.into_iter().map(Into::into).collect(),
            output: tx.vout.into_iter().map(Into::into).collect(),
        }
    }
}

/// Bitcoin-specific information about a transaction input.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct VinSpecific {
    pub sequence: Sequence,
    pub txid: Txid,
    #[serde(rename = "txinwitness")]
    pub tx_in_witness: Option<Witness>,
    #[serde(rename = "scriptSig")]
    pub script_sig: ScriptSig,
    pub vout: u32,
}

impl From<VinSpecific> for bitcoin::TxIn {
    fn from(vin: VinSpecific) -> Self {
        Self {
            previous_output: bitcoin::transaction::OutPoint {
                txid: vin.txid,
                vout: vin.vout,
            },
            script_sig: vin.script_sig.script,
            sequence: vin.sequence,
            witness: vin.tx_in_witness.unwrap_or_default(),
        }
    }
}

/// A script fulfilling spending conditions.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct ScriptSig {
    pub asm: String,
    #[serde(rename = "hex")]
    pub script: ScriptBuf,
}

/// Bitcoin-specific information about a transaction output.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct VoutSpecific {
    pub n: u32,
    pub script_pub_key: ScriptPubKey,
    #[serde(with = "amount")]
    pub value: Amount,
}

impl From<VoutSpecific> for bitcoin::TxOut {
    fn from(vout: VoutSpecific) -> Self {
        Self {
            value: vout.value,
            script_pubkey: vout.script_pub_key.script,
        }
    }
}

/// A script specifying spending conditions.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct ScriptPubKey {
    #[serde(deserialize_with = "deserialize_address")]
    pub address: Address,
    pub asm: String,
    pub desc: Option<String>,
    #[serde(rename = "hex")]
    pub script: ScriptBuf,
    pub r#type: ScriptPubKeyType,
}

/// The type of spending condition.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
#[non_exhaustive]
pub enum ScriptPubKeyType {
    NonStandard,
    PubKey,
    PubKeyHash,
    #[serde(rename = "witness_v0_keyhash")]
    WitnessV0PubKeyHash,
    ScriptHash,
    #[serde(rename = "witness_v0_scripthash")]
    WitnessV0ScriptHash,
    MultiSig,
    NullData,
    #[serde(rename = "witness_v1_taproot")]
    WitnessV1Taproot,
    #[serde(rename = "witness_unknown")]
    WitnessUnknown,
}

/// A balance history entry.
#[derive(Debug, PartialEq, serde::Deserialize)]
pub struct BalanceHistory {
    pub time: Time,
    pub txs: u32,
    #[serde(with = "amount")]
    pub received: Amount,
    #[serde(with = "amount")]
    pub sent: Amount,
    #[serde(rename = "sentToSelf")]
    #[serde(with = "amount")]
    pub sent_to_self: Amount,
    pub rates: std::collections::HashMap<Currency, f64>,
}

#[cfg(test)]
mod test {
    #[test]
    fn serde_amounts() {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct TestStruct {
            #[serde(with = "super::amount")]
            pub amount: super::Amount,
        }

        serde_test::assert_tokens(
            &TestStruct {
                amount: super::Amount::from_sat(123_456_789),
            },
            &[
                serde_test::Token::Struct {
                    name: "TestStruct",
                    len: 1,
                },
                serde_test::Token::Str("amount"),
                serde_test::Token::Str("123456789"),
                serde_test::Token::StructEnd,
            ],
        );
    }
}
