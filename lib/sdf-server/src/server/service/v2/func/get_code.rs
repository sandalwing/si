use axum::{
    extract::{OriginalUri, Path, Query},
    Json,
};
use dal::{ChangeSetId, FuncId, WorkspacePk};

use serde::{Deserialize, Serialize};
use si_frontend_types::FuncCode;

use crate::server::extract::{AccessBuilder, HandlerContext, PosthogClient};

use super::{get_code_response, FuncAPIResult};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]

// TODO: find the right way to pass a Vec<FuncId>
// the API call uses the `id[]=<...>&id[]=<...?` format
// but that doesn't work here with Rust
pub struct GetRequest {
    pub id: FuncId,
}

pub async fn get_code(
    HandlerContext(builder): HandlerContext,
    AccessBuilder(access_builder): AccessBuilder,
    PosthogClient(_posthog_client): PosthogClient,
    OriginalUri(_original_uri): OriginalUri,
    Path((_workspace_pk, change_set_id)): Path<(WorkspacePk, ChangeSetId)>,
    Query(request): Query<GetRequest>,
) -> FuncAPIResult<Json<Vec<FuncCode>>> {
    let ctx = builder
        .build(access_builder.build(change_set_id.into()))
        .await?;
    let mut funcs = Vec::new();

    funcs.push(get_code_response(&ctx, request.id).await?);
    Ok(Json(funcs))
}
