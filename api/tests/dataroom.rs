//! Dataroom RPC tests against a real PostgreSQL database.
//!
//! `#[sqlx::test]` creates a fresh database per test from `DATABASE_URL`
//! and applies `api/migrations` (including the sample seed) before each test.

use axum::http::StatusCode;
use dataroom_api::{
    auth::AuthenticatedUser,
    dataroom::dispatch,
    error::ApiError,
    types::{DataroomRpcRequest, UserRole},
};
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection, PgPool};

fn company() -> AuthenticatedUser {
    AuthenticatedUser {
        id: "company-user".into(),
        name: "김서연".into(),
        role: UserRole::Company,
        workspace_id: "lighthouse".into(),
        workspace_name: "라이트하우스".into(),
    }
}

fn investor() -> AuthenticatedUser {
    AuthenticatedUser {
        id: "investor-user".into(),
        name: "이도윤".into(),
        role: UserRole::Investor,
        workspace_id: "lighthouse".into(),
        workspace_name: "라이트하우스".into(),
    }
}

async fn call_in(
    pool: &PgPool,
    user: &AuthenticatedUser,
    workspace_id: &str,
    method: &str,
    params: Value,
) -> Result<Value, ApiError> {
    dispatch(
        pool,
        user,
        DataroomRpcRequest {
            workspace_id: workspace_id.into(),
            method: method.into(),
            params,
        },
    )
    .await
    .map(|response| response.result)
}

async fn call(
    pool: &PgPool,
    user: &AuthenticatedUser,
    method: &str,
    params: Value,
) -> Result<Value, ApiError> {
    call_in(pool, user, "lighthouse", method, params).await
}

fn status_of(result: Result<Value, ApiError>) -> StatusCode {
    match result {
        Ok(_) => StatusCode::OK,
        Err(error) => error.0,
    }
}

async fn create(
    pool: &PgPool,
    title: &str,
    file_name: &str,
    content: &str,
) -> Result<Value, ApiError> {
    call(
        pool,
        &company(),
        "createDocument",
        json!({ "title": title, "fileName": file_name, "content": content }),
    )
    .await
}

fn ids(list: &Value) -> Vec<String> {
    list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_string())
        .collect()
}

async fn add_other_room(pool: &PgPool) {
    sqlx::query("INSERT INTO workspaces (id, name) VALUES ('other', '다른 룸')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO documents (id, workspace_id, title, file_name, content, status, created_by)
         VALUES ('doc-other', 'other', '다른 룸 자료', 'other.md', '비공개', 'ready', 'company-user')",
    )
    .execute(pool)
    .await
    .unwrap();
}

// --- 권한 ---

#[sqlx::test]
async fn workspace_mismatch_is_forbidden(pool: PgPool) {
    add_other_room(&pool).await;
    for user in [company(), investor()] {
        for method in [
            "getDataroom",
            "listDocuments",
            "getDocument",
            "createDocument",
        ] {
            let result = call_in(&pool, &user, "other", method, json!({ "id": "doc-other" })).await;
            assert_eq!(status_of(result), StatusCode::FORBIDDEN, "{method}");
        }
    }
}

#[sqlx::test]
async fn investor_cannot_create_document_even_with_invalid_input(pool: PgPool) {
    for params in [
        json!({ "title": "투자자 자료", "fileName": "a.md", "content": "본문" }),
        json!({ "title": "", "fileName": "a.pdf", "content": "" }),
        json!(null),
        json!({ "unknown": true }),
    ] {
        let result = call(&pool, &investor(), "createDocument", params).await;
        assert_eq!(status_of(result), StatusCode::FORBIDDEN);
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 4, "only the seed documents remain");
}

#[sqlx::test]
async fn investor_can_read_documents(pool: PgPool) {
    let list = call(&pool, &investor(), "listDocuments", json!(null))
        .await
        .unwrap();
    assert_eq!(ids(&list).len(), 4);
    let detail = call(
        &pool,
        &investor(),
        "getDocument",
        json!({ "id": "doc-team" }),
    )
    .await
    .unwrap();
    assert_eq!(detail["title"], "팀 소개");
}

// --- 404 / 400 구분 ---

#[sqlx::test]
async fn missing_and_other_room_documents_are_not_found(pool: PgPool) {
    add_other_room(&pool).await;
    for id in ["doc-missing", "doc-other"] {
        let result = call(&pool, &company(), "getDocument", json!({ "id": id })).await;
        assert_eq!(status_of(result), StatusCode::NOT_FOUND, "{id}");
    }
    let list = call(&pool, &company(), "listDocuments", json!({}))
        .await
        .unwrap();
    assert!(!ids(&list).contains(&"doc-other".to_string()));
}

#[sqlx::test]
async fn malformed_document_id_is_invalid(pool: PgPool) {
    let too_long = "a".repeat(101);
    for id in ["", "doc team", "doc_team", "문서", too_long.as_str()] {
        let result = call(&pool, &company(), "getDocument", json!({ "id": id })).await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{id:?}");
    }
    let exactly_100 = "a".repeat(100);
    let result = call(
        &pool,
        &company(),
        "getDocument",
        json!({ "id": exactly_100 }),
    )
    .await;
    assert_eq!(status_of(result), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn unknown_method_is_invalid(pool: PgPool) {
    let result = call(&pool, &company(), "deleteDocument", json!(null)).await;
    let error = result.unwrap_err();
    assert_eq!(error.0, StatusCode::BAD_REQUEST);
    assert_eq!(error.1, "invalid_input");
}

#[sqlx::test]
async fn optional_params_accept_null_and_empty_object_only(pool: PgPool) {
    for method in ["getDataroom", "listDocuments"] {
        for params in [json!(null), json!({})] {
            assert_eq!(
                status_of(call(&pool, &company(), method, params).await),
                StatusCode::OK
            );
        }
        let result = call(&pool, &company(), method, json!({ "q": "x" })).await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{method}");
    }
}

#[sqlx::test]
async fn get_dataroom_returns_description(pool: PgPool) {
    let room = call(&pool, &investor(), "getDataroom", json!(null))
        .await
        .unwrap();
    assert_eq!(room["id"], "lighthouse");
    assert_eq!(room["name"], "라이트하우스");
    assert_eq!(
        room["description"],
        "소규모 제조사를 위한 재고 관리 소프트웨어"
    );
}

// --- 입력 경계값 ---

#[sqlx::test]
async fn title_length_boundary(pool: PgPool) {
    assert!(
        create(&pool, &"가".repeat(200), "a.md", "본문")
            .await
            .is_ok()
    );
    let result = create(&pool, &"가".repeat(201), "a.md", "본문").await;
    assert_eq!(status_of(result), StatusCode::BAD_REQUEST);
    for title in ["", "   ", "\u{3000}", "탭\t제목"] {
        let result = create(&pool, title, "a.md", "본문").await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{title:?}");
    }
}

#[sqlx::test]
async fn content_size_boundary(pool: PgPool) {
    assert!(
        create(&pool, "최대 본문", "a.txt", &"a".repeat(200_000))
            .await
            .is_ok()
    );
    let result = create(&pool, "초과 본문", "a.txt", &"a".repeat(200_001)).await;
    assert_eq!(status_of(result), StatusCode::BAD_REQUEST);
    // 한글 1자 = UTF-8 3바이트: 66,667자 = 200,001바이트
    let result = create(&pool, "초과 한글", "a.txt", &"가".repeat(66_667)).await;
    assert_eq!(status_of(result), StatusCode::BAD_REQUEST);
    for content in ["", "앞\0뒤"] {
        let result = create(&pool, "본문 오류", "a.txt", content).await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{content:?}");
    }
}

#[sqlx::test]
async fn file_name_rules(pool: PgPool) {
    for name in ["notes.MD", "notes.Txt", "보고서.md"] {
        assert!(
            create(&pool, "파일명 허용", name, "본문").await.is_ok(),
            "{name}"
        );
    }
    let too_long = format!("{}.md", "a".repeat(253));
    for name in [
        "report.pdf",
        ".md",
        "dir/a.md",
        "dir\\a.md",
        "noext",
        "",
        too_long.as_str(),
    ] {
        let result = create(&pool, "파일명 거부", name, "본문").await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{name:?}");
    }
    assert!(
        create(
            &pool,
            "파일명 255자",
            &format!("{}.md", "a".repeat(252)),
            "본문"
        )
        .await
        .is_ok()
    );
}

#[sqlx::test]
async fn title_and_file_name_are_stored_trimmed(pool: PgPool) {
    let created = create(&pool, "  매출 보완  ", "  revenue-update.md ", "본문")
        .await
        .unwrap();
    assert_eq!(created["title"], "매출 보완");
    assert_eq!(created["fileName"], "revenue-update.md");
    let (title, file_name): (String, String) =
        sqlx::query_as("SELECT title, file_name FROM documents WHERE id = $1")
            .bind(created["id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        (title.as_str(), file_name.as_str()),
        ("매출 보완", "revenue-update.md")
    );
}

#[sqlx::test]
async fn unknown_create_fields_are_invalid(pool: PgPool) {
    let result = call(
        &pool,
        &company(),
        "createDocument",
        json!({ "title": "t", "fileName": "a.md", "content": "c", "status": "failed" }),
    )
    .await;
    assert_eq!(status_of(result), StatusCode::BAD_REQUEST);
}

// --- 등록·조회 ---

#[sqlx::test]
async fn created_document_is_ready_and_persisted(pool: PgPool) {
    let created = create(
        &pool,
        "매출 현황 보완",
        "revenue-update.md",
        "# 매출\n600만 원",
    )
    .await
    .unwrap();
    assert_eq!(created["status"], "ready");
    assert_eq!(created["createdBy"]["id"], "company-user");
    assert_eq!(created["createdBy"]["name"], "김서연");
    let id = created["id"].as_str().unwrap();

    let detail = call(&pool, &investor(), "getDocument", json!({ "id": id }))
        .await
        .unwrap();
    assert_eq!(detail["content"], "# 매출\n600만 원");
    assert_eq!(detail["fileName"], "revenue-update.md");

    let list = call(&pool, &company(), "listDocuments", json!(null))
        .await
        .unwrap();
    assert_eq!(ids(&list)[0], id, "newest document comes first");
}

#[sqlx::test]
async fn list_order_is_created_at_desc_then_id_asc(pool: PgPool) {
    let list = call(&pool, &company(), "listDocuments", json!(null))
        .await
        .unwrap();
    assert_eq!(
        ids(&list),
        ["doc-pipeline", "doc-revenue", "doc-team", "doc-business"]
    );
    assert_eq!(list["items"][0]["createdAt"], "2026-09-01T09:03:00.000Z");

    sqlx::query(
        "INSERT INTO documents (id, workspace_id, title, file_name, content, status, created_by, created_at)
         VALUES ('tie-b', 'lighthouse', '동시 B', 'b.md', 'b', 'ready', 'company-user', '2026-09-02 00:00:00+00'),
                ('tie-a', 'lighthouse', '동시 A', 'a.md', 'a', 'ready', 'company-user', '2026-09-02 00:00:00+00')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let list = call(&pool, &company(), "listDocuments", json!(null))
        .await
        .unwrap();
    assert_eq!(ids(&list)[..2], ["tie-a", "tie-b"]);
}

// --- 검색 ---

#[sqlx::test]
async fn search_matches_title_substring(pool: PgPool) {
    let list = call(
        &pool,
        &company(),
        "listDocuments",
        json!({ "query": " 소개 " }),
    )
    .await
    .unwrap();
    assert_eq!(ids(&list), ["doc-team", "doc-business"]);
    let list = call(
        &pool,
        &company(),
        "listDocuments",
        json!({ "query": "없는 제목" }),
    )
    .await
    .unwrap();
    assert!(ids(&list).is_empty());
    let list = call(
        &pool,
        &company(),
        "listDocuments",
        json!({ "query": "   " }),
    )
    .await
    .unwrap();
    assert_eq!(ids(&list).len(), 4, "blank query means no filter");
}

#[sqlx::test]
async fn search_escapes_like_wildcards(pool: PgPool) {
    let percent = create(&pool, "달성률 100% 보고", "p.md", "본문")
        .await
        .unwrap();
    create(&pool, "달성률 1000 보고", "q.md", "본문")
        .await
        .unwrap();
    let underscore = create(&pool, "snake_case 메모", "u.md", "본문")
        .await
        .unwrap();
    create(&pool, "snakeXcase 메모", "v.md", "본문")
        .await
        .unwrap();

    let list = call(&pool, &company(), "listDocuments", json!({ "query": "%" }))
        .await
        .unwrap();
    assert_eq!(ids(&list), [percent["id"].as_str().unwrap()]);
    let list = call(
        &pool,
        &company(),
        "listDocuments",
        json!({ "query": "100%" }),
    )
    .await
    .unwrap();
    assert_eq!(ids(&list), [percent["id"].as_str().unwrap()]);
    let list = call(&pool, &company(), "listDocuments", json!({ "query": "_" }))
        .await
        .unwrap();
    assert_eq!(ids(&list), [underscore["id"].as_str().unwrap()]);
}

#[sqlx::test]
async fn search_query_length_boundary(pool: PgPool) {
    let ok = call(
        &pool,
        &company(),
        "listDocuments",
        json!({ "query": "가".repeat(100) }),
    )
    .await;
    assert_eq!(status_of(ok), StatusCode::OK);
    let too_long = call(
        &pool,
        &company(),
        "listDocuments",
        json!({ "query": "가".repeat(101) }),
    )
    .await;
    assert_eq!(status_of(too_long), StatusCode::BAD_REQUEST);
}

// --- 9단계 보강 (spec 6.1) ---

/// A connection opened outside the test pool, so reads see only committed data.
async fn fresh_connection(pool: &PgPool) -> PgConnection {
    PgConnection::connect_with(&pool.connect_options())
        .await
        .unwrap()
}

#[sqlx::test]
async fn created_document_is_committed(pool: PgPool) {
    let created = create(&pool, "커밋 확인", "commit.md", "본문 확인")
        .await
        .unwrap();
    let mut conn = fresh_connection(&pool).await;
    let row: (String, String, String, String) =
        sqlx::query_as("SELECT title, file_name, content, status FROM documents WHERE id = $1")
            .bind(created["id"].as_str().unwrap())
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(
        row,
        (
            "커밋 확인".into(),
            "commit.md".into(),
            "본문 확인".into(),
            "ready".into()
        )
    );
}

#[sqlx::test]
async fn title_and_content_minimum_and_emoji_length(pool: PgPool) {
    assert!(create(&pool, "가", "a.md", "a").await.is_ok());
    // Code points, not UTF-16 units: an emoji is one character (D-24).
    assert!(
        create(&pool, &"😀".repeat(200), "a.md", "본문")
            .await
            .is_ok()
    );
    let result = create(&pool, &"😀".repeat(201), "a.md", "본문").await;
    assert_eq!(status_of(result), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn evidence_lock_blocks_status_change_until_commit(pool: PgPool) {
    // Hold the same lock saveReview takes on evidence documents (spec 2.6, D-21).
    let mut holder = pool.begin().await.unwrap();
    dataroom_api::dataroom::lock_ready_documents(
        &mut *holder,
        "lighthouse",
        &["doc-business".to_string()],
    )
    .await
    .unwrap();

    let mut other = fresh_connection(&pool).await;
    sqlx::query("SET lock_timeout = '200ms'")
        .execute(&mut other)
        .await
        .unwrap();
    let blocked = sqlx::query("UPDATE documents SET status = 'failed' WHERE id = 'doc-business'")
        .execute(&mut other)
        .await
        .unwrap_err();
    let code = blocked.as_database_error().and_then(|e| e.code());
    assert_eq!(
        code.as_deref(),
        Some("55P03"),
        "expected lock_not_available"
    );

    holder.rollback().await.unwrap();
    sqlx::query("UPDATE documents SET status = 'failed' WHERE id = 'doc-business'")
        .execute(&mut other)
        .await
        .unwrap();
}
