pub use serde_json::Error as SerdeJsonError;
pub use tokio_tungstenite::tungstenite::Error as TungsteniteError;

use futures::{SinkExt, StreamExt};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    TungsteniteError(tokio_tungstenite::tungstenite::Error),
    DataObjectMismatch,
    SubscriptionFailed,
    WebsocketClosed,
    WebsocketError(std::sync::Arc<tokio_tungstenite::tungstenite::Error>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::TungsteniteError(e) => e.fmt(f),
            Self::DataObjectMismatch => {
                write!(
                    f,
                    "the json-rpc data field doesn't match the object expected based on its id"
                )
            }
            Self::SubscriptionFailed => {
                write!(f, "the server did not establish your subscription")
            }
            Self::WebsocketClosed => {
                write!(
                    f,
                    "the websocket connection got closed; reinstantiate the client to reconnect"
                )
            }
            Self::WebsocketError(e) => {
                write!(
                    f,
                    "the websocket connection experienced a fatal error: {e:?}\nreinstantiate the client to reconnect"
                )
            }
        }
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::TungsteniteError(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Info {
    pub name: String,
    pub shortcut: String,
    pub decimals: u8,
    pub version: semver::Version,
    pub best_height: super::Height,
    pub best_hash: super::BlockHash,
    pub block_0_hash: super::BlockHash,
    pub testnet: bool,
    pub backend: Backend,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Backend {
    pub version: String,
    pub subversion: String,
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
pub struct Block {
    pub height: super::Height,
    pub hash: super::BlockHash,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
enum StreamingResponse {
    Block(Block),
    FiatRates {
        rates: std::collections::HashMap<super::Currency, f64>,
    },
    Address {
        #[serde(deserialize_with = "super::deserialize_address")]
        address: super::Address,
        tx: super::Transaction,
    },
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
enum OneOffResponse {
    Info(Info),
    TransactionSpecific(super::TransactionSpecific),
    BlockHash { hash: super::BlockHash },
    CurrentFiatRates(FiatRates),
    AvailableCurrencies(super::TickersList),
    FiatRatesAtTimestamps { tickers: Vec<FiatRates> },
    AddressInfoTxs(super::AddressInfoDetailed),
    AddressInfoTxIds(super::AddressInfo),
    AddressInfoBasic(super::AddressInfoBasic),
    UtxosFromAddress(Vec<super::Utxo>),
    BalanceHistory(Vec<super::BalanceHistory>),
    Transaction(super::Transaction),
    SendTransaction { result: super::Txid },
    EstimateTxFee(Vec<EstimateTxFee>),
    EstimateFee(Vec<EstimateFee>),
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
enum Response {
    OneOff(OneOffResponse),
    Streaming(StreamingResponse),
    SubscriptionAck { subscribed: bool },
}

#[derive(serde::Serialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
struct JsonRpcRequest<'a, T: erased_serde::Serialize + ?Sized + Send + Sync> {
    id: &'a uuid::Uuid,
    method: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<&'a T>,
}

#[derive(serde::Deserialize)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
struct JsonRpcResponse {
    id: uuid::Uuid,
    data: Response,
}

struct Job {
    method: &'static str,
    params: Option<Box<dyn erased_serde::Serialize + Send + Sync>>,
    response_channel: futures::channel::mpsc::Sender<Result<Response>>,
}

pub struct Client {
    jobs: futures::channel::mpsc::Sender<Job>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Drop for Client {
    fn drop(&mut self) {
        if self.shutdown.take().unwrap().send(()).is_err() {
            tracing::info!("processing queue already exited");
        }
    }
}

impl Client {
    pub async fn new(url: url::Url) -> Result<Self> {
        let stream = tokio_tungstenite::connect_async(url).await?;
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let (job_tx, job_rx) = futures::channel::mpsc::channel(10);
        tokio::spawn(Self::process(stream.0, job_rx, shutdown_rx));
        Ok(Self {
            jobs: job_tx,
            shutdown: Some(shutdown_tx),
        })
    }

    #[allow(clippy::too_many_lines)]
    async fn process(
        stream: tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        mut jobs: futures::channel::mpsc::Receiver<Job>,
        mut shutdown: tokio::sync::oneshot::Receiver<()>,
    ) {
        let (mut outgoing, mut incoming) = stream.split();
        let mut response_channels = std::collections::HashMap::<
            uuid::Uuid,
            futures::channel::mpsc::Sender<Result<Response>>,
        >::new();
        let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(10));
        // skip first immediate tick
        ping_interval.tick().await;
        loop {
            tokio::select! {
                _ = &mut shutdown => return,
                msg = incoming.next() => {
                    match msg {
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Pong(_))) => {
                            continue;
                        }
                        Some(Ok(msg)) => {
                            match serde_json::from_slice::<JsonRpcResponse>(&msg.into_data()) {
                                Ok(JsonRpcResponse { id, data: Response::SubscriptionAck{ subscribed } }) => {
                                    if subscribed {
                                        continue;
                                    }
                                    let Some(mut response_channel) = response_channels.remove(&id) else {
                                        tracing::error!("couldn't find requester for received msg with id {id}");
                                        continue;
                                    };
                                    response_channel.send(Err(Error::SubscriptionFailed)).await.unwrap();
                                }
                                Ok(JsonRpcResponse { id, data: data@Response::Streaming(_) }) => {
                                    let Some(response_channel) = response_channels.get_mut(&id) else {
                                        tracing::error!("couldn't find requester for received msg with id {id}");
                                        continue;
                                    };
                                    response_channel.send(Ok(data)).await.unwrap();
                                },
                                Ok(JsonRpcResponse { id, data: data@Response::OneOff(_) }) => {
                                    let Some(mut response_channel) = response_channels.remove(&id) else {
                                        tracing::error!("couldn't find requester for received msg with id {id}");
                                        continue;
                                    };
                                    response_channel.send(Ok(data)).await.unwrap();
                                },
                                Err(e) => {
                                    tracing::error!("received unexpected message: {e:?}");
                                },
                            }
                        },
                        None => {
                            response_channels
                                .iter_mut()
                                .map(|(_,ch)| {
                                    ch.send(Err(Error::WebsocketClosed))
                                })
                                .collect::<futures::stream::FuturesUnordered<_>>()
                                .collect::<Vec<std::result::Result<(), _>>>()
                                .await
                                .into_iter()
                                .collect::<std::result::Result<Vec<()>,_>>()
                                .unwrap();
                            return;
                        }
                        Some(Err(e)) => {
                            let err = std::sync::Arc::new(e);
                            response_channels
                                .iter_mut()
                                .map(|(_,ch)| {
                                    ch.send(Err(Error::WebsocketError(err.clone())))
                                })
                                .collect::<futures::stream::FuturesUnordered<_>>()
                                .collect::<Vec<std::result::Result<_, _>>>()
                                .await
                                .into_iter()
                                .collect::<std::result::Result<Vec<_>,_>>()
                                .unwrap();
                            return;
                        }
                    }
                },
                job = jobs.next() => {
                    let Job { method, params, response_channel } = job.unwrap();
                    let request_id = uuid::Uuid::new_v4();
                    if let Err(e) = outgoing
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            serde_json::to_string(&JsonRpcRequest {
                                id: &request_id,
                                method,
                                params: params.as_deref(),
                            }).unwrap()),
                        ).await {
                            tracing::error!("failed sending message on websocket: {e:?}");
                        };
                        response_channels.insert(request_id, response_channel);
                    },
                _ = ping_interval.tick() => {
                    if let Err(e) = outgoing
                        .send(tokio_tungstenite::tungstenite::Message::Ping(vec![]))
                        .await
                    {
                        tracing::error!("failed sending ping: {e:?}");
                    }
                }
            }
        }
    }

    pub async fn get_info(&mut self) -> Result<Info> {
        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getInfo",
                params: None,
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::Info(i)) = resp {
                return Ok(i);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn get_blockhash(&mut self, height: super::Height) -> Result<super::BlockHash> {
        #[derive(serde::Serialize)]
        struct Params {
            height: super::Height,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getBlockHash",
                params: Some(Box::new(Params { height })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::BlockHash { hash }) = resp {
                return Ok(hash);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn get_current_fiat_rates(
        &mut self,
        currencies: Vec<super::Currency>,
    ) -> Result<FiatRates> {
        #[derive(serde::Serialize)]
        struct Params {
            currencies: Vec<super::Currency>,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getCurrentFiatRates",
                params: Some(Box::new(Params { currencies })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::CurrentFiatRates(rates)) = resp {
                return Ok(rates);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    /// Uses the provided timestamp and returns the closest available
    /// timestamp and a list of available currencies at that timestamp.
    pub async fn get_available_currencies(
        &mut self,
        time: super::Time,
    ) -> Result<super::TickersList> {
        #[derive(serde::Serialize)]
        struct Params {
            time: u32,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getFiatRatesTickersList",
                params: Some(Box::new(Params {
                    time: time.to_consensus_u32(),
                })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::AvailableCurrencies(currencies)) = resp {
                return Ok(currencies);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn fiat_rates_for_timestamps(
        &mut self,
        timestamps: Vec<super::Time>,
        currencies: Option<Vec<super::Currency>>,
    ) -> Result<Vec<FiatRates>> {
        #[derive(serde::Serialize)]
        struct Params {
            timestamps: Vec<super::Time>,
            currencies: Option<Vec<super::Currency>>,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getFiatRatesForTimestamps",
                params: Some(Box::new(Params {
                    timestamps,
                    currencies,
                })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::FiatRatesAtTimestamps { tickers }) = resp {
                return Ok(tickers);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn address_info_basic(
        &mut self,
        address: super::Address,
        also_in: Option<super::Currency>,
    ) -> Result<super::AddressInfoBasic> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            descriptor: super::Address,
            secondary_currency: Option<super::Currency>,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getAccountInfo",
                params: Some(Box::new(Params {
                    descriptor: address,
                    secondary_currency: also_in,
                })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::AddressInfoBasic(info)) = resp {
                return Ok(info);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn address_info_txids(
        &mut self,
        address: super::Address,
        page: Option<std::num::NonZeroU32>,
        pagesize: Option<std::num::NonZeroU16>,
        from: Option<super::Height>,
        to: Option<super::Height>,
        also_in: Option<super::Currency>,
    ) -> Result<super::AddressInfo> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            descriptor: super::Address,
            details: String,
            page: Option<std::num::NonZeroU32>,
            page_size: Option<std::num::NonZeroU16>,
            from: Option<super::Height>,
            to: Option<super::Height>,
            secondary_currency: Option<super::Currency>,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getAccountInfo",
                params: Some(Box::new(Params {
                    descriptor: address,
                    details: "txids".into(),
                    page,
                    page_size: pagesize,
                    from,
                    to,
                    secondary_currency: also_in,
                })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::AddressInfoTxIds(info)) = resp {
                return Ok(info);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn address_info_txs(
        &mut self,
        address: super::Address,
        page: Option<std::num::NonZeroU32>,
        pagesize: Option<std::num::NonZeroU16>,
        from: Option<super::Height>,
        to: Option<super::Height>,
        also_in: Option<super::Currency>,
    ) -> Result<super::AddressInfoDetailed> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            descriptor: super::Address,
            details: String,
            page: Option<std::num::NonZeroU32>,
            page_size: Option<std::num::NonZeroU16>,
            from: Option<super::Height>,
            to: Option<super::Height>,
            secondary_currency: Option<super::Currency>,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getAccountInfo",
                params: Some(Box::new(Params {
                    descriptor: address,
                    details: "txs".into(),
                    page,
                    page_size: pagesize,
                    from,
                    to,
                    secondary_currency: also_in,
                })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::AddressInfoTxs(info)) = resp {
                return Ok(info);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn transaction(&mut self, txid: super::Txid) -> Result<super::Transaction> {
        #[derive(serde::Serialize)]
        struct Params {
            txid: super::Txid,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getTransaction",
                params: Some(Box::new(Params { txid })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::Transaction(tx)) = resp {
                return Ok(tx);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn transaction_specific(
        &mut self,
        txid: super::Txid,
    ) -> Result<super::TransactionSpecific> {
        #[derive(serde::Serialize)]
        struct Params {
            txid: super::Txid,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getTransactionSpecific",
                params: Some(Box::new(Params { txid })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::TransactionSpecific(tx)) = resp {
                return Ok(tx);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    /// Returns the estimated fee for a set of target blocks
    /// to wait. The returned unit is bitcoin per vByte.
    pub async fn estimate_fee(&mut self, blocks: Vec<u16>) -> Result<Vec<super::Amount>> {
        #[derive(serde::Serialize)]
        struct Params {
            blocks: Vec<u16>,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "estimateFee",
                params: Some(Box::new(Params { blocks })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::EstimateFee(fees)) = resp {
                // for unkonwon reasons, Blockbook returns fees in satoshis/vKB
                return Ok(fees.into_iter().map(|f| f.fee_per_unit / 1000).collect());
            }
            Err(Error::DataObjectMismatch)
        })
    }

    /// Returns the estimated total fee for a transaction of
    /// the given size in bytes for a set of target blocks
    /// to wait.
    pub async fn estimate_tx_fee(
        &mut self,
        blocks: Vec<u16>,
        tx_size: u32,
    ) -> Result<Vec<super::Amount>> {
        #[derive(serde::Serialize)]
        struct Params {
            blocks: Vec<u16>,
            specific: Specific,
        }
        #[derive(serde::Serialize)]
        struct Specific {
            #[serde(rename = "txsize")]
            tx_size: u32,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "estimateFee",
                params: Some(Box::new(Params {
                    blocks,
                    specific: Specific { tx_size },
                })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::EstimateTxFee(fees)) = resp {
                return Ok(fees.into_iter().map(|f| f.fee_per_tx).collect());
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn send_transaction(
        &mut self,
        transaction: &super::BitcoinTransaction,
    ) -> Result<super::Txid> {
        #[derive(serde::Serialize)]
        struct Params {
            hex: String,
        }
        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "sendTransaction",
                params: Some(Box::new(Params {
                    hex: bitcoin::consensus::encode::serialize_hex(transaction),
                })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::SendTransaction { result }) = resp {
                return Ok(result);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn get_utxos_from_address(
        &mut self,
        address: super::Address,
    ) -> Result<Vec<super::Utxo>> {
        #[derive(serde::Serialize)]
        struct Params {
            descriptor: super::Address,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getAccountUtxo",
                params: Some(Box::new(Params {
                    descriptor: address,
                })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::UtxosFromAddress(utxos)) = resp {
                return Ok(utxos);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    /// The `group_by` parameter sets the interval length (in seconds)
    /// over which transactions are consolidated into [`BalanceHistory`]
    /// entries. Defaults to 3600s.
    ///
    /// [`BalanceHistory`]: crate::BalanceHistory
    pub async fn get_balance_history(
        &mut self,
        address: super::Address,
        from: Option<super::Time>,
        to: Option<super::Time>,
        currencies: Option<Vec<super::Currency>>,
        group_by: Option<u32>,
    ) -> Result<Vec<super::BalanceHistory>> {
        #[derive(serde::Serialize)]
        struct Params {
            descriptor: super::Address,
            from: Option<super::Time>,
            to: Option<super::Time>,
            currencies: Option<Vec<super::Currency>>,
            #[serde(rename = "groupBy")]
            group_by: Option<u32>,
        }

        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        self.jobs
            .send(Job {
                method: "getBalanceHistory",
                params: Some(Box::new(Params {
                    descriptor: address,
                    from,
                    to,
                    currencies,
                    group_by,
                })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.next().await.unwrap().and_then(|resp| {
            if let Response::OneOff(OneOffResponse::BalanceHistory(history)) = resp {
                return Ok(history);
            }
            Err(Error::DataObjectMismatch)
        })
    }

    pub async fn subscribe_blocks(&mut self) -> impl futures::stream::Stream<Item = Result<Block>> {
        let (tx, rx) = futures::channel::mpsc::channel(10);
        self.jobs
            .send(Job {
                method: "subscribeNewBlock",
                params: None,
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.map(|result| {
            result.and_then(|resp| {
                if let Response::Streaming(StreamingResponse::Block(b)) = resp {
                    return Ok(b);
                }
                Err(Error::DataObjectMismatch)
            })
        })
    }

    /// If `None` is passed, all available fiat rates
    /// will be returned on each update.
    pub async fn subscribe_fiat_rates(
        &mut self,
        currency: Option<super::Currency>,
    ) -> impl futures::stream::Stream<Item = Result<std::collections::HashMap<super::Currency, f64>>>
    {
        #[derive(serde::Serialize)]
        struct Params {
            currency: super::Currency,
        }
        let (tx, rx) = futures::channel::mpsc::channel(10);
        self.jobs
            .send(Job {
                method: "subscribeFiatRates",
                params: currency.map(|c| {
                    Box::new(Params { currency: c })
                        as Box<(dyn erased_serde::Serialize + std::marker::Send + Sync + 'static)>
                }),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.map(|result| {
            result.and_then(|resp| {
                if let Response::Streaming(StreamingResponse::FiatRates { rates }) = resp {
                    return Ok(rates);
                }
                Err(Error::DataObjectMismatch)
            })
        })
    }

    pub async fn subscribe_addresses(
        &mut self,
        addresses: Vec<super::Address>,
    ) -> impl futures::stream::Stream<Item = Result<(super::Address, super::Transaction)>> {
        #[derive(serde::Serialize)]
        struct Params {
            addresses: Vec<super::Address>,
        }
        let (tx, rx) = futures::channel::mpsc::channel(10);
        self.jobs
            .send(Job {
                method: "subscribeAddresses",
                params: Some(Box::new(Params { addresses })),
                response_channel: tx,
            })
            .await
            .unwrap();
        rx.map(|result| {
            result.and_then(|resp| {
                if let Response::Streaming(StreamingResponse::Address { address, tx }) = resp {
                    return Ok((address, tx));
                }
                Err(Error::DataObjectMismatch)
            })
        })
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct FiatRates {
    #[serde(rename = "ts")]
    pub timestamp: u32,
    pub rates: std::collections::HashMap<super::Currency, f64>,
}

#[derive(Debug, serde::Deserialize)]
struct EstimateFee {
    #[serde(rename = "feePerUnit")]
    #[serde(with = "super::amount")]
    fee_per_unit: super::Amount,
}

#[derive(Debug, serde::Deserialize)]
struct EstimateTxFee {
    #[serde(rename = "feePerTx")]
    #[serde(with = "super::amount")]
    fee_per_tx: super::Amount,
}
