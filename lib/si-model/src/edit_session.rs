use crate::{pk, ChangeSetPk, HistoryActor, HistoryEvent, HistoryEventError, Tenancy, Timestamp};
use serde::{Deserialize, Serialize};
use si_data::{NatsTxn, NatsTxnError, PgError, PgTxn};
use strum_macros::{Display, EnumString};
use thiserror::Error;
use chrono::{DateTime, Utc};

#[derive(Error, Debug)]
pub enum EditSessionError {
    #[error("error serializing/deserializing json: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("pg error: {0}")]
    Pg(#[from] PgError),
    #[error("nats txn error: {0}")]
    NatsTxn(#[from] NatsTxnError),
    #[error("history event error: {0}")]
    HistoryEvent(#[from] HistoryEventError),
}

pub type EditSessionResult<T> = Result<T, EditSessionError>;

#[derive(Deserialize, Serialize, Debug, Display, EnumString, PartialEq, Eq, Clone)]
pub enum EditSessionStatus {
    Open,
    Canceled,
    Saved,
}

pk!(EditSessionPk);
pk!(EditSessionId);

pub const NO_EDIT_SESSION_PK: EditSessionPk = EditSessionPk(-1);

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct EditSession {
    pub pk: EditSessionPk,
    pub id: EditSessionId,
    pub name: String,
    pub note: Option<String>,
    pub status: EditSessionStatus,
    pub change_set_pk: ChangeSetPk,
    #[serde(flatten)]
    pub tenancy: Tenancy,
    #[serde(flatten)]
    pub timestamp: Timestamp,
}

impl EditSession {
    #[tracing::instrument(skip(txn, nats, name, note))]
    pub async fn new(
        txn: &PgTxn<'_>,
        nats: &NatsTxn,
        tenancy: &Tenancy,
        history_actor: &HistoryActor,
        change_set_pk: &ChangeSetPk,
        name: impl AsRef<str>,
        note: Option<&String>,
    ) -> EditSessionResult<Self> {
        let name = name.as_ref();
        let note = note.as_ref();
        let row = txn
            .query_one(
                "SELECT object FROM edit_session_create_v1($1, $2, $3, $4, $5)",
                &[
                    &name,
                    &note,
                    &EditSessionStatus::Open.to_string(),
                    &change_set_pk,
                    &tenancy,
                ],
            )
            .await?;
        let json: serde_json::Value = row.try_get("object")?;
        nats.publish(&json).await?;
        let _history_event = HistoryEvent::new(
            &txn,
            &nats,
            "edit_session.create",
            &history_actor,
            "Edit Session created",
            &json,
            &tenancy,
        )
        .await?;
        let object: Self = serde_json::from_value(json)?;
        Ok(object)
    }

    #[tracing::instrument(skip(txn, nats))]
    pub async fn save(
        &mut self,
        txn: &PgTxn<'_>,
        nats: &NatsTxn,
        history_actor: &HistoryActor,
    ) -> EditSessionResult<()> {
        let actor = serde_json::to_value(&history_actor)?;
        let row = txn
            .query_one(
                "SELECT timestamp_updated_at FROM edit_session_save_v1($1, $2)",
                &[&self.pk, &actor],
            )
            .await?;
        let updated_at: DateTime<Utc> = row.try_get("timestamp_updated_at")?;
        self.timestamp.updated_at = updated_at;
        self.status = EditSessionStatus::Saved;
        let _history_event = HistoryEvent::new(
            &txn,
            &nats,
            "edit_session.save",
            &history_actor,
            "Edit Session saved",
            &serde_json::json![{ "pk": &self.pk }],
            &self.tenancy,
        ).await?;
        Ok(())
    }

}
