use serde::{Deserialize, Serialize};
use si_events::{
    FuncId, InputSocketId, OutputSocketId, PropId, SchemaId, SchemaVariantId, Timestamp,
};
use strum::{AsRefStr, Display, EnumIter, EnumString};

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaVariant {
    pub schema_id: SchemaId,
    pub schema_name: String,
    pub schema_variant_id: SchemaVariantId,
    pub version: String,
    pub display_name: String,
    pub category: String,
    pub description: Option<String>,
    pub link: Option<String>,
    pub color: String,
    pub asset_func_id: FuncId,
    pub func_ids: Vec<FuncId>,
    pub component_type: ComponentType,
    pub input_sockets: Vec<InputSocket>,
    pub output_sockets: Vec<OutputSocket>,
    pub props: Vec<Prop>,
    pub is_locked: bool, // if unlocked, show in both places
    #[serde(flatten)]
    pub timestamp: Timestamp,
    pub can_create_new_components: bool, // if yes, show in modeling screen, if not, only show in customize
    pub can_contribute: bool,
}

#[remain::sorted]
#[derive(
    AsRefStr,
    Clone,
    Copy,
    Debug,
    Deserialize,
    EnumString,
    Eq,
    Serialize,
    Display,
    EnumIter,
    Ord,
    PartialEq,
    PartialOrd,
)]
#[serde(rename_all = "camelCase")]
pub enum ComponentType {
    AggregationFrame,
    Component,
    ConfigurationFrameDown,
    ConfigurationFrameUp,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputSocket {
    pub id: InputSocketId,
    pub name: String,
    pub eligible_to_send_data: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputSocket {
    pub id: OutputSocketId,
    pub name: String,
    pub eligible_to_receive_data: bool,
}
#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Prop {
    pub id: PropId,
    pub kind: PropKind,
    pub name: String,
    pub path: String,
    pub hidden: bool,
    pub eligible_to_receive_data: bool,
    pub eligible_to_send_data: bool,
}
#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PropKind {
    Any,
    Array,
    Boolean,
    Integer,
    Json,
    Map,
    Object,
    String,
}
