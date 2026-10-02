use crate::error::ApiError;

use super::types::{Criterion, EvidenceRef, Review, ReviewDecision};

/// `reviews r` timestamps as ISO 8601 UTC strings (spec D-5).
pub(super) const REVIEW_TIMESTAMPS: &str = r#"to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS created_at,
       to_char(r.updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS updated_at"#;

#[derive(Debug, sqlx::FromRow)]
pub(super) struct CriterionRow {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) review_question: String,
    pub(super) display_order: i16,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct ReviewRow {
    pub(super) id: String,
    pub(super) criterion_id: String,
    pub(super) decision: String,
    pub(super) comment: String,
    pub(super) created_at: String,
    pub(super) updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct EvidenceRow {
    pub(super) review_id: String,
    pub(super) document_id: String,
}

impl From<CriterionRow> for Criterion {
    fn from(row: CriterionRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            review_question: row.review_question,
            display_order: row.display_order,
        }
    }
}

impl ReviewRow {
    pub(super) fn into_review(self, evidence: Vec<EvidenceRef>) -> Result<Review, ApiError> {
        let decision = ReviewDecision::parse(&self.decision)
            .ok_or_else(|| ApiError::storage("Unknown review decision."))?;
        Ok(Review {
            id: self.id,
            criterion_id: self.criterion_id,
            decision,
            comment: self.comment,
            evidence,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
