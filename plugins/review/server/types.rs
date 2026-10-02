use serde::{Deserialize, Serialize};

use crate::dataroom::types::DocumentStatus;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct ReviewHealthResponse {
    pub status: String,
    pub plugin_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub enum ReviewDecision {
    Satisfied,
    NeedsInformation,
}

impl ReviewDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::NeedsInformation => "needs_information",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "satisfied" => Some(Self::Satisfied),
            "needs_information" => Some(Self::NeedsInformation),
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
pub struct Criterion {
    pub id: String,
    pub title: String,
    pub review_question: String,
    pub display_order: i16,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct EvidenceRef {
    pub document_id: String,
    pub title: String,
    pub file_name: String,
    pub status: DocumentStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct Review {
    pub id: String,
    pub criterion_id: String,
    pub decision: ReviewDecision,
    pub comment: String,
    pub evidence: Vec<EvidenceRef>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct ListCriteriaResponse {
    pub items: Vec<Criterion>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct ListMyReviewsResponse {
    pub items: Vec<Review>,
}

/// `listCriteria`·`listMyReviews` take no fields; `null` and `{}` are both accepted (spec D-39).
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoParams {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct GetMyReviewParams {
    pub criterion_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct SaveReviewParams {
    pub criterion_id: String,
    pub decision: ReviewDecision,
    pub comment: String,
    pub evidence_document_ids: Vec<String>,
}
