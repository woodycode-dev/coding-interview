//! Review plugin read RPC tests against a real PostgreSQL database.
//!
//! `#[sqlx::test]` creates a fresh database per test from `DATABASE_URL`
//! and applies `api/migrations` (including the sample seed) before each test.
//! There is no save API yet, so reviews are inserted with SQL.

use axum::http::StatusCode;
use dataroom_api::{
    auth::AuthenticatedUser,
    error::ApiError,
    plugins::dispatch,
    types::{PluginRpcRequest, UserRole},
};
use serde_json::{Value, json};
use sqlx::PgPool;

fn user(id: &str, name: &str, role: UserRole) -> AuthenticatedUser {
    AuthenticatedUser {
        id: id.into(),
        name: name.into(),
        role,
        workspace_id: "lighthouse".into(),
        workspace_name: "라이트하우스".into(),
    }
}

fn company() -> AuthenticatedUser {
    user("company-user", "김서연", UserRole::Company)
}

fn investor() -> AuthenticatedUser {
    user("investor-user", "이도윤", UserRole::Investor)
}

fn peer() -> AuthenticatedUser {
    user("investor-peer", "박지우", UserRole::Investor)
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
        PluginRpcRequest {
            plugin_id: "review".into(),
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

fn field_list(list: &Value, field: &str) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|item| item[field].as_str().unwrap().to_string())
        .collect()
}

async fn insert_review(
    pool: &PgPool,
    investor_id: &str,
    criterion_id: &str,
    decision: &str,
    comment: &str,
    document_ids: &[&str],
) -> String {
    let id: String = sqlx::query_scalar(
        "INSERT INTO reviews (workspace_id, investor_id, criterion_id, decision, comment)
         VALUES ('lighthouse', $1, $2, $3, $4)
         RETURNING id",
    )
    .bind(investor_id)
    .bind(criterion_id)
    .bind(decision)
    .bind(comment)
    .fetch_one(pool)
    .await
    .unwrap();
    for document_id in document_ids {
        sqlx::query(
            "INSERT INTO review_evidence (review_id, document_id, workspace_id)
             VALUES ($1, $2, 'lighthouse')",
        )
        .bind(&id)
        .bind(document_id)
        .execute(pool)
        .await
        .unwrap();
    }
    id
}

// --- 공통 ---

#[sqlx::test]
async fn workspace_mismatch_is_forbidden(pool: PgPool) {
    for method in ["listCriteria", "listMyReviews", "getMyReview"] {
        let result = call_in(&pool, &investor(), "other", method, json!(null)).await;
        assert_eq!(status_of(result), StatusCode::FORBIDDEN, "{method}");
    }
}

#[sqlx::test]
async fn unknown_method_is_invalid(pool: PgPool) {
    let result = call(&pool, &investor(), "noSuchMethod", json!(null)).await;
    assert_eq!(status_of(result), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn no_param_methods_accept_null_and_empty_object_only(pool: PgPool) {
    for method in ["listCriteria", "listMyReviews"] {
        for params in [json!(null), json!({})] {
            let result = call(&pool, &investor(), method, params.clone()).await;
            assert_eq!(status_of(result), StatusCode::OK, "{method} {params}");
        }
        let result = call(&pool, &investor(), method, json!({ "extra": 1 })).await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{method}");
    }
}

// --- listCriteria ---

#[sqlx::test]
async fn both_roles_list_criteria_in_display_order(pool: PgPool) {
    for user in [company(), investor()] {
        let list = call(&pool, &user, "listCriteria", json!(null))
            .await
            .unwrap();
        assert_eq!(
            field_list(&list["items"], "id"),
            ["business", "team", "revenue"]
        );
        let first = &list["items"][0];
        assert_eq!(first["title"], "사업 이해");
        assert!(
            first["reviewQuestion"]
                .as_str()
                .unwrap()
                .contains("사업 모델")
        );
        assert_eq!(first["displayOrder"], 1);
    }
}

// --- 권한 ---

#[sqlx::test]
async fn company_gets_empty_review_list_even_with_invalid_params(pool: PgPool) {
    insert_review(
        &pool,
        "investor-user",
        "business",
        "satisfied",
        "확인",
        &["doc-business"],
    )
    .await;
    for params in [json!(null), json!({ "extra": 1 })] {
        let list = call(&pool, &company(), "listMyReviews", params)
            .await
            .unwrap();
        assert_eq!(list, json!({ "items": [] }));
    }
}

#[sqlx::test]
async fn company_get_my_review_is_forbidden_even_with_invalid_input(pool: PgPool) {
    for params in [
        json!({ "criterionId": "business" }),
        json!({ "bad": "../x" }),
    ] {
        let result = call(&pool, &company(), "getMyReview", params).await;
        assert_eq!(status_of(result), StatusCode::FORBIDDEN);
    }
}

#[sqlx::test]
async fn investors_see_only_their_own_reviews(pool: PgPool) {
    insert_review(
        &pool,
        "investor-user",
        "business",
        "satisfied",
        "내 검토",
        &["doc-business"],
    )
    .await;
    insert_review(
        &pool,
        "investor-peer",
        "team",
        "needs_information",
        "동료 검토",
        &["doc-team"],
    )
    .await;

    let mine = call(&pool, &investor(), "listMyReviews", json!(null))
        .await
        .unwrap();
    assert_eq!(field_list(&mine["items"], "criterionId"), ["business"]);
    assert_eq!(mine["items"][0]["comment"], "내 검토");

    let peer_criterion = call(
        &pool,
        &investor(),
        "getMyReview",
        json!({ "criterionId": "team" }),
    )
    .await
    .unwrap();
    assert_eq!(peer_criterion, Value::Null);

    let peer_own = call(
        &pool,
        &peer(),
        "getMyReview",
        json!({ "criterionId": "team" }),
    )
    .await
    .unwrap();
    assert_eq!(peer_own["comment"], "동료 검토");
}

// --- getMyReview 입력·오류 ---

#[sqlx::test]
async fn unwritten_review_is_null(pool: PgPool) {
    let review = call(
        &pool,
        &investor(),
        "getMyReview",
        json!({ "criterionId": "revenue" }),
    )
    .await
    .unwrap();
    assert_eq!(review, Value::Null);
}

#[sqlx::test]
async fn unknown_criterion_is_not_found(pool: PgPool) {
    let result = call(
        &pool,
        &investor(),
        "getMyReview",
        json!({ "criterionId": "unknown" }),
    )
    .await;
    assert_eq!(status_of(result), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn malformed_criterion_params_are_invalid(pool: PgPool) {
    let too_long = "a".repeat(101);
    for params in [
        json!({}),
        json!(null),
        json!({ "criterionId": "" }),
        json!({ "criterionId": "a b" }),
        json!({ "criterionId": "../team" }),
        json!({ "criterionId": too_long }),
        json!({ "criterionId": 1 }),
        json!({ "criterionId": "team", "extra": 1 }),
    ] {
        let result = call(&pool, &investor(), "getMyReview", params.clone()).await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{params}");
    }
    let max = "a".repeat(100);
    let result = call(
        &pool,
        &investor(),
        "getMyReview",
        json!({ "criterionId": max }),
    )
    .await;
    assert_eq!(status_of(result), StatusCode::NOT_FOUND);
}

// --- 응답 형태 ---

#[sqlx::test]
async fn review_has_fields_and_evidence_in_document_order(pool: PgPool) {
    // Seed created_at: business 09:00 < team 09:01 < revenue(failed) 09:02.
    let id = insert_review(
        &pool,
        "investor-user",
        "business",
        "needs_information",
        "매출 근거 보완 필요",
        &["doc-business", "doc-revenue", "doc-team"],
    )
    .await;

    let review = call(
        &pool,
        &investor(),
        "getMyReview",
        json!({ "criterionId": "business" }),
    )
    .await
    .unwrap();
    assert_eq!(review["id"], id);
    assert_eq!(review["criterionId"], "business");
    assert_eq!(review["decision"], "needs_information");
    assert_eq!(review["comment"], "매출 근거 보완 필요");
    assert!(review["createdAt"].as_str().unwrap().ends_with('Z'));
    assert!(review["updatedAt"].as_str().unwrap().ends_with('Z'));
    assert_eq!(
        field_list(&review["evidence"], "documentId"),
        ["doc-revenue", "doc-team", "doc-business"]
    );
    // A document that became `failed` after saving still shows with its current status.
    assert_eq!(
        review["evidence"][0],
        json!({ "documentId": "doc-revenue", "title": "매출 자료", "fileName": "revenue.txt", "status": "failed" })
    );
}

#[sqlx::test]
async fn reviews_follow_criterion_order_and_keep_their_own_evidence(pool: PgPool) {
    insert_review(
        &pool,
        "investor-user",
        "revenue",
        "satisfied",
        "매출",
        &["doc-revenue"],
    )
    .await;
    insert_review(
        &pool,
        "investor-user",
        "business",
        "satisfied",
        "사업",
        &["doc-business", "doc-team"],
    )
    .await;

    let list = call(&pool, &investor(), "listMyReviews", json!({}))
        .await
        .unwrap();
    let items = &list["items"];
    assert_eq!(field_list(items, "criterionId"), ["business", "revenue"]);
    assert_eq!(
        field_list(&items[0]["evidence"], "documentId"),
        ["doc-team", "doc-business"]
    );
    assert_eq!(
        field_list(&items[1]["evidence"], "documentId"),
        ["doc-revenue"]
    );
}
