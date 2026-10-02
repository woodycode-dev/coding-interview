use crate::error::ApiError;

use super::types::{DocumentAuthor, DocumentDetail, DocumentStatus, DocumentSummary};

/// `documents d` column `created_at` as an ISO 8601 UTC string (spec D-5).
pub(super) const DOCUMENT_CREATED_AT: &str =
    r#"to_char(d.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"')"#;

#[derive(Debug, sqlx::FromRow)]
pub(super) struct DataroomRow {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) description: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct DocumentSummaryRow {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) file_name: String,
    pub(super) status: String,
    pub(super) created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct DocumentDetailRow {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) file_name: String,
    pub(super) status: String,
    pub(super) content: String,
    pub(super) created_at: String,
    pub(super) created_by_id: String,
    pub(super) created_by_name: String,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct DocumentRefRow {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) file_name: String,
    pub(super) status: String,
}

/// Document fields other modules (the review plugin) may show for a document ID (spec D-21).
#[derive(Debug)]
pub struct DocumentRef {
    pub id: String,
    pub title: String,
    pub file_name: String,
    pub status: DocumentStatus,
}

fn status(value: &str) -> Result<DocumentStatus, ApiError> {
    DocumentStatus::parse(value).ok_or_else(|| ApiError::storage("Unknown document status."))
}

impl TryFrom<DocumentSummaryRow> for DocumentSummary {
    type Error = ApiError;

    fn try_from(row: DocumentSummaryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            status: status(&row.status)?,
            id: row.id,
            title: row.title,
            file_name: row.file_name,
            created_at: row.created_at,
        })
    }
}

impl TryFrom<DocumentDetailRow> for DocumentDetail {
    type Error = ApiError;

    fn try_from(row: DocumentDetailRow) -> Result<Self, Self::Error> {
        Ok(Self {
            status: status(&row.status)?,
            id: row.id,
            title: row.title,
            file_name: row.file_name,
            content: row.content,
            created_at: row.created_at,
            created_by: DocumentAuthor {
                id: row.created_by_id,
                name: row.created_by_name,
            },
        })
    }
}

impl TryFrom<DocumentRefRow> for DocumentRef {
    type Error = ApiError;

    fn try_from(row: DocumentRefRow) -> Result<Self, Self::Error> {
        Ok(Self {
            status: status(&row.status)?,
            id: row.id,
            title: row.title,
            file_name: row.file_name,
        })
    }
}
