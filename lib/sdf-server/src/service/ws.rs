use axum::{http::StatusCode, response::IntoResponse, response::Response, routing::get, Router};
use crdt::CrdtError;
use dal::{TransactionsError, WsEventError};
use nats_multiplexer_client::MultiplexerClientError;
use si_data_pg::{PgError, PgPoolError};
use telemetry::prelude::*;
use thiserror::Error;
use tokio::sync::TryLockError;

use crate::AppState;

use super::ApiError;

#[remain::sorted]
#[derive(Debug, Error)]
pub enum WsError {
    #[error("crdt error: {0}")]
    Crdt(#[from] CrdtError),
    #[error("nats multiplexer client error: {0}")]
    MultiplexerClient(#[from] MultiplexerClientError),
    #[error("nats error: {0}")]
    Nats(#[from] si_data_nats::Error),
    #[error("pg error: {0}")]
    Pg(#[from] PgError),
    #[error("pg pool error: {0}")]
    PgPool(#[from] PgPoolError),
    #[error("transactions error: {0}")]
    Transactions(#[from] TransactionsError),
    #[error("try lock error: {0}")]
    TryLock(#[from] TryLockError),
    #[error("wsevent error: {0}")]
    WsEvent(#[from] WsEventError),
}

pub mod crdt;
pub mod workspace_updates;

impl IntoResponse for WsError {
    fn into_response(self) -> Response {
        let (status_code, error_message) = (StatusCode::INTERNAL_SERVER_ERROR, self.to_string());

        ApiError::new(status_code, error_message).into_response()
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workspace_updates",
            get(workspace_updates::workspace_updates),
        )
        .route("/crdt", get(crdt::crdt))
        .route(
            "/bifrost",
            get(crate::service::v2::ws::bifrost::bifrost_handler),
        )
}
