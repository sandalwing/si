use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowResolveRequest {
    pub execution_id: String,
    pub handler: String,
    pub code_base64: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowResolveResultSuccess {
    pub execution_id: String,
}

#[cfg(feature = "server")]
pub(crate) mod server {
    use super::*;

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LangServerWorkflowResolveResultSuccess {
        pub execution_id: String,
    }

    impl From<LangServerWorkflowResolveResultSuccess> for WorkflowResolveResultSuccess {
        fn from(value: LangServerWorkflowResolveResultSuccess) -> Self {
            Self {
                execution_id: value.execution_id,
            }
        }
    }
}
