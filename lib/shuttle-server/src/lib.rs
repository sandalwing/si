//! This crate provides an opinionated [`naxum`] server that "shuttles" (consumes on a source stream and publishes to a
//! destination subject) NATS JetStream stream messages to another subject until a "final message" (a message with
//! [`FINAL_MESSAGE_HEADER_KEY`] in its headers) is seen.

#![warn(
    bad_style,
    clippy::missing_panics_doc,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    dead_code,
    improper_ctypes,
    missing_debug_implementations,
    missing_docs,
    no_mangle_generic_items,
    non_shorthand_field_patterns,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    unconditional_recursion,
    unreachable_pub,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]

use std::{future::IntoFuture, io};

use futures::Future;
use middleware::DeleteMessageOnSuccess;
use naxum::{
    handler::Handler,
    middleware::{post_process::PostProcessLayer, trace::TraceLayer},
    response::{IntoResponse, Response},
    ServiceBuilder, ServiceExt, TowerServiceExt,
};
use si_data_nats::{
    async_nats::{
        self,
        jetstream::{
            consumer::StreamErrorKind, context::RequestErrorKind, stream::ConsumerErrorKind,
        },
    },
    jetstream, Subject,
};
use si_data_nats::{jetstream::Context, NatsClient};
use si_events::ulid::Ulid;
use telemetry::prelude::*;
use telemetry::tracing::error;
use thiserror::Error;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

mod app_state;
mod handlers;
mod middleware;

pub use shuttle_core::FINAL_MESSAGE_HEADER_KEY;

#[allow(missing_docs)]
#[remain::sorted]
#[derive(Debug, Error)]
pub enum ShuttleError {
    #[error("async nats consumer error: {0}")]
    AsyncNatsConsumer(#[from] async_nats::error::Error<ConsumerErrorKind>),
    #[error("async nats request error: {0}")]
    AsyncNatsRequest(#[from] async_nats::error::Error<RequestErrorKind>),
    #[error("async nats stream error: {0}")]
    AsyncNatsStream(#[from] async_nats::error::Error<StreamErrorKind>),
    #[error("naxum error: {0}")]
    Naxum(#[source] io::Error),
}

type Result<T> = std::result::Result<T, ShuttleError>;

/// A running, opinionated [`naxum`] server that "shuttles" messages from a limits-based stream to
/// another given subject.
pub struct Shuttle {
    source_subject: Subject,
    destination_subject: Subject,
    shutdown_cleanup_toolkit: ShuttleShutdownCleanupToolkit,
    inner: Box<dyn Future<Output = io::Result<()>> + Unpin + Send>,
}

impl std::fmt::Debug for Shuttle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shuttle")
            .field("source_subject", &self.source_subject)
            .field("destination_subject", &self.destination_subject)
            .field("shutdown_cleanup_toolkit", &self.shutdown_cleanup_toolkit)
            .finish_non_exhaustive()
    }
}

impl Shuttle {
    /// Creates a new running [`Shuttle`] instance.
    #[instrument(
        name = "shuttle.new",
        level = "trace",
        skip_all,
        fields(source_subject, destination_subject)
    )]
    pub async fn new(
        nats: NatsClient,
        tracker: TaskTracker,
        limits_based_source_stream: async_nats::jetstream::stream::Stream,
        source_subject: Subject,
        destination_subject: Subject,
    ) -> Result<Self> {
        let self_shutdown_token = CancellationToken::new();

        let deliver_subject = nats.new_inbox();
        let connection_metadata = nats.metadata_clone();
        let context = jetstream::new(nats);

        let consumer_name = format!("shuttle-{}", Ulid::new());
        let source_stream_name = limits_based_source_stream
            .get_info()
            .await?
            .config
            .name
            .to_owned();

        let incoming = {
            limits_based_source_stream
                .create_consumer(async_nats::jetstream::consumer::push::OrderedConfig {
                    name: Some(consumer_name.to_owned()),
                    deliver_subject,
                    filter_subject: source_subject.to_string(),
                    ..Default::default()
                })
                .await?
                .messages()
                .await?
        };

        let state = crate::app_state::AppState::new(
            context.clone(),
            destination_subject.clone(),
            self_shutdown_token.clone(),
        );

        let app = ServiceBuilder::new()
            .layer(
                TraceLayer::new()
                    .make_span_with(
                        telemetry_nats::NatsMakeSpan::builder(connection_metadata).build(),
                    )
                    .on_response(telemetry_nats::NatsOnResponse::new()),
            )
            .layer(
                PostProcessLayer::new()
                    .on_success(DeleteMessageOnSuccess::new(limits_based_source_stream)),
            )
            .service(crate::handlers::default.with_state(state))
            .map_response(Response::into_response);

        let inner = naxum::serve(incoming, app.into_make_service())
            .with_graceful_shutdown(naxum::wait_on_cancelled(self_shutdown_token));

        Ok(Self {
            source_subject,
            destination_subject,
            shutdown_cleanup_toolkit: ShuttleShutdownCleanupToolkit {
                consumer_name,
                source_stream_name,
                context,
                tracker,
            },
            inner: Box::new(inner.into_future()),
        })
    }

    /// Fallibly awaits the inner naxum task.
    #[instrument(name = "shuttle.try_run", level = "trace", skip_all)]
    pub async fn try_run(self) -> Result<()> {
        self.inner.await.map_err(ShuttleError::Naxum)?;
        trace!(%self.source_subject, %self.destination_subject, "shuttle inner loop exited, now performing cleanup");
        self.shutdown_cleanup_toolkit.spawn_cleanup_task()?;
        trace!(%self.source_subject, %self.destination_subject, "shuttle main loop shutdown complete");
        Ok(())
    }
}

#[derive(Debug)]
struct ShuttleShutdownCleanupToolkit {
    consumer_name: String,
    source_stream_name: String,
    context: Context,
    tracker: TaskTracker,
}

impl ShuttleShutdownCleanupToolkit {
    #[instrument(
        name = "shuttle.shutdown_cleanup_toolkit.spawn_cleanup_task",
        level = "trace",
        skip_all
    )]
    pub(crate) fn spawn_cleanup_task(self) -> Result<()> {
        self.tracker.spawn(async move {
            if let Err(err) = self
                .context
                .delete_consumer_from_stream(self.consumer_name, self.source_stream_name)
                .await
            {
                error!(?err, "error deleting consumer from stream");
            }
        });
        Ok(())
    }
}
