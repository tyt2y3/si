use axum::extract::{Json, Query};
use dal::{
    InputSocketId, OutputSocketId, Schema, SchemaId, SchemaVariant, SchemaVariantId, Visibility,
};
use serde::{Deserialize, Serialize};

use super::DiagramResult;
use crate::server::extract::{AccessBuilder, HandlerContext};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListSchemaVariantsRequest {
    #[serde(flatten)]
    pub visibility: Visibility,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OutputSocketView {
    id: OutputSocketId,
    name: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InputSocketView {
    id: InputSocketId,
    name: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaVariantView {
    id: SchemaVariantId,
    builtin: bool,
    name: String,
    schema_name: String,
    schema_id: SchemaId,
    color: String,
    category: String,
    input_sockets: Vec<InputSocketView>,
    output_sockets: Vec<OutputSocketView>,
}

pub type ListSchemaVariantsResponse = Vec<SchemaVariantView>;

pub async fn list_schema_variants(
    HandlerContext(builder): HandlerContext,
    AccessBuilder(request_ctx): AccessBuilder,
    Query(request): Query<ListSchemaVariantsRequest>,
) -> DiagramResult<Json<ListSchemaVariantsResponse>> {
    let ctx = builder.build(request_ctx.build(request.visibility)).await?;

    let mut schema_variants_views: Vec<SchemaVariantView> = Vec::new();
    let schemas = Schema::list(&ctx).await?;

    for schema in schemas {
        if schema.ui_hidden {
            continue;
        }

        let schema_variants = SchemaVariant::list_for_schema(&ctx, schema.id()).await?;
        for schema_variant in schema_variants {
            if schema_variant.ui_hidden() {
                continue;
            }

            let (output_sockets, input_sockets) =
                SchemaVariant::list_all_sockets(&ctx, schema_variant.id()).await?;

            schema_variants_views.push(SchemaVariantView {
                id: schema_variant.id(),
                // FIXME(nick): use the real value here
                builtin: true,
                // builtin: schema_variant.is_builtin(&ctx).await?,
                name: schema_variant.name().to_owned(),
                schema_id: schema.id(),
                schema_name: schema.name.to_owned(),
                color: schema_variant
                    .get_color(&ctx)
                    .await?
                    .unwrap_or("#0F0F0F".into()),
                category: schema_variant.category().to_owned(),
                input_sockets: input_sockets
                    .iter()
                    .map(|s| InputSocketView {
                        id: s.id(),
                        name: s.name().to_owned(),
                    })
                    .collect(),
                output_sockets: output_sockets
                    .iter()
                    .map(|s| OutputSocketView {
                        id: s.id(),
                        name: s.name().to_owned(),
                    })
                    .collect(),
            });
        }
    }

    Ok(Json(schema_variants_views))
}
