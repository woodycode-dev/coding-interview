pub mod models;
pub mod types;
use std::collections::HashSet;

use crate::{
    auth::AuthenticatedUser,
    dataroom::{self, models::DocumentRef},
    error::ApiError,
    types::{PluginRpcRequest, RpcResponse, UserRole},
};
use axum::http::StatusCode;
use models::{CriterionRow, EvidenceRow, REVIEW_TIMESTAMPS, ReviewRow};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use types::{
    Criterion, EvidenceRef, GetMyReviewParams, ListCriteriaResponse, ListMyReviewsResponse,
    NoParams, Review, ReviewHealthResponse, SaveReviewParams,
};

pub const ID: &str = "review";

const ID_MAX_CHARS: usize = 100;
const COMMENT_MAX_CHARS: usize = 2000;
const EVIDENCE_MAX_COUNT: usize = 20;

pub async fn dispatch(
    pool: &PgPool,
    user: &AuthenticatedUser,
    request: PluginRpcRequest,
) -> Result<RpcResponse, ApiError> {
    if request.workspace_id != user.workspace_id {
        return Err(ApiError::forbidden());
    }
    let result = match request.method.as_str() {
        "health" => to_result(ReviewHealthResponse {
            status: "ok".into(),
            plugin_id: ID.into(),
        }),
        "listCriteria" => to_result(list_criteria(pool, request.params).await?),
        "listMyReviews" => to_result(list_my_reviews(pool, user, request.params).await?),
        "getMyReview" => to_result(get_my_review(pool, user, request.params).await?),
        "saveReview" => to_result(save_review(pool, user, request.params).await?),
        _ => Err(ApiError::invalid("Unknown method.")),
    }?;
    Ok(RpcResponse { result })
}

async fn list_criteria(pool: &PgPool, params: Value) -> Result<ListCriteriaResponse, ApiError> {
    let _: NoParams = parse_optional_params(params)?;
    let rows = sqlx::query_as::<_, CriterionRow>(
        "SELECT id, title, review_question, display_order
         FROM review_criteria
         ORDER BY display_order ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(ApiError::storage)?;
    Ok(ListCriteriaResponse {
        items: rows.into_iter().map(Criterion::from).collect(),
    })
}

async fn list_my_reviews(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: Value,
) -> Result<ListMyReviewsResponse, ApiError> {
    // README: a company member gets an empty list; role comes before params (spec 1.2, D-51).
    if !matches!(user.role, UserRole::Investor) {
        return Ok(ListMyReviewsResponse { items: Vec::new() });
    }
    let _: NoParams = parse_optional_params(params)?;
    let mut tx = begin_read(pool).await?;
    let rows = sqlx::query_as::<_, ReviewRow>(&format!(
        r#"SELECT r.id, r.criterion_id, r.decision, r.comment,
                  {REVIEW_TIMESTAMPS}
           FROM reviews r
           JOIN review_criteria c ON c.id = r.criterion_id
           WHERE r.workspace_id = $1 AND r.investor_id = $2
           ORDER BY c.display_order ASC"#
    ))
    .bind(&user.workspace_id)
    .bind(&user.id)
    .fetch_all(&mut *tx)
    .await
    .map_err(ApiError::storage)?;
    let items = with_evidence(&mut tx, &user.workspace_id, rows).await?;
    tx.commit().await.map_err(ApiError::storage)?;
    Ok(ListMyReviewsResponse { items })
}

async fn get_my_review(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: Value,
) -> Result<Option<Review>, ApiError> {
    // Reject before reading params (spec 1.2, D-6).
    if !matches!(user.role, UserRole::Investor) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Only investors can view their reviews.",
        ));
    }
    let params: GetMyReviewParams = parse_params(params)?;
    validate_id(&params.criterion_id, "Criterion id is invalid.")?;
    let mut tx = begin_read(pool).await?;
    if !criterion_exists(&mut tx, &params.criterion_id).await? {
        return Err(ApiError::not_found());
    }
    let review = find_review(&mut tx, user, &params.criterion_id).await?;
    tx.commit().await.map_err(ApiError::storage)?;
    Ok(review)
}

async fn save_review(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: Value,
) -> Result<Review, ApiError> {
    // Write permission comes before input validation (README, spec 1.2).
    if !matches!(user.role, UserRole::Investor) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Only investors can save reviews.",
        ));
    }
    let params: SaveReviewParams = parse_params(params)?;
    validate_id(&params.criterion_id, "Criterion id is invalid.")?;
    let comment = validate_comment(&params.comment)?;
    validate_evidence_ids(&params.evidence_document_ids)?;

    // One transaction (spec 2.6): any failure below rolls back, keeping the previous
    // review and evidence. Dropping `tx` without commit rolls back.
    let mut tx = pool.begin().await.map_err(ApiError::storage)?;
    // Missing targets before not-ready documents: criterion 404 → document 404 → 400.
    if !criterion_exists(&mut tx, &params.criterion_id).await? {
        return Err(ApiError::not_found());
    }
    dataroom::lock_ready_documents(&mut *tx, &user.workspace_id, &params.evidence_document_ids)
        .await?;
    // The unique (workspace, investor, criterion) row lock serializes concurrent saves;
    // the last one wins (D-14). `id` and `created_at` stay the same on update.
    let review_id: String = sqlx::query_scalar(
        "INSERT INTO reviews (workspace_id, investor_id, criterion_id, decision, comment)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (workspace_id, investor_id, criterion_id) DO UPDATE
         SET decision = EXCLUDED.decision, comment = EXCLUDED.comment, updated_at = NOW()
         RETURNING id",
    )
    .bind(&user.workspace_id)
    .bind(&user.id)
    .bind(&params.criterion_id)
    .bind(params.decision.as_str())
    .bind(&comment)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::storage)?;
    sqlx::query("DELETE FROM review_evidence WHERE workspace_id = $1 AND review_id = $2")
        .bind(&user.workspace_id)
        .bind(&review_id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::storage)?;
    sqlx::query(
        "INSERT INTO review_evidence (review_id, document_id, workspace_id)
         SELECT $1, document_id, $2 FROM UNNEST($3::text[]) AS document_id",
    )
    .bind(&review_id)
    .bind(&user.workspace_id)
    .bind(&params.evidence_document_ids)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::storage)?;
    let review = find_review(&mut tx, user, &params.criterion_id)
        .await?
        .ok_or_else(|| ApiError::storage("Saved review is missing."))?;
    tx.commit().await.map_err(ApiError::storage)?;
    Ok(review)
}

/// Read-only snapshot so a review row and its evidence come from the same moment,
/// even while another request is saving.
async fn begin_read(pool: &PgPool) -> Result<Transaction<'static, Postgres>, ApiError> {
    let mut tx = pool.begin().await.map_err(ApiError::storage)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(ApiError::storage)?;
    Ok(tx)
}

async fn criterion_exists(conn: &mut PgConnection, id: &str) -> Result<bool, ApiError> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM review_criteria WHERE id = $1)")
        .bind(id)
        .fetch_one(&mut *conn)
        .await
        .map_err(ApiError::storage)
}

async fn find_review(
    conn: &mut PgConnection,
    user: &AuthenticatedUser,
    criterion_id: &str,
) -> Result<Option<Review>, ApiError> {
    let row = sqlx::query_as::<_, ReviewRow>(&format!(
        r#"SELECT r.id, r.criterion_id, r.decision, r.comment,
                  {REVIEW_TIMESTAMPS}
           FROM reviews r
           WHERE r.workspace_id = $1 AND r.investor_id = $2 AND r.criterion_id = $3"#
    ))
    .bind(&user.workspace_id)
    .bind(&user.id)
    .bind(criterion_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(ApiError::storage)?;
    match row {
        Some(row) => Ok(with_evidence(conn, &user.workspace_id, vec![row])
            .await?
            .pop()),
        None => Ok(None),
    }
}

/// Attaches each review's evidence. Document fields come from the dataroom module,
/// so this plugin never reads `documents` itself (spec D-21, D-48).
async fn with_evidence(
    conn: &mut PgConnection,
    workspace_id: &str,
    rows: Vec<ReviewRow>,
) -> Result<Vec<Review>, ApiError> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let review_ids: Vec<String> = rows.iter().map(|row| row.id.clone()).collect();
    let links = sqlx::query_as::<_, EvidenceRow>(
        "SELECT review_id, document_id
         FROM review_evidence
         WHERE workspace_id = $1 AND review_id = ANY($2)",
    )
    .bind(workspace_id)
    .bind(&review_ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(ApiError::storage)?;
    let mut document_ids: Vec<String> = links.iter().map(|l| l.document_id.clone()).collect();
    document_ids.sort();
    document_ids.dedup();
    // Already sorted by the document list order (spec D-13).
    let documents = dataroom::document_refs(&mut *conn, workspace_id, &document_ids).await?;
    rows.into_iter()
        .map(|row| {
            let evidence = documents
                .iter()
                .filter(|document| {
                    links
                        .iter()
                        .any(|l| l.review_id == row.id && l.document_id == document.id)
                })
                .map(evidence_ref)
                .collect();
            row.into_review(evidence)
        })
        .collect()
}

fn evidence_ref(document: &DocumentRef) -> EvidenceRef {
    EvidenceRef {
        document_id: document.id.clone(),
        title: document.title.clone(),
        file_name: document.file_name.clone(),
        status: document.status,
    }
}

fn to_result<T: Serialize>(value: T) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(ApiError::storage)
}

fn parse_params<T: DeserializeOwned>(params: Value) -> Result<T, ApiError> {
    serde_json::from_value(params).map_err(|_| ApiError::invalid("Invalid params."))
}

/// Accepts `null` as "no params" (spec D-39).
fn parse_optional_params<T: DeserializeOwned + Default>(params: Value) -> Result<T, ApiError> {
    if params.is_null() {
        Ok(T::default())
    } else {
        parse_params(params)
    }
}

/// Same rule as document IDs (spec D-38, D-49).
fn validate_id(id: &str, message: &'static str) -> Result<(), ApiError> {
    let valid = (1..=ID_MAX_CHARS).contains(&id.len())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(ApiError::invalid(message))
    }
}

/// Trimmed with Unicode whitespace (`char::is_whitespace`), 1-2000 code points (D-24);
/// control characters other than line breaks and tabs are rejected.
fn validate_comment(comment: &str) -> Result<String, ApiError> {
    let comment = comment.trim();
    if comment.is_empty() {
        return Err(ApiError::invalid("Comment must not be blank."));
    }
    if comment.chars().count() > COMMENT_MAX_CHARS {
        return Err(ApiError::invalid("Comment must be 1-2000 characters."));
    }
    if comment
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(ApiError::invalid(
            "Comment must not contain control characters other than line breaks and tabs.",
        ));
    }
    Ok(comment.to_string())
}

/// 1-20 IDs, each in the document ID format, no duplicates (README, D-23).
fn validate_evidence_ids(ids: &[String]) -> Result<(), ApiError> {
    if ids.is_empty() {
        return Err(ApiError::invalid(
            "At least one evidence document is required.",
        ));
    }
    if ids.len() > EVIDENCE_MAX_COUNT {
        return Err(ApiError::invalid(
            "At most 20 evidence documents are allowed.",
        ));
    }
    for id in ids {
        validate_id(id, "Evidence document id is invalid.")?;
    }
    let unique: HashSet<&String> = ids.iter().collect();
    if unique.len() != ids.len() {
        return Err(ApiError::invalid("Evidence documents must not repeat."));
    }
    Ok(())
}
