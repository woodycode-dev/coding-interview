use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub enum DocumentStatus {
    Ready,
    Processing,
    Failed,
}

impl DocumentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Processing => "processing",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ready" => Some(Self::Ready),
            "processing" => Some(Self::Processing),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct DataroomInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct DocumentSummary {
    pub id: String,
    pub title: String,
    pub file_name: String,
    pub status: DocumentStatus,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct DocumentAuthor {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct DocumentDetail {
    pub id: String,
    pub title: String,
    pub file_name: String,
    pub status: DocumentStatus,
    pub content: String,
    pub created_at: String,
    pub created_by: DocumentAuthor,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct ListDocumentsResponse {
    pub items: Vec<DocumentSummary>,
}

/// `getDataroom` takes no fields; `null` and `{}` are both accepted.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetDataroomParams {}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct ListDocumentsParams {
    #[serde(default)]
    #[cfg_attr(feature = "ts-bridge", ts(optional))]
    pub query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct GetDocumentParams {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct CreateDocumentParams {
    pub title: String,
    pub file_name: String,
    pub content: String,
}
