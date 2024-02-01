use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::IntoResponse,
};
use dal::WorkspacePk;
use nats_multiplexer_client::MultiplexerClient;
use si_data_nats::NatsClient;
use std::sync::Arc;
use telemetry::prelude::*;
use tokio::sync::{broadcast, Mutex};

use super::WsError;
use crate::server::nats_multiplexer::NatsMultiplexerClients;
use crate::server::{
    extract::{Nats, WsAuthorization},
    state::ShutdownBroadcast,
};

#[allow(clippy::unused_async)]
pub async fn workspace_updates(
    wsu: WebSocketUpgrade,
    Nats(nats): Nats,
    WsAuthorization(claim): WsAuthorization,
    State(shutdown_broadcast): State<ShutdownBroadcast>,
    State(channel_multiplexer_clients): State<NatsMultiplexerClients>,
) -> Result<impl IntoResponse, WsError> {
    async fn handle_socket(
        socket: WebSocket,
        nats: NatsClient,
        mut shutdown: broadcast::Receiver<()>,
        workspace_pk: WorkspacePk,
        ws_multiplexer_client: Arc<Mutex<MultiplexerClient>>,
    ) {
        tokio::select! {
            _ = run_workspace_updates_proto(socket, nats, workspace_pk, ws_multiplexer_client) => {
                trace!("finished workspace_updates proto");
            }
            _ = shutdown.recv() => {
                trace!("workspace_updates received shutdown, ending session");
            }
            else => {
                trace!("returning from workspace_updates, all select arms closed");
            }
        }
    }

    let shutdown = shutdown_broadcast.subscribe();
    Ok(wsu.on_upgrade(move |socket| {
        handle_socket(
            socket,
            nats,
            shutdown,
            claim.workspace_pk,
            channel_multiplexer_clients.ws,
        )
    }))
}

async fn run_workspace_updates_proto(
    mut socket: WebSocket,
    nats: NatsClient,
    workspace_pk: WorkspacePk,
    ws_multiplexer_client: Arc<Mutex<MultiplexerClient>>,
) {
    let proto = match workspace_updates::run(nats, workspace_pk)
        .start(ws_multiplexer_client)
        .await
    {
        Ok(started) => started,
        Err(err) => {
            // This is likely due to nats failing to subscribe to the required topic, which is
            // suspicious
            warn!(error = ?err, "protocol failed to start");
            return;
        }
    };
    let proto = match proto.process(&mut socket).await {
        Ok(processed) => processed,
        Err(err) => {
            // An error is most likely returned when the client side terminates the websocket
            // session or if a network partition occurs, so this is our "normal" behavior
            trace!(error = ?err, "failed to cleanly complete update stream");
            return;
        }
    };
    if let Err(err) = proto.finish(socket).await {
        // We'd like finish to complete cleanly
        warn!(error = ?err, "failed to finish protocol");
    }
}

mod workspace_updates {
    use axum::extract::ws::{self, WebSocket};
    use dal::{
        user::CursorPayload, user::OnlinePayload, ChangeSetPk, UserPk, WorkspacePk, WsEvent,
        WsEventError,
    };
    use nats_multiplexer_client::{MultiplexerClient, MultiplexerClientError};
    use serde::{Deserialize, Serialize};
    use si_data_nats::{NatsClient, Subject};
    use std::error::Error;
    use std::sync::Arc;
    use telemetry::prelude::*;
    use thiserror::Error;
    use tokio::sync::broadcast::error::RecvError;
    use tokio::sync::{broadcast, Mutex};
    use tokio_tungstenite::tungstenite;

    #[remain::sorted]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "kind", content = "data")]
    pub enum WebsocketEventRequest {
        #[serde(rename_all = "camelCase")]
        Cursor {
            user_pk: UserPk,
            user_name: String,
            change_set_pk: Option<ChangeSetPk>,
            container: Option<String>,
            container_key: Option<String>,
            x: Option<String>,
            y: Option<String>,
        },
        #[serde(rename_all = "camelCase")]
        Online {
            user_pk: UserPk,
            name: String,
            picture_url: Option<String>,
            change_set_pk: Option<ChangeSetPk>,
            idle: bool,
        },
    }

    pub fn run(nats: NatsClient, workspace_pk: WorkspacePk) -> WorkspaceUpdates {
        WorkspaceUpdates { nats, workspace_pk }
    }

    #[remain::sorted]
    #[derive(Debug, Error)]
    pub enum WorkspaceUpdatesError {
        #[error("axum error: {0}")]
        Axum(#[from] axum::Error),
        #[error("broadcast recv error: {0}")]
        BroadcastRecv(#[from] RecvError),
        #[error("nats multiplexer client error: {0}")]
        MultiplexerClient(#[from] MultiplexerClientError),
        #[error("nats error: {0}")]
        Nats(#[from] si_data_nats::Error),
        #[error("serde json error: {0}")]
        Serde(#[from] serde_json::Error),
        #[error("try lock error: {0}")]
        TryLock(#[from] tokio::sync::TryLockError),
        #[error("error when closing websocket")]
        WsClose(#[source] axum::Error),
        #[error("wsevent error: {0}")]
        WsEvent(#[from] WsEventError),
        #[error("error when sending websocket message")]
        WsSendIo(#[source] axum::Error),
    }

    type Result<T> = std::result::Result<T, WorkspaceUpdatesError>;

    #[derive(Debug)]
    pub struct WorkspaceUpdates {
        nats: NatsClient,
        workspace_pk: WorkspacePk,
    }

    impl WorkspaceUpdates {
        pub async fn start(
            self,
            ws_multiplexer_client: Arc<Mutex<MultiplexerClient>>,
        ) -> Result<WorkspaceUpdatesStarted> {
            let subject = Subject::from(format!("si.workspace_pk.{}.>", self.workspace_pk));
            let receiver = ws_multiplexer_client.try_lock()?.receiver(subject).await?;

            Ok(WorkspaceUpdatesStarted {
                nats: self.nats.clone(),
                workspace_pk: self.workspace_pk,
                receiver,
            })
        }
    }

    #[derive(Debug)]
    pub struct WorkspaceUpdatesStarted {
        workspace_pk: WorkspacePk,
        nats: NatsClient,
        receiver: broadcast::Receiver<si_data_nats::Message>,
    }

    impl WorkspaceUpdatesStarted {
        pub async fn process(mut self, ws: &mut WebSocket) -> Result<WorkspaceUpdatesClosing> {
            // Send all messages down the WebSocket until and unless an error is encountered, the
            // client websocket connection is closed, or the nats subscriber naturally closes
            loop {
                tokio::select! {
                    msg = ws.recv() => {
                        match msg {
                            Some(Ok(message)) => if let ws::Message::Text(msg) = message {
                                let event: WebsocketEventRequest = match serde_json::from_str(&msg) {
                                    Ok(event) => event,
                                    Err(err) => {
                                        error!("Unable to deserialize websocket message: {err}");
                                        continue;
                                    }
                                };
                                match event {
                                    WebsocketEventRequest::Cursor { user_pk, user_name, change_set_pk, container, container_key, x, y } => {
                                        let subject = format!("si.workspace_pk.{}.event", self.workspace_pk);
                                        let event = WsEvent::cursor(self.workspace_pk, change_set_pk.unwrap_or(ChangeSetPk::NONE), CursorPayload {
                                            user_pk,
                                            user_name,
                                            x,
                                            y,
                                            container,
                                            container_key
                                        }).await?;
                                        self.nats.publish(subject, serde_json::to_vec(&event)?.into()).await?;
                                    }
                                    WebsocketEventRequest::Online { user_pk, name, picture_url, change_set_pk, idle } => {
                                        let subject = format!("si.workspace_pk.{}.event", self.workspace_pk);
                                        let event = WsEvent::online(self.workspace_pk, OnlinePayload {
                                            user_pk,
                                            name,
                                            picture_url,
                                            change_set_pk,
                                            idle,
                                        }).await?;
                                        self.nats.publish(subject, serde_json::to_vec(&event)?.into()).await?;
                                    }
                                }
                            },
                            Some(Err(err)) => return Err(err.into()),
                            None => return Ok(WorkspaceUpdatesClosing { ws_is_closed: true }),
                        }
                    }
                    recv_result = self.receiver.recv() => {
                        // NOTE(nick): in the long term, determine if we want to return this result or just log it.
                        let nats_msg =  recv_result?;
                        let msg = ws::Message::Text(String::from_utf8_lossy(nats_msg.payload()).to_string());

                        if let Err(err) = ws.send(msg).await {
                            match err
                                .source()
                                .and_then(|err| err.downcast_ref::<tungstenite::Error>())
                            {
                                Some(ws_err) => match ws_err {
                                    // If the websocket has cleanly closed, we should cleanly finish as
                                    // well--this is not an error condition
                                    tungstenite::Error::ConnectionClosed
                                    | tungstenite::Error::AlreadyClosed => {
                                        trace!("websocket has cleanly closed, ending");
                                        return Ok(WorkspaceUpdatesClosing { ws_is_closed: true });
                                    }
                                    _ => return Err(WorkspaceUpdatesError::WsSendIo(err)),
                                },
                                None => return Err(WorkspaceUpdatesError::WsSendIo(err)),
                            }
                        }
                    }
                    else => break,
                }
            }

            Ok(WorkspaceUpdatesClosing {
                ws_is_closed: false,
            })
        }
    }

    #[derive(Debug)]
    pub struct WorkspaceUpdatesClosing {
        ws_is_closed: bool,
    }

    impl WorkspaceUpdatesClosing {
        pub async fn finish(self, ws: WebSocket) -> Result<()> {
            if !self.ws_is_closed {
                ws.close().await.map_err(WorkspaceUpdatesError::WsClose)?;
            }
            Ok(())
        }
    }
}
