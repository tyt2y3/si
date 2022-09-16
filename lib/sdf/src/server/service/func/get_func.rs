use super::{FuncError, FuncResult};
use crate::server::extract::{AccessBuilder, HandlerContext};
use axum::{extract::Query, Json};
use dal::{
    ComponentId, Func, FuncBackendKind, FuncId, QualificationPrototype, SchemaVariantId,
    StandardModel, Visibility,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetFuncRequest {
    pub id: FuncId,
    #[serde(flatten)]
    pub visibility: Visibility,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetFuncResponse {
    pub id: FuncId,
    pub handler: Option<String>,
    pub kind: FuncBackendKind,
    pub name: String,
    pub description: Option<String>,
    pub code: Option<String>,
    pub is_builtin: bool,
    pub is_revertable: bool,
    pub schema_variants: Vec<SchemaVariantId>,
    pub components: Vec<ComponentId>,
}

pub async fn get_func(
    HandlerContext(builder): HandlerContext,
    AccessBuilder(request_ctx): AccessBuilder,
    Query(request): Query<GetFuncRequest>,
) -> FuncResult<Json<GetFuncResponse>> {
    let ctx = builder.build(request_ctx.build(request.visibility)).await?;

    let func = Func::get_by_id(&ctx, &request.id)
        .await?
        .ok_or(FuncError::FuncNotFound)?;

    let prototypes = QualificationPrototype::find_for_func(&ctx, func.id()).await?;

    let mut schema_variants = vec![];
    let mut components = vec![];

    for proto in prototypes {
        if proto.context().schema_variant_id().is_some() {
            schema_variants.push(proto.context().schema_variant_id());
        } else if proto.context().component_id().is_some() {
            components.push(proto.context().component_id());
        }
    }

    let is_revertable = super::is_func_revertable(&ctx, &func).await?;

    Ok(Json(GetFuncResponse {
        id: func.id().to_owned(),
        handler: func.handler().map(|h| h.to_owned()),
        kind: func.backend_kind().to_owned(),
        name: func
            .display_name()
            .unwrap_or_else(|| func.name())
            .to_owned(),
        description: func.description().map(|d| d.to_owned()),
        code: func.code_plaintext()?,
        is_builtin: func.is_builtin(),
        components,
        schema_variants,
        is_revertable,
    }))
}
