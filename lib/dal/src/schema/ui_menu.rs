use serde::{Deserialize, Serialize};
use telemetry::prelude::*;

use crate::{
    impl_standard_model, pk, standard_model, standard_model_belongs_to, DalContext, StandardModel,
    Timestamp, Visibility, WriteTenancy,
};

use super::{Schema, SchemaId, SchemaResult};

const FIND_FOR_SCHEMA: &str = include_str!("../queries/ui_menus_find_for_schema.sql");

pk!(SchemaUiMenuPk);
pk!(SchemaUiMenuId);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchemaUiMenu {
    pub pk: SchemaUiMenuPk,
    id: SchemaUiMenuId,
    name: String,
    category: String,
    #[serde(flatten)]
    tenancy: WriteTenancy,
    #[serde(flatten)]
    timestamp: Timestamp,
    #[serde(flatten)]
    visibility: Visibility,
}

impl_standard_model! {
    model: SchemaUiMenu,
    pk: SchemaUiMenuPk,
    id: SchemaUiMenuId,
    table_name: "schema_ui_menus",
    history_event_label_base: "schema_ui_menu",
    history_event_message_name: "Schema UI Menu"
}

impl SchemaUiMenu {
    #[instrument(skip_all)]
    pub async fn new(
        ctx: &DalContext,
        name: impl AsRef<str>,
        category: impl AsRef<str>,
    ) -> SchemaResult<Self> {
        let name = name.as_ref();
        let category = category.as_ref();

        let row = ctx
            .txns()
            .pg()
            .query_one(
                "SELECT object FROM schema_ui_menu_create_v1($1, $2, $3, $4)",
                &[
                    ctx.write_tenancy(),
                    ctx.visibility(),
                    &(name.to_string()),
                    &(category.to_string()),
                ],
            )
            .await?;
        let object = standard_model::finish_create_from_row(ctx, row).await?;
        Ok(object)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn category(&self) -> &str {
        &self.category
    }

    standard_model_belongs_to!(
        lookup_fn: schema,
        set_fn: set_schema,
        unset_fn: unset_schema,
        table: "schema_ui_menu_belongs_to_schema",
        model_table: "schemas",
        belongs_to_id: SchemaId,
        returns: Schema,
        result: SchemaResult,
    );

    #[instrument(skip_all)]
    pub async fn find_for_schema(
        ctx: &DalContext,
        schema_id: SchemaId,
    ) -> SchemaResult<Option<Self>> {
        let maybe_row = ctx
            .txns()
            .pg()
            .query_opt(
                FIND_FOR_SCHEMA,
                &[ctx.read_tenancy(), ctx.visibility(), &schema_id],
            )
            .await?;

        // NOTE(nick): currently, we are assuming there can only be one "schema_ui_menu" for a given
        // schema id. This might not always be the case.
        let maybe_object: Option<Self> = standard_model::option_object_from_row(maybe_row)?;
        Ok(maybe_object)
    }

    pub fn category_path(&self) -> Vec<String> {
        self.category.split('.').map(|f| f.to_string()).collect()
    }
}
