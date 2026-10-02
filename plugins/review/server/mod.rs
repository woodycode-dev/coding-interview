pub mod models;
pub mod types;
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
use sqlx::PgPool;
use types::{
    Criterion, EvidenceRef, GetMyReviewParams, ListCriteriaResponse, ListMyReviewsResponse,
    NoParams, Review, ReviewHealthResponse,
};

pub const ID: &str = "review";

const CRITERION_ID_MAX_CHARS: usize = 100;

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
    // README: a company member gets an empty list; role comes before params (spec 1.2).
    if !matches!(user.role, UserRole::Investor) {
        return Ok(ListMyReviewsResponse { items: Vec::new() });
    }
    let _: NoParams = parse_optional_params(params)?;
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
    .fetch_all(pool)
    .await
    .map_err(ApiError::storage)?;
    let items = with_evidence(pool, &user.workspace_id, rows).await?;
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
    validate_criterion_id(&params.criterion_id)?;
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM review_criteria WHERE id = $1)")
            .bind(&params.criterion_id)
            .fetch_one(pool)
            .await
            .map_err(ApiError::storage)?;
    if !exists {
        return Err(ApiError::not_found());
    }
    let row = sqlx::query_as::<_, ReviewRow>(&format!(
        r#"SELECT r.id, r.criterion_id, r.decision, r.comment,
                  {REVIEW_TIMESTAMPS}
           FROM reviews r
           WHERE r.workspace_id = $1 AND r.investor_id = $2 AND r.criterion_id = $3"#
    ))
    .bind(&user.workspace_id)
    .bind(&user.id)
    .bind(&params.criterion_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::storage)?;
    match row {
        Some(row) => Ok(with_evidence(pool, &user.workspace_id, vec![row])
            .await?
            .pop()),
        None => Ok(None),
    }
}

/// Attaches each review's evidence. Document fields come from the dataroom module,
/// so this plugin never reads `documents` itself (spec D-21).
async fn with_evidence(
    pool: &PgPool,
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
    .fetch_all(pool)
    .await
    .map_err(ApiError::storage)?;
    let mut document_ids: Vec<String> = links.iter().map(|l| l.document_id.clone()).collect();
    document_ids.sort();
    document_ids.dedup();
    // Already sorted by the document list order (spec D-13).
    let documents = dataroom::document_refs(pool, workspace_id, &document_ids).await?;
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

/// Same rule as document IDs (spec D-38).
fn validate_criterion_id(id: &str) -> Result<(), ApiError> {
    let valid = (1..=CRITERION_ID_MAX_CHARS).contains(&id.len())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(ApiError::invalid("Criterion id is invalid."))
    }
}
