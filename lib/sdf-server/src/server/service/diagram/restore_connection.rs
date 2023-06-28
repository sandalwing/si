use axum::extract::OriginalUri;
use axum::Json;
use dal::edge::EdgeId;
use dal::{ChangeSet, ChangeSetPk, Connection, Edge, Node, Socket, Visibility, WsEvent};
use serde::{Deserialize, Serialize};

use super::DiagramResult;
use crate::server::extract::{AccessBuilder, HandlerContext, PosthogClient};
use crate::server::tracking::track;
use crate::service::diagram::DiagramError;
use dal::standard_model::StandardModel;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UndeleteConnectionRequest {
    pub edge_id: EdgeId,
    #[serde(flatten)]
    pub visibility: Visibility,
}

/// Delete a [`Connection`](dal::Connection) via its EdgeId.
pub async fn restore_connection(
    HandlerContext(builder): HandlerContext,
    AccessBuilder(request_ctx): AccessBuilder,
    PosthogClient(posthog_client): PosthogClient,
    OriginalUri(original_uri): OriginalUri,
    Json(request): Json<UndeleteConnectionRequest>,
) -> DiagramResult<()> {
    let ctx = builder.build(request_ctx.build(request.visibility)).await?;

    Connection::restore_for_edge(&ctx, request.edge_id).await?;

    let edge = Edge::get_by_id(&ctx, &request.edge_id)
        .await?
        .ok_or(DiagramError::EdgeNotFound)?;

    let conn = Connection::from_edge(&edge);
    let from_component_schema = Node::get_by_id(&ctx, &conn.source.node_id)
        .await?
        .ok_or(DiagramError::NodeNotFound(conn.source.node_id))?
        .component(&ctx)
        .await?
        .ok_or(DiagramError::ComponentNotFound)?
        .schema(&ctx)
        .await?
        .ok_or(DiagramError::SchemaNotFound)?;

    let from_socket = Socket::get_by_id(&ctx, &conn.source.socket_id)
        .await?
        .ok_or(DiagramError::SocketNotFound)?;

    let to_component_schema = Node::get_by_id(&ctx, &conn.destination.node_id)
        .await?
        .ok_or(DiagramError::NodeNotFound(conn.destination.node_id))?
        .component(&ctx)
        .await?
        .ok_or(DiagramError::ComponentNotFound)?
        .schema(&ctx)
        .await?
        .ok_or(DiagramError::SchemaNotFound)?;

    let to_socket = Socket::get_by_id(&ctx, &conn.destination.socket_id)
        .await?
        .ok_or(DiagramError::SocketNotFound)?;

    track(
        &posthog_client,
        &ctx,
        &original_uri,
        "restore_connection",
        serde_json::json!({
            "from_node_id": conn.source.node_id,
            "from_node_schema_name": from_component_schema.name(),
            "from_socket_id": conn.source.socket_id,
            "from_socket_name": &from_socket.name(),
            "to_node_id": conn.destination.node_id,
            "to_node_schema_name": to_component_schema.name(),
            "to_socket_id": conn.destination.socket_id,
            "to_socket_name":  &to_socket.name(),
        }),
    );

    WsEvent::change_set_written(&ctx)
        .await?
        .publish_on_commit(&ctx)
        .await?;

    ctx.commit().await?;

    Ok(())
}

/// Delete a [`Connection`](dal::Connection) via its EdgeId. Creates change-set if on head.
pub async fn restore_connection2(
    HandlerContext(builder): HandlerContext,
    AccessBuilder(request_ctx): AccessBuilder,
    PosthogClient(posthog_client): PosthogClient,
    OriginalUri(original_uri): OriginalUri,
    Json(request): Json<UndeleteConnectionRequest>,
) -> DiagramResult<Json<ForceChangeSet>> {
    let mut ctx = builder.build(request_ctx.build(request.visibility)).await?;

    let mut force_changeset_pk = None;
    if ctx.visibility().is_head() {
        let change_set = ChangeSet::new(&ctx, ChangeSet::generate_name(&ctx).await?, None).await?;

        let new_visibility = Visibility::new(change_set.pk, request.visibility.deleted_at);

        ctx.update_visibility(new_visibility);

        force_changeset_pk = Some(change_set.pk);

        WsEvent::change_set_created(&ctx, change_set.pk)
            .await?
            .publish_on_commit(&ctx)
            .await?;
    };

    Connection::restore_for_edge(&ctx, request.edge_id).await?;

    let edge = Edge::get_by_id(&ctx, &request.edge_id)
        .await?
        .ok_or(DiagramError::EdgeNotFound)?;

    let conn = Connection::from_edge(&edge);
    let from_component_schema = Node::get_by_id(&ctx, &conn.source.node_id)
        .await?
        .ok_or(DiagramError::NodeNotFound(conn.source.node_id))?
        .component(&ctx)
        .await?
        .ok_or(DiagramError::ComponentNotFound)?
        .schema(&ctx)
        .await?
        .ok_or(DiagramError::SchemaNotFound)?;

    let from_socket = Socket::get_by_id(&ctx, &conn.source.socket_id)
        .await?
        .ok_or(DiagramError::SocketNotFound)?;

    let to_component_schema = Node::get_by_id(&ctx, &conn.destination.node_id)
        .await?
        .ok_or(DiagramError::NodeNotFound(conn.destination.node_id))?
        .component(&ctx)
        .await?
        .ok_or(DiagramError::ComponentNotFound)?
        .schema(&ctx)
        .await?
        .ok_or(DiagramError::SchemaNotFound)?;

    let to_socket = Socket::get_by_id(&ctx, &conn.destination.socket_id)
        .await?
        .ok_or(DiagramError::SocketNotFound)?;

    track(
        &posthog_client,
        &ctx,
        &original_uri,
        "restore_connection",
        serde_json::json!({
            "from_node_id": conn.source.node_id,
            "from_node_schema_name": from_component_schema.name(),
            "from_socket_id": conn.source.socket_id,
            "from_socket_name": &from_socket.name(),
            "to_node_id": conn.destination.node_id,
            "to_node_schema_name": to_component_schema.name(),
            "to_socket_id": conn.destination.socket_id,
            "to_socket_name":  &to_socket.name(),
        }),
    );

    WsEvent::change_set_written(&ctx)
        .await?
        .publish_on_commit(&ctx)
        .await?;

    ctx.commit().await?;

    Ok(Json(ForceChangeSet { force_changeset_pk }))
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ForceChangeSet {
    #[serde(rename = "_forceChangesetPk")] // TODO(victor) find a way to return this as a header
    pub force_changeset_pk: Option<ChangeSetPk>,
}
