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
    for method in [
        "listCriteria",
        "listMyReviews",
        "getMyReview",
        "saveReview",
        "getMyProgress",
    ] {
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

// --- saveReview ---

async fn save(
    pool: &PgPool,
    user: &AuthenticatedUser,
    criterion_id: &str,
    decision: &str,
    comment: &str,
    document_ids: &[&str],
) -> Result<Value, ApiError> {
    call(
        pool,
        user,
        "saveReview",
        json!({
            "criterionId": criterion_id,
            "decision": decision,
            "comment": comment,
            "evidenceDocumentIds": document_ids,
        }),
    )
    .await
}

async fn count(pool: &PgPool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

async fn evidence_of(pool: &PgPool, criterion_id: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT e.document_id FROM review_evidence e
         JOIN reviews r ON r.id = e.review_id
         WHERE r.investor_id = 'investor-user' AND r.criterion_id = $1
         ORDER BY e.document_id",
    )
    .bind(criterion_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn add_ready_document(pool: &PgPool, workspace_id: &str, id: &str) {
    sqlx::query(
        "INSERT INTO documents (id, workspace_id, title, file_name, content, status, created_by)
         VALUES ($1, $2, $1, 'evidence.md', '본문', 'ready', 'company-user')",
    )
    .bind(id)
    .bind(workspace_id)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test]
async fn company_cannot_save_even_with_invalid_input(pool: PgPool) {
    for params in [
        json!({}),
        json!({ "criterionId": "business", "decision": "satisfied", "comment": "확인", "evidenceDocumentIds": ["doc-business"] }),
    ] {
        let result = call(&pool, &company(), "saveReview", params).await;
        assert_eq!(status_of(result), StatusCode::FORBIDDEN);
    }
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM reviews").await, 0);
}

#[sqlx::test]
async fn save_creates_and_persists_review(pool: PgPool) {
    let saved = save(
        &pool,
        &investor(),
        "business",
        "satisfied",
        "  사업 모델 확인  ",
        &["doc-business", "doc-team"],
    )
    .await
    .unwrap();
    assert_eq!(saved["criterionId"], "business");
    assert_eq!(saved["decision"], "satisfied");
    assert_eq!(saved["comment"], "사업 모델 확인");
    assert_eq!(
        field_list(&saved["evidence"], "documentId"),
        ["doc-team", "doc-business"]
    );

    // Read back through a separate request (another pooled connection).
    let loaded = call(
        &pool,
        &investor(),
        "getMyReview",
        json!({ "criterionId": "business" }),
    )
    .await
    .unwrap();
    assert_eq!(loaded, saved);
    let stored: (String, String) =
        sqlx::query_as("SELECT decision, comment FROM reviews WHERE investor_id = 'investor-user'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, ("satisfied".into(), "사업 모델 확인".into()));
}

#[sqlx::test]
async fn resave_keeps_review_identity_and_replaces_evidence(pool: PgPool) {
    let first = save(
        &pool,
        &investor(),
        "team",
        "satisfied",
        "처음",
        &["doc-business", "doc-team"],
    )
    .await
    .unwrap();
    let second = save(
        &pool,
        &investor(),
        "team",
        "needs_information",
        "수정",
        &["doc-team"],
    )
    .await
    .unwrap();
    assert_eq!(second["id"], first["id"]);
    assert_eq!(second["createdAt"], first["createdAt"]);
    assert!(second["updatedAt"].as_str().unwrap() >= first["updatedAt"].as_str().unwrap());
    assert_eq!(second["decision"], "needs_information");
    assert_eq!(second["comment"], "수정");
    assert_eq!(field_list(&second["evidence"], "documentId"), ["doc-team"]);
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM reviews").await, 1);
    assert_eq!(evidence_of(&pool, "team").await, ["doc-team"]);
}

#[sqlx::test]
async fn comment_rules(pool: PgPool) {
    let max = "가".repeat(2000);
    let emoji_max = "😀".repeat(2000);
    let padded_max = format!("  {max}\n");
    for comment in [
        "가",
        "줄1\n\t줄2",
        max.as_str(),
        emoji_max.as_str(),
        padded_max.as_str(),
    ] {
        let result = save(
            &pool,
            &investor(),
            "business",
            "satisfied",
            comment,
            &["doc-business"],
        )
        .await;
        assert_eq!(
            status_of(result),
            StatusCode::OK,
            "{} chars",
            comment.chars().count()
        );
    }
    let over = "가".repeat(2001);
    let emoji_over = "😀".repeat(2001);
    for comment in [
        "",
        "   ",
        "\u{3000}\u{3000}",
        "\n\t",
        over.as_str(),
        emoji_over.as_str(),
        "a\u{0}b",
        "a\u{7}b",
    ] {
        let result = save(
            &pool,
            &investor(),
            "business",
            "satisfied",
            comment,
            &["doc-business"],
        )
        .await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{comment:?}");
    }
}

#[sqlx::test]
async fn evidence_id_rules(pool: PgPool) {
    let twenty_one: Vec<String> = (1..=21).map(|n| format!("doc-{n}")).collect();
    let twenty_one: Vec<&str> = twenty_one.iter().map(String::as_str).collect();
    for ids in [
        vec![],
        vec!["doc-business", "doc-business"],
        vec!["doc business"],
        twenty_one,
    ] {
        let result = save(&pool, &investor(), "business", "satisfied", "확인", &ids).await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{ids:?}");
    }
}

#[sqlx::test]
async fn invalid_decision_and_fields_are_invalid(pool: PgPool) {
    for params in [
        json!({ "criterionId": "business", "decision": "approved", "comment": "확인", "evidenceDocumentIds": ["doc-business"] }),
        json!({ "criterionId": "business", "decision": "satisfied", "comment": "확인" }),
        json!({ "criterionId": "business", "decision": "satisfied", "comment": "확인", "evidenceDocumentIds": ["doc-business"], "investorId": "investor-peer" }),
        json!({ "criterionId": "a b", "decision": "satisfied", "comment": "확인", "evidenceDocumentIds": ["doc-business"] }),
    ] {
        let result = call(&pool, &investor(), "saveReview", params.clone()).await;
        assert_eq!(status_of(result), StatusCode::BAD_REQUEST, "{params}");
    }
}

#[sqlx::test]
async fn missing_targets_are_not_found_and_unready_documents_invalid(pool: PgPool) {
    sqlx::query("INSERT INTO workspaces (id, name) VALUES ('other', '다른 룸')")
        .execute(&pool)
        .await
        .unwrap();
    add_ready_document(&pool, "other", "doc-other").await;
    let cases: [(&str, &[&str], StatusCode); 7] = [
        ("unknown", &["doc-business"], StatusCode::NOT_FOUND),
        ("business", &["doc-missing"], StatusCode::NOT_FOUND),
        ("business", &["doc-other"], StatusCode::NOT_FOUND),
        ("business", &["doc-pipeline"], StatusCode::BAD_REQUEST),
        ("business", &["doc-revenue"], StatusCode::BAD_REQUEST),
        // Missing targets win over not-ready documents, whatever the order.
        (
            "business",
            &["doc-pipeline", "doc-missing"],
            StatusCode::NOT_FOUND,
        ),
        ("unknown", &["doc-revenue"], StatusCode::NOT_FOUND),
    ];
    for (criterion_id, ids, expected) in cases {
        let result = save(&pool, &investor(), criterion_id, "satisfied", "확인", ids).await;
        assert_eq!(status_of(result), expected, "{criterion_id} {ids:?}");
    }
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM reviews").await, 0);
}

#[sqlx::test]
async fn failed_save_rolls_back_to_previous_review(pool: PgPool) {
    save(
        &pool,
        &investor(),
        "business",
        "satisfied",
        "원래 의견",
        &["doc-business"],
    )
    .await
    .unwrap();
    // Test-only trigger (D-31): fail the evidence INSERT after the review row was
    // updated and the old evidence deleted in the same transaction.
    sqlx::query(
        "CREATE FUNCTION fail_evidence() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'injected failure'; END $$",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_evidence BEFORE INSERT ON review_evidence
         FOR EACH ROW WHEN (NEW.document_id = 'doc-team') EXECUTE FUNCTION fail_evidence()",
    )
    .execute(&pool)
    .await
    .unwrap();

    let result = save(
        &pool,
        &investor(),
        "business",
        "needs_information",
        "바뀐 의견",
        &["doc-team"],
    )
    .await;
    assert_eq!(status_of(result), StatusCode::INTERNAL_SERVER_ERROR);

    let review = call(
        &pool,
        &investor(),
        "getMyReview",
        json!({ "criterionId": "business" }),
    )
    .await
    .unwrap();
    assert_eq!(review["decision"], "satisfied");
    assert_eq!(review["comment"], "원래 의견");
    assert_eq!(evidence_of(&pool, "business").await, ["doc-business"]);
}

#[sqlx::test]
async fn concurrent_saves_keep_one_review_with_one_requests_evidence(pool: PgPool) {
    for n in 1..=4 {
        add_ready_document(&pool, "lighthouse", &format!("doc-c{n}")).await;
    }
    let sets: Vec<Vec<String>> = vec![
        vec!["doc-business".into()],
        vec!["doc-team".into(), "doc-c1".into()],
        vec!["doc-c1".into(), "doc-c2".into(), "doc-c3".into()],
        vec!["doc-c4".into()],
        vec!["doc-business".into(), "doc-team".into(), "doc-c4".into()],
        vec!["doc-c2".into()],
        vec!["doc-c3".into(), "doc-business".into()],
        vec!["doc-team".into()],
    ];
    let tasks: Vec<_> = sets
        .iter()
        .enumerate()
        .map(|(n, ids)| {
            let pool = pool.clone();
            let params = json!({
                "criterionId": "revenue",
                "decision": "satisfied",
                "comment": format!("동시 저장 {n}"),
                "evidenceDocumentIds": ids,
            });
            tokio::spawn(async move { call(&pool, &investor(), "saveReview", params).await })
        })
        .collect();
    for task in tasks {
        assert_eq!(status_of(task.await.unwrap()), StatusCode::OK);
    }

    assert_eq!(count(&pool, "SELECT COUNT(*) FROM reviews").await, 1);
    let stored = evidence_of(&pool, "revenue").await;
    let matches_one_request = sets.iter().any(|ids| {
        let mut ids = ids.clone();
        ids.sort();
        ids == stored
    });
    assert!(matches_one_request, "mixed evidence: {stored:?}");
    // The stored comment belongs to the same request as the stored evidence.
    let comment: String = sqlx::query_scalar("SELECT comment FROM reviews")
        .fetch_one(&pool)
        .await
        .unwrap();
    let n: usize = comment.trim_start_matches("동시 저장 ").parse().unwrap();
    let mut expected = sets[n].clone();
    expected.sort();
    assert_eq!(stored, expected);
}

// --- getMyProgress ---

#[sqlx::test]
async fn progress_with_no_reviews_is_all_unwritten_despite_documents(pool: PgPool) {
    // Seed documents exist, but progress comes from reviews only (README).
    let progress = call(&pool, &investor(), "getMyProgress", json!(null))
        .await
        .unwrap();
    assert_eq!(progress["totalCount"], 3);
    assert_eq!(progress["writtenCount"], 0);
    assert_eq!(progress["unwrittenCount"], 3);
    assert_eq!(progress["satisfiedCount"], 0);
    assert_eq!(progress["needsInformationCount"], 0);
    let items = progress["items"].as_array().unwrap();
    let criteria: Vec<&str> = items
        .iter()
        .map(|item| item["criterion"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(criteria, ["business", "team", "revenue"]);
    assert!(items.iter().all(|item| item["review"].is_null()));
}

#[sqlx::test]
async fn needs_information_counts_as_written_but_not_satisfied(pool: PgPool) {
    save(
        &pool,
        &investor(),
        "business",
        "satisfied",
        "확인",
        &["doc-business"],
    )
    .await
    .unwrap();
    save(
        &pool,
        &investor(),
        "revenue",
        "needs_information",
        "보완 필요",
        &["doc-team"],
    )
    .await
    .unwrap();
    // Another investor's review must not change these counts.
    save(&pool, &peer(), "team", "satisfied", "동료", &["doc-team"])
        .await
        .unwrap();

    let progress = call(&pool, &investor(), "getMyProgress", json!({}))
        .await
        .unwrap();
    assert_eq!(progress["totalCount"], 3);
    assert_eq!(progress["writtenCount"], 2);
    assert_eq!(progress["unwrittenCount"], 1);
    assert_eq!(progress["satisfiedCount"], 1);
    assert_eq!(progress["needsInformationCount"], 1);
    let items = &progress["items"];
    assert_eq!(items[0]["review"]["decision"], "satisfied");
    assert!(items[1]["review"].is_null());
    assert_eq!(items[2]["review"]["decision"], "needs_information");
    assert_eq!(
        field_list(&items[2]["review"]["evidence"], "documentId"),
        ["doc-team"]
    );
}

#[sqlx::test]
async fn company_progress_is_forbidden_even_with_invalid_params(pool: PgPool) {
    for params in [json!(null), json!({ "extra": 1 })] {
        let result = call(&pool, &company(), "getMyProgress", params).await;
        assert_eq!(status_of(result), StatusCode::FORBIDDEN);
    }
}

#[sqlx::test]
async fn progress_rejects_unknown_params(pool: PgPool) {
    let result = call(&pool, &investor(), "getMyProgress", json!({ "extra": 1 })).await;
    assert_eq!(status_of(result), StatusCode::BAD_REQUEST);
}
