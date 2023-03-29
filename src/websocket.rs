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
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "test", serde(deny_unknown_fields))]
enum OneOffResponse {
    Info(Info),
    BlockHash { hash: super::BlockHash },
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
}
