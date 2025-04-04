use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use dal::{ChangeSet, ChangeSetId, WorkspacePk, WorkspaceSnapshotAddress};
use hyper::StatusCode;
use si_frontend_types::object::FrontendObject;
use telemetry::prelude::*;
use thiserror::Error;

use crate::{
    extract::{FriggStore, HandlerContext},
    service::ApiError,
    AppState,
};

use super::AccessBuilder;

#[remain::sorted]
#[derive(Error, Debug)]
pub enum IndexError {
    #[error("change set error: {0}")]
    ChangeSet(#[from] dal::ChangeSetError),
    #[error("frigg error: {0}")]
    Frigg(#[from] frigg::FriggError),
    #[error("index not found; workspace_pk={0}, change_set_id={1}")]
    IndexNotFound(WorkspacePk, ChangeSetId),
    #[error("Materialized view error: {0}")]
    MaterializedView(#[from] dal::materialized_view::MaterializedViewError),
    #[error("transactions error: {0}")]
    Transactions(#[from] dal::TransactionsError),
}

pub type IndexResult<T> = Result<T, IndexError>;

impl IntoResponse for IndexError {
    fn into_response(self) -> Response {
        let status_code = match &self {
            IndexError::IndexNotFound(_, _) => StatusCode::NOT_FOUND,
            _ => ApiError::DEFAULT_ERROR_STATUS_CODE,
        };

        ApiError::new(status_code, self).into_response()
    }
}

pub async fn get_workspace_index(
    HandlerContext(builder): HandlerContext,
    AccessBuilder(access_builder): AccessBuilder,
    FriggStore(frigg): FriggStore,
    Path(workspace_pk): Path<WorkspacePk>,
) -> IndexResult<Json<HashMap<ChangeSetId, Option<FrontendObject>>>> {
    let ctx = builder.build_head(access_builder).await?;

    let mut indexes = HashMap::new();
    for change_set in ChangeSet::list_active(&ctx).await? {
        let maybe_index = frigg.get_index(workspace_pk, change_set.id).await?;
        indexes.insert(change_set.id, maybe_index.map(|i| i.0));
    }

    Ok(Json(indexes))
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrontEndObjectMeta {
    workspace_snapshot_address: WorkspaceSnapshotAddress,
    front_end_object: FrontendObject,
}

pub async fn get_change_set_index(
    HandlerContext(builder): HandlerContext,
    AccessBuilder(access_builder): AccessBuilder,
    FriggStore(frigg): FriggStore,
    Path((workspace_pk, change_set_id)): Path<(WorkspacePk, ChangeSetId)>,
) -> IndexResult<Json<FrontEndObjectMeta>> {
    let ctx = builder
        .build(access_builder.build(change_set_id.into()))
        .await?;
    let change_set = ChangeSet::get_by_id(&ctx, change_set_id).await?;

    let index = match frigg.get_index(workspace_pk, change_set_id).await? {
        Some((index, _kv_revision)) => index,
        None => {
            info!(
                "Index not found for change_set {}; attempting full build",
                change_set_id,
            );
            // We know the change set exists, but the index hasn't been built yet, so
            // we'll trigger a full MV build and then try again.
            dal::materialized_view::build_all_mv_for_change_set(&ctx, &frigg)
                .instrument(tracing::info_span!(
                    "Initial build of all materialized views"
                ))
                .await?;
            ctx.commit_no_rebase().await?;

            frigg
                .get_index(workspace_pk, change_set_id)
                .await?
                .map(|i| i.0)
                .ok_or(IndexError::IndexNotFound(workspace_pk, change_set_id))?
        }
    };

    Ok(Json(FrontEndObjectMeta {
        workspace_snapshot_address: change_set.workspace_snapshot_address,
        front_end_object: index,
    }))
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FrontendObjectRequest {
    pub kind: String,
    pub id: String,
    pub checksum: Option<String>,
}

pub async fn get_front_end_object(
    HandlerContext(builder): HandlerContext,
    AccessBuilder(access_builder): AccessBuilder,
    FriggStore(frigg): FriggStore,
    Path((workspace_pk, change_set_id)): Path<(WorkspacePk, ChangeSetId)>,
    Query(request): Query<FrontendObjectRequest>,
) -> IndexResult<Json<FrontEndObjectMeta>> {
    let ctx = builder
        .build(access_builder.build(change_set_id.into()))
        .await?;
    let change_set = ChangeSet::get_by_id(&ctx, change_set_id).await?;

    let obj;
    if let Some(checksum) = request.checksum {
        obj = frigg
            .get_object(workspace_pk, &request.kind, &request.id, &checksum)
            .await?
            .ok_or(IndexError::IndexNotFound(workspace_pk, change_set_id))?;
    } else {
        obj = frigg
            .get_current_object(workspace_pk, change_set_id, &request.kind, &request.id)
            .await?
            .ok_or(IndexError::IndexNotFound(workspace_pk, change_set_id))?;
    }

    Ok(Json(FrontEndObjectMeta {
        workspace_snapshot_address: change_set.workspace_snapshot_address,
        front_end_object: obj,
    }))
}

pub fn v2_workspace_routes() -> Router<AppState> {
    Router::new().route("/", get(get_workspace_index))
}

pub fn v2_change_set_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_change_set_index))
        .route("/mjolnir", get(get_front_end_object))
}
