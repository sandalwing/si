use chrono::{DateTime, Utc};
use derive_builder::{Builder, UninitializedFieldError};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};
use thiserror::Error;
use url::Url;

#[derive(Builder, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[builder(build_fn(error = "SpecError"))]
pub struct PkgSpec {
    #[builder(setter(into))]
    pub name: String,
    #[builder(setter(into))]
    pub version: String,

    #[builder(setter(into), default)]
    pub description: String,
    #[builder(try_setter, setter(into), default = "Utc::now()")]
    pub created_at: DateTime<Utc>,
    #[builder(setter(into))]
    pub created_by: String,

    #[builder(setter(each(name = "schema", into)), default)]
    pub schemas: Vec<SchemaSpec>,

    #[builder(setter(each(name = "func", into)), default)]
    pub funcs: Vec<FuncSpec>,
}

impl PkgSpec {
    pub fn builder() -> PkgSpecBuilder {
        PkgSpecBuilder::default()
    }
}

impl PkgSpecBuilder {
    #[allow(unused_mut)]
    pub fn try_schema<I>(&mut self, item: I) -> Result<&mut Self, I::Error>
    where
        I: TryInto<SchemaSpec>,
    {
        let converted: SchemaSpec = item.try_into()?;
        Ok(self.schema(converted))
    }

    #[allow(unused_mut)]
    pub fn try_func<I>(&mut self, item: I) -> Result<&mut Self, I::Error>
    where
        I: TryInto<FuncSpec>,
    {
        let converted: FuncSpec = item.try_into()?;
        Ok(self.func(converted))
    }
}

impl TryFrom<PkgSpecBuilder> for PkgSpec {
    type Error = SpecError;

    fn try_from(value: PkgSpecBuilder) -> Result<Self, Self::Error> {
        value.build()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, AsRefStr, Display, EnumIter, EnumString)]
#[serde(rename_all = "camelCase")]
pub enum FuncBackendKind {
    JsAttribute,
    JsWorkflow,
    JsCommand,
    JsValidation,
    Json,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, AsRefStr, Display, EnumIter, EnumString)]
#[serde(rename_all = "camelCase")]
pub enum FuncBackendResponseType {
    Array,
    Boolean,
    Integer,
    Map,
    Object,
    Qualification,
    CodeGeneration,
    Confirmation,
    String,
    Json,
    Validation,
    Workflow,
    Command,
}

#[derive(Builder, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[builder(build_fn(error = "SpecError"))]
pub struct FuncSpec {
    #[builder(setter(into))]
    pub name: String,
    #[builder(setter(into, strip_option), default)]
    pub display_name: Option<String>,
    #[builder(setter(into, strip_option), default)]
    pub description: Option<String>,
    #[builder(setter(into))]
    pub handler: String,
    #[builder(setter(into))]
    pub code_base64: String,
    #[builder(setter(into))]
    pub backend_kind: FuncBackendKind,
    #[builder(setter(into))]
    pub response_type: FuncBackendResponseType,
    #[builder(setter(into))]
    pub hidden: bool,
    #[builder(setter(into, strip_option), default)]
    pub link: Option<Url>,
}

impl FuncSpec {
    pub fn builder() -> FuncSpecBuilder {
        FuncSpecBuilder::default()
    }
}

#[derive(Builder, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[builder(build_fn(error = "SpecError"))]
pub struct SchemaSpec {
    #[builder(setter(into))]
    pub name: String,
    #[builder(setter(into))]
    pub category: String,

    #[builder(setter(each(name = "variant", into)), default)]
    pub variants: Vec<SchemaVariantSpec>,
}

impl SchemaSpec {
    #[must_use]
    pub fn builder() -> SchemaSpecBuilder {
        SchemaSpecBuilder::default()
    }

    #[allow(unused_mut)]
    pub fn try_variant<I>(&mut self, item: I) -> Result<&mut Self, I::Error>
    where
        I: TryInto<SchemaVariantSpec>,
    {
        let converted: SchemaVariantSpec = item.try_into()?;
        self.variants.extend(Some(converted));
        Ok(self)
    }
}

impl TryFrom<SchemaSpecBuilder> for SchemaSpec {
    type Error = SpecError;

    fn try_from(value: SchemaSpecBuilder) -> Result<Self, Self::Error> {
        value.build()
    }
}

#[derive(Builder, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[builder(build_fn(error = "SpecError"))]
pub struct SchemaVariantSpec {
    #[builder(setter(into))]
    pub name: String,
    #[builder(setter(into, strip_option), default)]
    pub link: Option<Url>,
    #[builder(setter(into, strip_option), default)]
    pub color: Option<String>,

    #[builder(private, default = "Self::default_domain()")]
    pub domain: PropSpec,
}

impl SchemaVariantSpec {
    pub fn builder() -> SchemaVariantSpecBuilder {
        SchemaVariantSpecBuilder::default()
    }
}

impl SchemaVariantSpecBuilder {
    fn default_domain() -> PropSpec {
        PropSpec::Object {
            name: "domain".to_string(),
            entries: vec![],
        }
    }

    #[allow(unused_mut)]
    pub fn try_link<V>(&mut self, value: V) -> Result<&mut Self, V::Error>
    where
        V: TryInto<Url>,
    {
        let converted: Url = value.try_into()?;
        Ok(self.link(converted))
    }

    #[allow(unused_mut)]
    pub fn prop(&mut self, item: impl Into<PropSpec>) -> &mut Self {
        let converted: PropSpec = item.into();
        match self.domain.get_or_insert_with(Self::default_domain) {
            PropSpec::Object { entries, .. } => entries.push(converted),
            invalid => unreachable!(
                "domain prop is an object but was found to be: {:?}",
                invalid
            ),
        };
        self
    }

    #[allow(unused_mut)]
    pub fn try_prop<I>(&mut self, item: I) -> Result<&mut Self, I::Error>
    where
        I: TryInto<PropSpec>,
    {
        let converted: PropSpec = item.try_into()?;
        Ok(self.prop(converted))
    }

    #[allow(unused_mut)]
    pub fn props(&mut self, value: Vec<PropSpec>) -> &mut Self {
        match self.domain.get_or_insert_with(Self::default_domain) {
            PropSpec::Object { entries, .. } => *entries = value,
            invalid => unreachable!(
                "domain prop is an object but was found to be: {:?}",
                invalid
            ),
        };
        self
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PropSpec {
    #[serde(rename_all = "camelCase")]
    String { name: String },
    #[serde(rename_all = "camelCase")]
    Number { name: String },
    #[serde(rename_all = "camelCase")]
    Boolean { name: String },
    #[serde(rename_all = "camelCase")]
    Map {
        name: String,
        type_prop: Box<PropSpec>,
    },
    #[serde(rename_all = "camelCase")]
    Array {
        name: String,
        type_prop: Box<PropSpec>,
    },
    #[serde(rename_all = "camelCase")]
    Object {
        name: String,
        entries: Vec<PropSpec>,
    },
}

impl PropSpec {
    pub fn builder() -> PropSpecBuilder {
        PropSpecBuilder::default()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PropSpecKind {
    String,
    Number,
    Boolean,
    Map,
    Array,
    Object,
}

#[derive(Clone, Debug, Default)]
pub struct PropSpecBuilder {
    kind: Option<PropSpecKind>,
    name: Option<String>,
    type_prop: Option<PropSpec>,
    entries: Vec<PropSpec>,
}

impl PropSpecBuilder {
    #[allow(unused_mut)]
    pub fn kind(&mut self, value: PropSpecKind) -> &mut Self {
        self.kind = Some(value);
        self
    }

    pub fn get_kind(&self) -> Option<PropSpecKind> {
        self.kind
    }

    #[allow(unused_mut)]
    pub fn name(&mut self, value: impl Into<String>) -> &mut Self {
        self.name = Some(value.into());
        self
    }

    #[allow(unused_mut)]
    pub fn type_prop(&mut self, value: impl Into<PropSpec>) -> &mut Self {
        self.type_prop = Some(value.into());
        self
    }

    #[allow(unused_mut)]
    pub fn entry(&mut self, value: impl Into<PropSpec>) -> &mut Self {
        self.entries.push(value.into());
        self
    }

    #[allow(unused_mut)]
    pub fn entries(&mut self, value: Vec<impl Into<PropSpec>>) -> &mut Self {
        self.entries = value.into_iter().map(Into::into).collect();
        self
    }

    /// Builds a new `Prop`.
    ///
    /// # Errors
    ///
    /// If a required field has not been initialized.
    pub fn build(&self) -> Result<PropSpec, SpecError> {
        let name = match self.name {
            Some(ref name) => name.clone(),
            None => {
                return Err(UninitializedFieldError::from("name").into());
            }
        };

        Ok(match self.kind {
            Some(kind) => match kind {
                PropSpecKind::String => PropSpec::String { name },
                PropSpecKind::Number => PropSpec::Number { name },
                PropSpecKind::Boolean => PropSpec::Boolean { name },
                PropSpecKind::Map => PropSpec::Map {
                    name,
                    type_prop: match self.type_prop {
                        Some(ref value) => Box::new(value.clone()),
                        None => {
                            return Err(UninitializedFieldError::from("type_prop").into());
                        }
                    },
                },
                PropSpecKind::Array => PropSpec::Array {
                    name,
                    type_prop: match self.type_prop {
                        Some(ref value) => Box::new(value.clone()),
                        None => {
                            return Err(UninitializedFieldError::from("type_prop").into());
                        }
                    },
                },
                PropSpecKind::Object => PropSpec::Object {
                    name,
                    entries: self.entries.clone(),
                },
            },
            None => {
                return Err(UninitializedFieldError::from("kind").into());
            }
        })
    }
}

impl TryFrom<PropSpecBuilder> for PropSpec {
    type Error = SpecError;

    fn try_from(value: PropSpecBuilder) -> Result<Self, Self::Error> {
        value.build()
    }
}

#[derive(Debug, Error)]
pub enum SpecError {
    /// Uninitialized field
    #[error("{0} must be initialized")]
    UninitializedField(&'static str),
    /// Custom validation error
    #[error("{0}")]
    ValidationError(String),
}

impl From<UninitializedFieldError> for SpecError {
    fn from(value: UninitializedFieldError) -> Self {
        Self::UninitializedField(value.field_name())
    }
}

impl From<String> for SpecError {
    fn from(value: String) -> Self {
        Self::ValidationError(value)
    }
}
