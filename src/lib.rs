pub use bitcoin::blockdata::locktime::{Height, PackedLockTime, Time};
pub use bitcoin::blockdata::script::Script;
pub use bitcoin::blockdata::witness::Witness;
pub use bitcoin::hash_types::{BlockHash, TxMerkleNode, Txid, Wtxid};
pub use bitcoin::hashes;
pub use bitcoin::util::address::Address;
pub use bitcoin::util::amount::Amount;
pub use bitcoin::Sequence;
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

    // https://github.com/trezor/blockbook/blob/95eb699ccbaeef0ec6d8fd0486de3445b8405e8a/docs/api.md#status-page
    pub async fn status(&self) -> Result<Status> {
        self.query::<Status>("/api/v2").await
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

    // https://github.com/trezor/blockbook/blob/95eb699ccbaeef0ec6d8fd0486de3445b8405e8a/docs/api.md#get-transaction
    pub async fn transaction(&self, txid: impl AsRef<str>) -> Result<Transaction> {
        self.query(format!("/api/v2/tx/{}", txid.as_ref())).await
    }

    // https://github.com/trezor/blockbook/blob/95eb699ccbaeef0ec6d8fd0486de3445b8405e8a/docs/api.md#get-transaction-specific
    pub async fn transaction_specific(&self, txid: impl AsRef<str>) -> Result<TransactionSpecific> {
        self.query(format!("/api/v2/tx-specific/{}", txid.as_ref()))
            .await
    }

    // https://github.com/trezor/blockbook/blob/86ff5a9538dba6b869f53850676f9edfc3cb5fa8/docs/api.md#get-block
    pub async fn block_by_height(&self, height: Height) -> Result<Block> {
        self.query::<Block>(format!("/api/v2/block/{height}")).await
    }

    // https://github.com/trezor/blockbook/blob/86ff5a9538dba6b869f53850676f9edfc3cb5fa8/docs/api.md#get-block
    pub async fn block_by_hash(&self, hash: BlockHash) -> Result<Block> {
        self.query::<Block>(format!("/api/v2/block/{hash}")).await
    }
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
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
    pub version: u32,
    pub merkle_root: TxMerkleNode,
    pub nonce: String,
    pub bits: String,
    pub difficulty: String,
    pub tx_count: u32,
    pub txs: Vec<BlockTransaction>,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub struct BlockTransaction {
    pub txid: Txid,
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

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub struct BlockVin {
    pub n: u16,
    /// Can be `None` or multiple addresses for a non-standard script,
    /// where the latter indicates a multisig input
    pub addresses: Option<Vec<Address>>,
    /// Indicates a standard script, See:
    /// https://github.com/trezor/blockbook/blob/0ebbf16f18551f1c73b59bec6cfcbbdc96ec47e8/bchain/coins/btc/bitcoinlikeparser.go#L193-L194
    pub is_address: bool,
    #[serde(with = "amount")]
    pub value: Amount,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
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

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum AddressBlockVout {
    Address(Address),
    OpReturn(OpReturn),
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OpReturn(pub String);

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub enum Asset {
    Bitcoin,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Status {
    pub blockbook: StatusBlockbook,
    pub backend: Backend,
}

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

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub enum Chain {
    #[serde(rename = "main")]
    Main,
}

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
    pub version: String,
    pub subversion: String,
    pub protocol_version: String,
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
                super::Amount::from_str_in(value, bitcoin::util::amount::Denomination::Satoshi)
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Transaction {
    pub txid: Txid,
    pub version: u8,
    pub vin: Vec<Vin>,
    pub vout: Vec<Vout>,
    pub size: u32,
    pub vsize: u32,
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
    #[serde(rename = "hex")]
    pub script: Script,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Vin {
    pub txid: Txid,
    pub vout: Option<u16>,
    pub sequence: Sequence,
    pub n: u16,
    pub addresses: Vec<Address>,
    pub is_address: bool,
    #[serde(with = "amount")]
    pub value: Amount,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Vout {
    #[serde(with = "amount")]
    pub value: Amount,
    pub n: u16,
    pub spent: Option<bool>,
    #[serde(rename = "hex")]
    pub script: Script,
    pub addresses: Vec<Address>,
    pub is_address: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct TransactionSpecific {
    pub txid: Txid,
    pub version: u8,
    pub vin: Vec<VinSpecific>,
    pub vout: Vec<VoutSpecific>,
    pub blockhash: BlockHash,
    pub blocktime: Time,
    #[serde(rename = "hash")]
    pub wtxid: Wtxid,
    pub confirmations: u32,
    pub locktime: PackedLockTime,
    #[serde(rename = "hex")]
    pub script: Script,
    pub size: u32,
    pub time: Time,
    pub vsize: u32,
    pub weight: u32,
}

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

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct ScriptSig {
    pub asm: String,
    #[serde(rename = "hex")]
    pub script: Script,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct VoutSpecific {
    pub n: u32,
    pub script_pub_key: ScriptPubKey,
    #[serde(with = "amount")]
    pub value: Amount,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct ScriptPubKey {
    pub address: Address,
    pub asm: String,
    pub desc: Option<String>,
    #[serde(rename = "hex")]
    pub script: Script,
    pub r#type: ScriptPubKeyType,
}

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
