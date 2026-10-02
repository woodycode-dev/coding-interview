pub mod models;
pub mod types;
use crate::{
    auth::AuthenticatedUser,
    error::ApiError,
    types::{DataroomRpcRequest, RpcResponse, UserRole},
};
use axum::http::StatusCode;
use models::{
    DOCUMENT_CREATED_AT, DataroomRow, DocumentDetailRow, DocumentRef, DocumentRefRow,
    DocumentSummaryRow,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{PgExecutor, PgPool};
use types::{
    CreateDocumentParams, DataroomInfo, DocumentDetail, DocumentStatus, DocumentSummary,
    GetDataroomParams, GetDocumentParams, ListDocumentsParams, ListDocumentsResponse,
};

const TITLE_MAX_CHARS: usize = 200;
const FILE_NAME_MAX_CHARS: usize = 255;
const CONTENT_MAX_BYTES: usize = 200_000;
const QUERY_MAX_CHARS: usize = 100;
const DOCUMENT_ID_MAX_CHARS: usize = 100;

pub async fn dispatch(
    pool: &PgPool,
    user: &AuthenticatedUser,
    request: DataroomRpcRequest,
) -> Result<RpcResponse, ApiError> {
    if request.workspace_id != user.workspace_id {
        return Err(ApiError::forbidden());
    }
    let result = match request.method.as_str() {
        "getDataroom" => to_result(get_dataroom(pool, user, request.params).await?),
        "listDocuments" => to_result(list_documents(pool, user, request.params).await?),
        "getDocument" => to_result(get_document(pool, user, request.params).await?),
        "createDocument" => to_result(create_document(pool, user, request.params).await?),
        _ => Err(ApiError::invalid("Unknown method.")),
    }?;
    Ok(RpcResponse { result })
}

async fn get_dataroom(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: Value,
) -> Result<DataroomInfo, ApiError> {
    let _: GetDataroomParams = parse_optional_params(params)?;
    let row = sqlx::query_as::<_, DataroomRow>(
        "SELECT id, name, description FROM workspaces WHERE id = $1",
    )
    .bind(&user.workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::storage)?
    .ok_or_else(ApiError::not_found)?;
    Ok(DataroomInfo {
        id: row.id,
        name: row.name,
        description: row.description,
    })
}

async fn list_documents(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: Value,
) -> Result<ListDocumentsResponse, ApiError> {
    let params: ListDocumentsParams = parse_optional_params(params)?;
    let pattern = search_pattern(params.query)?;
    let rows = sqlx::query_as::<_, DocumentSummaryRow>(&format!(
        r#"SELECT d.id, d.title, d.file_name, d.status, {DOCUMENT_CREATED_AT} AS created_at
           FROM documents d
           WHERE d.workspace_id = $1
             AND ($2::text IS NULL OR d.title ILIKE $2 ESCAPE '\')
           ORDER BY d.created_at DESC, d.id ASC"#
    ))
    .bind(&user.workspace_id)
    .bind(pattern)
    .fetch_all(pool)
    .await
    .map_err(ApiError::storage)?;
    let items = rows
        .into_iter()
        .map(DocumentSummary::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ListDocumentsResponse { items })
}

async fn get_document(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: Value,
) -> Result<DocumentDetail, ApiError> {
    let params: GetDocumentParams = parse_params(params)?;
    validate_document_id(&params.id)?;
    let row = sqlx::query_as::<_, DocumentDetailRow>(&format!(
        r#"SELECT d.id, d.title, d.file_name, d.status, d.content,
                  {DOCUMENT_CREATED_AT} AS created_at,
                  u.id AS created_by_id, u.display_name AS created_by_name
           FROM documents d
           JOIN users u ON u.id = d.created_by
           WHERE d.workspace_id = $1 AND d.id = $2"#
    ))
    .bind(&user.workspace_id)
    .bind(&params.id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::storage)?
    .ok_or_else(ApiError::not_found)?;
    row.try_into()
}

async fn create_document(
    pool: &PgPool,
    user: &AuthenticatedUser,
    params: Value,
) -> Result<DocumentDetail, ApiError> {
    // Reject before reading params: write permission comes before input validation.
    if !matches!(user.role, UserRole::Company) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Only company members can upload documents.",
        ));
    }
    let params: CreateDocumentParams = parse_params(params)?;
    let title = validate_title(&params.title)?;
    let file_name = validate_file_name(&params.file_name)?;
    validate_content(&params.content)?;
    let row = sqlx::query_as::<_, DocumentDetailRow>(&format!(
        r#"WITH d AS (
               INSERT INTO documents (workspace_id, title, file_name, content, status, created_by)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING *
           )
           SELECT d.id, d.title, d.file_name, d.status, d.content,
                  {DOCUMENT_CREATED_AT} AS created_at,
                  u.id AS created_by_id, u.display_name AS created_by_name
           FROM d
           JOIN users u ON u.id = d.created_by"#
    ))
    .bind(&user.workspace_id)
    .bind(title)
    .bind(file_name)
    .bind(&params.content)
    .bind(DocumentStatus::Ready.as_str())
    .bind(&user.id)
    .fetch_one(pool)
    .await
    .map_err(ApiError::storage)?;
    row.try_into()
}

/// Locks evidence documents for the caller's transaction and checks they can be linked
/// (spec 2.6, D-21). Missing or other-workspace IDs are 404 (existence is not revealed),
/// then `processing`·`failed` documents are 400 (D-3). `ids` must be duplicate-free.
pub async fn lock_ready_documents<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    ids: &[String],
) -> Result<(), ApiError> {
    let statuses: Vec<String> = sqlx::query_scalar(
        "SELECT status FROM documents
         WHERE workspace_id = $1 AND id = ANY($2)
         FOR SHARE",
    )
    .bind(workspace_id)
    .bind(ids)
    .fetch_all(executor)
    .await
    .map_err(ApiError::storage)?;
    if statuses.len() != ids.len() {
        return Err(ApiError::not_found());
    }
    if statuses
        .iter()
        .any(|status| status != DocumentStatus::Ready.as_str())
    {
        return Err(ApiError::invalid("Evidence documents must be ready."));
    }
    Ok(())
}

/// Looks up documents of one workspace by ID for other modules (spec D-21).
/// IDs from other workspaces or unknown IDs are simply absent from the result.
/// Sorted like the document list: `created_at DESC, id ASC` (spec D-13).
pub async fn document_refs<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    ids: &[String],
) -> Result<Vec<DocumentRef>, ApiError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, DocumentRefRow>(
        r#"SELECT d.id, d.title, d.file_name, d.status
           FROM documents d
           WHERE d.workspace_id = $1 AND d.id = ANY($2)
           ORDER BY d.created_at DESC, d.id ASC"#,
    )
    .bind(workspace_id)
    .bind(ids)
    .fetch_all(executor)
    .await
    .map_err(ApiError::storage)?;
    rows.into_iter().map(DocumentRef::try_from).collect()
}

fn to_result<T: Serialize>(value: T) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(ApiError::storage)
}

fn parse_params<T: DeserializeOwned>(params: Value) -> Result<T, ApiError> {
    serde_json::from_value(params).map_err(|_| ApiError::invalid("Invalid params."))
}

/// Accepts `null` as "no params" (spec D-39); `{}` and known fields go through serde.
fn parse_optional_params<T: DeserializeOwned + Default>(params: Value) -> Result<T, ApiError> {
    if params.is_null() {
        Ok(T::default())
    } else {
        parse_params(params)
    }
}

fn validate_document_id(id: &str) -> Result<(), ApiError> {
    let valid = (1..=DOCUMENT_ID_MAX_CHARS).contains(&id.len())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(ApiError::invalid("Document id is invalid."))
    }
}

fn validate_title(title: &str) -> Result<String, ApiError> {
    let title = title.trim();
    if !(1..=TITLE_MAX_CHARS).contains(&title.chars().count()) {
        return Err(ApiError::invalid("Title must be 1-200 characters."));
    }
    if title.chars().any(char::is_control) {
        return Err(ApiError::invalid(
            "Title must not contain control characters.",
        ));
    }
    Ok(title.to_string())
}

fn validate_file_name(file_name: &str) -> Result<String, ApiError> {
    let file_name = file_name.trim();
    if !(1..=FILE_NAME_MAX_CHARS).contains(&file_name.chars().count()) {
        return Err(ApiError::invalid("File name must be 1-255 characters."));
    }
    if file_name
        .chars()
        .any(|c| c.is_control() || c == '/' || c == '\\')
    {
        return Err(ApiError::invalid(
            "File name must not contain path separators or control characters.",
        ));
    }
    let Some((stem, extension)) = file_name.rsplit_once('.') else {
        return Err(ApiError::invalid("File name must end with .txt or .md."));
    };
    if !extension.eq_ignore_ascii_case("txt") && !extension.eq_ignore_ascii_case("md") {
        return Err(ApiError::invalid("File name must end with .txt or .md."));
    }
    if stem.is_empty() {
        return Err(ApiError::invalid(
            "File name must have a name before the extension.",
        ));
    }
    Ok(file_name.to_string())
}

fn validate_content(content: &str) -> Result<(), ApiError> {
    if content.is_empty() {
        return Err(ApiError::invalid("Content must not be empty."));
    }
    if content.len() > CONTENT_MAX_BYTES {
        return Err(ApiError::invalid("Content must be at most 200000 bytes."));
    }
    if content.contains('\0') {
        return Err(ApiError::invalid(
            "Content must not contain NUL characters.",
        ));
    }
    Ok(())
}

/// Trimmed query → `%query%` with LIKE wildcards escaped; empty means no filter.
fn search_pattern(query: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(query) = query else {
        return Ok(None);
    };
    let query = query.trim();
    if query.is_empty() {
        return Ok(None);
    }
    if query.chars().count() > QUERY_MAX_CHARS {
        return Err(ApiError::invalid(
            "Search query must be at most 100 characters.",
        ));
    }
    let mut pattern = String::with_capacity(query.len() + 2);
    pattern.push('%');
    for c in query.chars() {
        if matches!(c, '\\' | '%' | '_') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    Ok(Some(pattern))
}
