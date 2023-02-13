use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use dal::{
    schema::variant::definition::{
        SchemaVariantDefinitionError as DalSchemaVariantDefinitionError, SchemaVariantDefinitionId,
    },
    StandardModelError, TenancyError, TransactionsError, WsEventError,
};
use thiserror::Error;

pub mod create_variant_def;
pub mod exec_variant_def;
pub mod get_variant_def;
pub mod list_variant_defs;
pub mod save_variant_def;

#[derive(Error, Debug)]
pub enum SchemaVariantDefinitionError {
    #[error(transparent)]
    Pg(#[from] si_data_pg::PgError),
    #[error(transparent)]
    PgPool(#[from] si_data_pg::PgPoolError),
    #[error("tenancy error: {0}")]
    Tenancy(#[from] TenancyError),
    #[error(transparent)]
    StandardModel(#[from] StandardModelError),
    #[error(transparent)]
    ContextTransaction(#[from] TransactionsError),
    #[error("could not publish websocket event: {0}")]
    WsEvent(#[from] WsEventError),
    #[error("json serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    SchemaVariantDefinition(#[from] DalSchemaVariantDefinitionError),
    #[error("Schema Variant Definition {0} not found")]
    VariantDefnitionNotFound(SchemaVariantDefinitionId),
    #[error("error creating schema variant from definition: {0}")]
    CouldNotCreateSchemaVariantFromDefinition(String),
}

pub type SchemaVariantDefinitionResult<T> = Result<T, SchemaVariantDefinitionError>;

impl IntoResponse for SchemaVariantDefinitionError {
    fn into_response(self) -> Response {
        let (status, error_message) = (StatusCode::INTERNAL_SERVER_ERROR, self.to_string());

        let body = Json(
            serde_json::json!({ "error": { "message": error_message, "code": 42, "statusCode": status.as_u16() } }),
        );

        (status, body).into_response()
    }
}

pub fn routes() -> Router {
    Router::new()
        .route(
            "/list_variant_defs",
            get(list_variant_defs::list_variant_defs),
        )
        .route("/get_variant_def", get(get_variant_def::get_variant_def))
        .route(
            "/save_variant_def",
            post(save_variant_def::save_variant_def),
        )
        .route(
            "/create_variant_def",
            post(create_variant_def::create_variant_def),
        )
        .route(
            "/exec_variant_def",
            post(exec_variant_def::exec_variant_def),
        )
}
