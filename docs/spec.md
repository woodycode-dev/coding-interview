# 설계 (spec)

> 상태: **D-1~D-62 확정** (2026-10-03).
> `[README]` = README에 정해진 규칙, `[스켈레톤]` = 제공 코드가 이미 정한 동작, `[프로젝트 규칙]` = CLAUDE.md 규칙.
> 각 결정 항목은 "결정: …(이유)"로 적고, 선택하지 않은 선택지는 "대안: …" 한 줄만 남긴다.

---

## 범위

- [README] "제외 범위"(회원가입·OAuth, workspace·데이터룸 생성·삭제, 자료 삭제, 검토 기준 생성·수정, Plugin 설치 관리 UI, 실제 파일 스토리지, PDF·OCR, 비동기 처리·자동 상태 전환 등)는 구현하지 않는다.
- 선택 구현 **AI 검토 초안**은 구현하지 않는다.

결정: **선택 구현(AI 검토 초안) 제외** ([README] 이 기능은 가산점이 없고 필수 기능의 미완료를 대체하지 않으므로 필수 기능 완성도를 우선한다.)
- 대안: 필수 기능 완료 후 남는 시간에 구현 — 자료와 판단의 연결을 보여줄 수 있지만 모델 설정·검증 방법·재현 문서가 추가로 필요하다.

---

## 0. 전제 (스켈레톤에서 확인한 사실)

- [README] workspace 하나 = 데이터룸 하나. 구현 대상은 `lighthouse`. 요청·저장 데이터의 `workspaceId`가 범위를 결정한다.
- [스켈레톤] 업무 API는 REST가 아니라 **RPC 엔드포인트 두 개**로 들어온다.
  - 본체: `POST /api/dataroom/rpc` — body `{ workspaceId, method, params }` → `api/src/dataroom/mod.rs::dispatch`
  - Plugin: `POST /api/plugins/rpc` — body `{ pluginId, workspaceId, method, params }` → `plugins/review/server/mod.rs::dispatch`
  - 응답은 `{ result: unknown }`(`RpcResponse`). `result`의 실제 타입은 Rust DTO에 `ts-bridge`를 붙여 Gen-TS로 만든다(`ReviewHealthResponse` 예제와 동일).
  - Plugin UI는 `host.call(method, params)`(Plugin API) 또는 `host.call(method, params, { target: "dataroom" })`(본체 API)로 호출한다.
- [스켈레톤] 두 엔드포인트 모두 `CurrentUser` 추출기를 거치므로 미인증은 dispatch 전에 `401 not_authenticated`.
- [스켈레톤] 사용자 1명은 workspace 1개에만 매핑된다(`auth.rs`의 `ORDER BY w.id LIMIT 1`). `AuthenticatedUser { id, name, role, workspace_id, workspace_name }`만 신뢰한다.
- [스켈레톤] 전체 요청 본문 제한 256KiB(`DefaultBodyLimit::max(256 * 1024)`, `api/src/lib.rs`).
- [스켈레톤] 오류 본문은 `{ kind, message }`. 이미 있는 종류:

  | kind | HTTP | 생성 함수 |
  |---|---|---|
  | `invalid_input` | 400 | `ApiError::invalid(msg)` |
  | `not_authenticated` | 401 | `ApiError::unauthorized()` |
  | `forbidden` | 403 | `ApiError::forbidden()` |
  | `not_found` | 404 | `ApiError::not_found()` |
  | `storage_error` | 500 | `ApiError::storage(e)` |
  | `assignment_pending` | 501 | `ApiError::not_implemented(..)` |

  `message`는 `&'static str`라 요청마다 달라지는 값(필드명, 문제 ID 등)을 담을 수 없다.
- [스켈레톤] `Cargo.toml`의 sqlx에는 `uuid`·`chrono`·`time` feature가 없다 (→ D-4, D-5에서 추가하지 않기로 함).
- [스켈레톤] 호스트 라우트는 `/workspace/:workspaceId/*`(본체)와 `/workspace/:workspaceId/plugins/:pluginId/*`(Plugin). Plugin의 `host.navigate(path)`는 Plugin 경로 밖으로 나갈 수 없다(`web/src/plugins/navigation.ts`) (→ D-22).

---

## 1. 권한

### 1.1 역할별 행동 ([README])

| 행동 | 기업 담당자(`company`) | 투자자(`investor`) |
|---|---|---|
| 데이터룸·자료·검토 기준 조회 | 가능 | 가능 |
| 자료 등록 | 가능 | **불가** |
| 자신의 검토 조회·작성·수정·현황 확인 | **불가** | 가능 |
| 다른 투자자의 검토 조회 | 불가 | 불가 |

- [README] 기업 담당자가 비공개 검토 목록을 요청하면 **빈 목록**을 반환하고, 화면에서도 검토·현황 기능을 쓸 수 없음을 알려야 한다.
- [README] 쓰기 권한이 없는 요청은 **입력값 검증 전에** 거부한다.
- [README] 클라이언트가 보낸 사용자 ID·역할은 신뢰하지 않는다. 작성자는 항상 `AuthenticatedUser.id`.

### 1.2 검사 순서 (모든 method 공통)

1. 인증 — 없으면 `401 not_authenticated` ([스켈레톤] 추출기가 처리)
2. `request.workspaceId == user.workspace_id` — 다르면 `403 forbidden` (D-1)
3. method 존재 여부 — 없으면 `400 invalid_input` (D-2)
4. 역할 권한 — 권한 없으면 `403 forbidden` (params 파싱·검증 전에)
5. params 파싱·입력 검증 — `400 invalid_input`
6. 대상 존재·상태 확인 — 없는 기준·자료는 `404 not_found`, 준비되지 않은 자료는 `400 invalid_input` (D-3)
7. 저장 — DB 오류는 `500 storage_error`

### 1.3 method별 권한 표

| 대상 | method | company | investor |
|---|---|---|---|
| dataroom | `getDataroom` | 허용 | 허용 |
| dataroom | `listDocuments` | 허용 | 허용 |
| dataroom | `getDocument` | 허용 | 허용 |
| dataroom | `createDocument` | 허용 | **403** (검증 전) |
| review | `listCriteria` | 허용 | 허용 |
| review | `listMyReviews` | **빈 목록** ([README]) | 허용 (본인 것만) |
| review | `getMyReview` | **403** (D-6) | 허용 (본인 것만) |
| review | `saveReview` | **403** (검증 전) | 허용 |
| review | `getMyProgress` (현황) | **403** (D-6) | 허용 (본인 것만) |

### D-1. `workspaceId` 불일치 응답

결정: **A. `403 forbidden`** ([스켈레톤] review `dispatch`가 이미 이렇게 하므로 본체·Plugin 응답이 일관된다.)
- 대안: B. `404 not_found` — 존재를 숨기지만 review 쪽 기존 코드와 달라진다.

### D-2. 알 수 없는 method 응답

결정: **B. `400 invalid_input`** (호출 측 실수라는 의미가 분명하고, 4xx라 React Query가 재시도하지 않는다.)
- 대안: A. `404 not_found` — 없는 자료 404와 구분이 어렵다.
- 대안: C. 현행 `501` 유지 — 5xx라 재시도되고 의미상 서버 오류가 아니다.

### D-6. 기업 담당자가 단건 검토·현황을 요청할 때

결정: **B. 단건(`getMyReview`)·현황(`getMyProgress`)은 `403 forbidden`, 목록(`listMyReviews`)만 [README] 빈 목록** ("불가" 의미가 명확하고, 현황 0/3이 "미작성"으로 오해될 여지가 없다.)
- 대안: A. 단건 `null`·현황 0/3 성공 응답 — 목록과 일관되지만 미작성으로 오해될 수 있다.
- 대안: C. `{ available: false, items: [] }` 형태 — 응답 타입이 복잡해진다.

---

## 2. DB 모델

### 2.1 기존 테이블 ([스켈레톤] 수정 금지, 새 마이그레이션만 추가)

- `review_criteria(id TEXT PK, title, review_question, display_order SMALLINT UNIQUE)` — `business`·`team`·`revenue` 고정
- `workspaces(id TEXT PK, name)`, `users(id TEXT PK, email UNIQUE, display_name, password_hash)`
- `workspace_users(workspace_id, user_id, role CHECK IN ('company','investor'), PK(workspace_id, user_id))`
- `sessions(...)`

새 마이그레이션 번호는 `0003_` 이후. 기존 마이그레이션은 덮어쓰지 않는다 [README].

### 2.2 `workspaces` 확장 (D-10)

- 새 마이그레이션에서 `ALTER TABLE workspaces ADD COLUMN description TEXT` 후 `lighthouse`에 `scenario.json`의 `company.description` 값을 넣는다.

### D-10. 데이터룸 정보(회사 설명)

결정: **A. `workspaces`에 `description TEXT` 컬럼 추가** (샘플 시나리오를 온전히 재현한다. 새 마이그레이션으로 확장하므로 기존 마이그레이션을 덮어쓰지 않는다.)
- 대안: B. 별도 `datarooms` 1:1 테이블 — 제공 테이블은 그대로지만 테이블이 하나 늘어난다.
- 대안: C. 저장하지 않음 — 샘플 일부가 누락된다.

### 2.3 `documents` (자료)

| 컬럼 | 타입 | 제약 |
|---|---|---|
| `id` | `TEXT` | PK, `DEFAULT gen_random_uuid()::text` (D-4) |
| `workspace_id` | `TEXT` | `NOT NULL REFERENCES workspaces(id)` |
| `title` | `TEXT` | `NOT NULL`, 길이 CHECK (4장) |
| `file_name` | `TEXT` | `NOT NULL`, 확장자 CHECK (4장) |
| `content` | `TEXT` | `NOT NULL` |
| `status` | `TEXT` | `NOT NULL CHECK (status IN ('ready','processing','failed'))` [README] |
| `created_by` | `TEXT` | `NOT NULL REFERENCES users(id)` (D-7) |
| `created_at` | `TIMESTAMPTZ` | `NOT NULL DEFAULT NOW()` |

- `UNIQUE (workspace_id, id)` — 근거 테이블의 복합 FK 대상 (D-9)
- 인덱스: `(workspace_id, created_at DESC, id ASC)` — [README] 목록 정렬 "작성 시각 내림차순, 같으면 ID 오름차순"
- 제목 검색용 인덱스는 두지 않는다 (D-11)
- 같은 제목·파일명 재등록 허용 — UNIQUE 없음 (D-19)

### D-4. 자료(·검토) ID 타입

결정: **C. `TEXT` — 시드는 샘플 label(`doc-business` 등), 신규는 `gen_random_uuid()::text`** (기존 테이블의 TEXT ID 스타일과 맞고, 시드 ID가 샘플 label과 같아 테스트가 읽기 쉬우며, sqlx feature 추가가 필요 없다.) `reviews.id`도 같은 방식.
- 대안: A. `BIGINT IDENTITY` — ID 오름차순이 삽입 순서와 같지만 시드 ID를 고정값으로 쓰기 불편하다.
- 대안: B. `UUID` 타입 — sqlx `uuid` feature 또는 캐스팅이 필요하다.

참고: "ID 오름차순"은 문자열 순서다. 같은 시각 자료가 생기는 경우에만 쓰이는 2순위 정렬이다.

### D-5. 시각 값을 Rust에서 다루는 방법

결정: **B. SQL에서 ISO 8601 UTC 문자열로 변환해 `String`으로 받는다** (의존성·feature 변경이 없다.) 형식: `to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"')`. 쿼리마다 반복되므로 같은 식을 한곳에 상수로 둔다.
- 대안: A. sqlx `chrono`/`time` feature 추가 — 타입 안전하지만 라이브러리 feature가 늘어난다.

### D-7. `created_by` (등록자) 저장 여부

결정: **A. `created_by TEXT NOT NULL REFERENCES users(id)` 저장** (누가 올렸는지 추적할 수 있고 상세에 표시할 수 있다.) 시드 자료의 등록자는 `company-user`.
- 대안: B. 저장 안 함 — 단순하지만 등록자를 알 수 없다.

### 2.4 `reviews` (검토)

[README] 투자자 한 명의 기준별 검토는 하나이며, 수정해도 같은 검토로 유지된다.

| 컬럼 | 타입 | 제약 |
|---|---|---|
| `id` | `TEXT` | PK, `DEFAULT gen_random_uuid()::text` (D-4) |
| `workspace_id` | `TEXT` | `NOT NULL` |
| `investor_id` | `TEXT` | `NOT NULL` |
| `criterion_id` | `TEXT` | `NOT NULL REFERENCES review_criteria(id)` |
| `decision` | `TEXT` | `NOT NULL CHECK (decision IN ('satisfied','needs_information'))` [README] |
| `comment` | `TEXT` | `NOT NULL`, CHECK 길이 1~2000 & 공백만 불가 (4장) |
| `created_at` | `TIMESTAMPTZ` | `NOT NULL DEFAULT NOW()` — 수정 시 유지 |
| `updated_at` | `TIMESTAMPTZ` | `NOT NULL DEFAULT NOW()` — 수정 시 갱신 |

- `FOREIGN KEY (workspace_id, investor_id) REFERENCES workspace_users(workspace_id, user_id)` (D-12). 역할이 `investor`인지는 앱에서 검사.
- `UNIQUE (workspace_id, investor_id, criterion_id)` — [README] 기준별 1개 + 동시 저장 시 중복 방지 (`INSERT … ON CONFLICT … DO UPDATE`)
- `UNIQUE (workspace_id, id)` — 근거 테이블의 복합 FK 대상 (D-9)
- 추가 인덱스 없음: UNIQUE 제약이 `(workspace_id, investor_id, …)` 조회를 커버한다.

### D-12. `reviews.investor_id` 참조 대상

결정: **A. `(workspace_id, investor_id) → workspace_users(workspace_id, user_id)` 복합 FK** (해당 룸의 멤버만 검토할 수 있음을 DB가 보장한다.)
- 대안: B. `users(id)`·`workspaces(id)` 개별 FK — 단순하지만 룸 멤버십은 앱에서만 보장된다.

### 2.5 `review_evidence` (근거 연결)

[README] 검토 내용과 근거 자료는 구분해 관리한다. 파일명이 아닌 자료 ID로 연결한다.

| 컬럼 | 타입 | 제약 |
|---|---|---|
| `review_id` | `TEXT` | `NOT NULL` |
| `document_id` | `TEXT` | `NOT NULL` |
| `workspace_id` | `TEXT` | `NOT NULL` |

- `PRIMARY KEY (review_id, document_id)` — [README] 중복 근거 방지
- `FOREIGN KEY (workspace_id, review_id) REFERENCES reviews(workspace_id, id)`
- `FOREIGN KEY (workspace_id, document_id) REFERENCES documents(workspace_id, id)`
- 순서 컬럼 없음. 조회 시 자료 정렬 규칙(`created_at DESC, id ASC`)으로 정렬한다 (D-13).
- [README] "`ready` 자료만"은 상태가 바뀔 수 있는 값이라 FK/CHECK로 보장할 수 없다. 저장 트랜잭션 안에서 앱이 검사한다.

### D-9. "같은 데이터룸의 자료만" 보장 방식

결정: **A. 복합 FK `(workspace_id, review_id)`·`(workspace_id, document_id)`** (다른 룸 자료 연결을 DB가 원천 차단해 앱 검사에 빈틈이 있어도 룸 경계가 유지된다.)
- 대안: B. 단순 FK + 앱 검사 — 스키마는 단순하지만 앱 검사가 빠지면 룸 경계가 깨질 수 있다.

### D-13. 근거 순서 보존

결정: **B. 순서를 저장하지 않고 자료 정렬 규칙으로 정렬** (단순하고 자료 목록 정렬과 일관된다.)
- 대안: A. `position` 컬럼 — 입력 순서를 보존하지만 컬럼·검증이 늘어난다.

### 2.6 저장 일관성 ([README])

`saveReview`는 한 트랜잭션:

1. `INSERT INTO reviews … ON CONFLICT (workspace_id, investor_id, criterion_id) DO UPDATE SET decision, comment, updated_at = NOW() RETURNING id` — 행 잠금으로 같은 (투자자, 기준) 동시 저장이 직렬화된다.
2. 근거 자료 검증: 본체 모듈 함수(D-21)가 `SELECT id, status FROM documents WHERE workspace_id = $1 AND id = ANY($2) FOR SHARE`로 잠그고 확인한다. 검증과 삽입 사이에 상태가 바뀌지 않는다.
3. `DELETE FROM review_evidence WHERE review_id = $1`
4. 근거 INSERT
5. COMMIT. 중간 실패 시 ROLLBACK으로 기존 내용·근거를 보존한다.

(2를 1보다 먼저 해도 된다. 입력 검증(4장)은 트랜잭션 시작 전에 끝낸다.)

구현 순서(6단계 확정): 입력 검증 → BEGIN → 기준 존재 확인(없으면 404, FK 위반이 500으로 새지 않게) → 2 → 1 → 3 → 4 → 같은 트랜잭션에서 저장 결과 조회 → COMMIT. 오류 우선순위는 D-53.

### D-14. 동시 수정 충돌 처리

결정: **A. 마지막 저장 우선(last-write-wins)** (README 조건인 "중복 없음·근거 섞임 없음"은 위 트랜잭션과 UNIQUE 제약만으로 충족되고 구현이 단순하다.) 다른 탭의 저장을 덮어쓸 수 있다는 점은 제출 문서의 한계에 적는다. 또 `updated_at = NOW()`는 트랜잭션 시작 시각이라, 동시 저장에서 마지막에 반영된 요청의 `updated_at`이 더 이를 수 있다(내용은 마지막 저장 기준으로 맞지만 시각은 단조 증가하지 않는다). 이것도 한계로 적는다.
- 대안: B. 낙관적 잠금 + `409 conflict` — 덮어쓰기를 알려주지만 오류 종류와 충돌 UX가 추가된다.

### 2.7 시드 (새 마이그레이션)

[README] `samples/scenario.json`의 자료 4건을 `lighthouse`에 재현한다. 샘플 JSON은 런타임 저장소가 아니다.

| id (= label) | 제목 | 파일명 | 상태 | `created_at` (UTC) |
|---|---|---|---|---|
| `doc-business` | 회사 소개 | `company-overview.md` | `ready` | `2026-09-01 09:00:00` |
| `doc-team` | 팀 소개 | `team.md` | `ready` | `2026-09-01 09:01:00` |
| `doc-revenue` | 매출 자료 | `revenue.txt` | `failed` | `2026-09-01 09:02:00` |
| `doc-pipeline` | 고객 인터뷰 | `customer-interviews.md` | `processing` | `2026-09-01 09:03:00` |

- 본문은 `scenario.json`의 `content` 그대로. 등록자는 `company-user` (D-7).
- 목록(내림차순)에서는 고객 인터뷰 → 매출 자료 → 팀 소개 → 회사 소개 순으로 보인다.
- 계정 3개·검토 기준 3개는 [스켈레톤] 이미 `0001`·`0002`에 있다.
- 검토는 시드하지 않는다. 현황 0/3에서 시작한다.

### D-15. 시드 세부

결정:
- `created_at`: **A. JSON 순서대로 1분 간격 고정 시각** (목록 순서가 예측 가능하다.)
- `samples/revenue-update.md`: **A. 시드하지 않고 등록 시연·E2E 업로드 파일로 사용** (등록 → 근거 연결 흐름을 직접 보여줄 수 있다.)
- 대안: 시각 B. 일부 동일 시각 — 정렬 2순위를 시드로 확인할 수 있지만 화면 순서가 직관적이지 않다(2순위는 백엔드 테스트에서 검증).
- 대안: revenue-update B. 시드에 포함 — 처음부터 매출 근거가 있지만 등록 흐름 시연 재료가 사라진다.

### 2.8 마이그레이션 구성

| 파일 | 내용 |
|---|---|
| `api/migrations/0003_documents.sql` | `workspaces.description` 추가·입력, `documents` 테이블·제약·정렬 인덱스, 자료 4건 시드 |
| `api/migrations/0004_reviews.sql` | `reviews`, `review_evidence` 테이블·복합 FK·UNIQUE |

DB CHECK 제약 목록 (D-36):

| 테이블 | 제약 이름 | 규칙 |
|---|---|---|
| `documents` | `documents_status_check` | `status IN ('ready','processing','failed')` |
| `documents` | `documents_title_length` | `char_length(title) BETWEEN 1 AND 200` |
| `documents` | `documents_file_name_format` | `char_length BETWEEN 1 AND 255` + `~* '^[^/\\]+\.(txt|md)$'` |
| `documents` | `documents_content_size` | `char_length(content) >= 1` + `octet_length(content) <= 200000` |
| `reviews` | `reviews_decision_check` | `decision IN ('satisfied','needs_information')` |
| `reviews` | `reviews_comment_length` | `char_length(comment) BETWEEN 1 AND 2000` |
| `reviews` | `reviews_comment_not_blank` | `comment ~ '[^[:space:]]'` |

앱 오류 처리에서 제약 이름으로 분기할 수 있도록 주요 제약에는 이름을 붙였다(`reviews_one_per_criterion`, `reviews_investor_membership`, `review_evidence_*_same_workspace` 등).

### D-34. 본문 최대 크기 단위

결정: **200,000 바이트 (UTF-8 `octet_length`)** (D-23의 "200KB"를 십진 KB로 읽은 값이다.)
- 대안: 204,800 바이트(200KiB) — 요청 제한 256KiB와 단위는 같지만 "200KB"라는 표기와 숫자가 달라 보인다.

### D-35. 마이그레이션 파일 분할

결정: **두 파일 — `0003_documents.sql`(자료), `0004_reviews.sql`(검토)** (도메인별로 읽기 쉽고 plan 단계 순서(자료 → 검토)와 맞는다.)
- 대안: 한 파일 — 한 번에 적용되지만 자료와 검토 변경이 섞인다.

### D-36. DB CHECK 범위

결정: **길이·형식 + 의견 "공백만 불가"까지 DB CHECK. 제어문자·앞뒤 공백 검사는 앱에서만** (README의 "공백뿐인 의견 거부"를 DB도 보장하고, 공백 정의가 Rust와 PostgreSQL 사이에서 어긋날 수 있는 규칙은 앱 한곳에 둔다.)
- 대안: 앱 규칙 전부를 CHECK로 — spec 문구에 가장 충실하지만 Rust `is_whitespace`와 PostgreSQL `[[:space:]]`(libc 기준) 판정이 드물게 다를 수 있다.
- 대안: 길이·형식만 — 단순하지만 "공백뿐인 의견 거부"를 DB가 보장하지 않는다.

참고: 현재 DB(`en_US.utf8`)에서 `[[:space:]]`는 전각 공백 `U+3000`도 공백으로 판정한다(검증함). 그래서 "공백만 불가" CHECK는 앱 규칙(D-24)과 같은 방향으로 동작한다.

### D-37. FK 삭제 동작

결정: **모든 새 FK는 기본값 `NO ACTION`** (자료·검토 삭제 기능이 범위 밖이라 삭제 연쇄 동작이 필요 없고, 실수로 삭제가 번지지 않는다.) [스켈레톤] `0002`는 `ON DELETE CASCADE`를 쓰지만 이번 테이블에는 따르지 않는다.
- 대안: `review_evidence → reviews`만 `ON DELETE CASCADE` — 검토 삭제 시 근거가 함께 정리되지만 지금은 쓰이지 않는 동작이다.

---

## 3. API 계약

공통:
- method 이름은 camelCase 동사+명사 (D-8).
- 요청 `params`는 method별 Rust 구조체로 역직렬화하고 `#[serde(deny_unknown_fields)]`를 붙인다. 역직렬화 실패는 `400 invalid_input`.
- 응답 `result` 타입은 Rust DTO + `ts-bridge` → Gen-TS. 필드는 camelCase ([스켈레톤] 기존 DTO 관례). 시각은 ISO 8601 UTC 문자열 (D-5).
- 오류는 1.2 순서를 따르고, [스켈레톤] 기존 `kind`만 쓴다. 규칙별로 정적 메시지를 나누고, 메시지는 영어 문장으로 쓴다 (D-16, D-41).
- 앱 검증을 통과한 뒤 DB CHECK에 걸리면 변환하지 않고 `500 storage_error`로 둔다 (D-42).

### D-8. method 이름 규칙

결정: **A. camelCase 동사+명사** (`listDocuments`, `saveReview`) (TS 쪽 관례와 같고 [스켈레톤] 예제 `health`와도 어울린다.)
- 대안: B. 점 표기(`documents.list`) — 도메인 그룹이 드러나지만 match 문이 길어진다.
- 대안: C. snake_case — Rust 관례지만 TS에서 어색하다.

### D-16. 오류 세부 정보

결정: **A. [스켈레톤] `ApiError`·`ApiErrorBody`는 그대로 두고 규칙별 정적 메시지를 나눈다** (예: `"Evidence document is not ready."`) (`error.rs`와 Gen-TS를 바꾸지 않는다.) 필드별 오류 표시는 클라이언트 사전 검증(4장과 같은 규칙)으로 한다.
- 대안: B. `ApiErrorBody`에 `field` 추가 — 필드 옆에 서버 오류를 표시할 수 있지만 제공 오류 타입이 바뀐다.
- 대안: C. 새 `kind` 추가 — 화면이 kind로 분기할 수 있지만 kind가 늘어난다.

### D-41. 오류 메시지 언어

결정: **영어 정적 문장** (예: `"Title must be 1-200 characters."`) ([스켈레톤] `"Resource not found."` 등 기존 메시지와 일관된다. 화면의 한국어 문구는 `kind`와 클라이언트 검증으로 표시한다.)
- 대안: 한국어 정적 문장 — 화면에 그대로 보여줄 수 있지만 기존 영어 메시지와 섞인다.

### D-42. 앱 검증 통과 후 DB CHECK 위반

결정: **변환하지 않고 `500 storage_error`** (앱 검증이 DB 규칙보다 넓거나 같으므로, DB CHECK 위반은 앱 버그라는 신호로 드러나야 한다.)
- 대안: SQLSTATE `23514`를 `400 invalid_input`으로 변환 — 입력 탓이면 알맞지만 앱 버그가 입력 오류로 가려진다.

### 3.1 dataroom (본체, `POST /api/dataroom/rpc`)

#### `getDataroom`
- params: `null` 또는 `{}`. 모르는 필드가 있으면 400 (D-39)
- result: `{ id, name, description }` (D-10, `description`은 `string | null`)
- 오류: 401, 403(D-1)

#### `listDocuments`
- params: `null`, `{}` 또는 `{ query?: string }` (제목 부분 일치 검색, 4장·D-11). 모르는 필드가 있으면 400 (D-39). 상태 필터 없음 (D-17)
- result: `{ items: DocumentSummary[] }`
  - `DocumentSummary = { id, title, fileName, status, createdAt }`
- 정렬: [README] `created_at DESC, id ASC`
- 페이지네이션 없음, 전체 반환 (D-18)
- 결과 없음: `items: []` (성공). 조회 실패를 빈 목록으로 대체하지 않는다 [README]
- 오류: 401, 403(D-1), 400(검색어 규칙 위반)

#### `getDocument`
- params: `{ id }` — id는 1~100자, `[A-Za-z0-9-]`만 허용. 아니면 400 (D-38)
- result: `DocumentDetail = { id, title, fileName, status, content, createdAt, createdBy: { id, name } }` (D-7)
- 오류: 401, 403(D-1), 400(id 형식), **404**(없음 또는 다른 룸. 존재를 드러내지 않는다)

#### `createDocument`
- params: `{ title, fileName, content }`
- 권한: company만. investor는 **검증 전에** 403 [README]
- 저장 상태: `ready` [README](업로드에 성공한 UTF-8 자료는 바로 ready로 저장할 수 있다)
- result: `DocumentDetail` (생성된 자료). 화면은 이 ID로 상세 화면에 이동한다.
- 같은 제목·파일명 재등록 허용 (D-19)
- 오류: 401, 403, 400(입력 규칙), 500

### D-17. 근거 선택용 자료 목록

결정: **B. 필터 없이 전체를 받고 화면에서 ready만 선택 가능, 처리 중·실패는 비활성+사유 표시** (선택할 수 없는 이유를 사용자에게 보여줄 수 있다. 서버 검증은 어차피 필수다.)
- 대안: A. `status` 필터 추가 — 선택기가 ready만 받지만 선택할 수 없는 자료가 보이지 않는다.

### D-18. 목록 페이지네이션

결정: **A. 없음 (전체 반환)** (과제 범위의 자료 수에 맞고 단순하다.) 제출 문서의 한계에 적는다.
- 대안: B. 키셋 커서 `(createdAt, id)` — 안정적이지만 구현·테스트가 늘어난다.

### D-19. 같은 제목·파일명 재등록

결정: **A. 허용 (별개 자료)** (보완 자료를 다시 올리는 흐름이 자연스럽고, 자료 삭제 기능이 없으니 잘못 올려도 다시 등록할 수 있다.)
- 대안: B. 룸 내 제목 UNIQUE — 혼동을 막지만 보완 등록 시 제목을 바꿔야 하고 409 처리가 필요하다.

### D-38. `getDocument` id 형식

결정: **1~100자, `[A-Za-z0-9-]`만 허용, 아니면 `400 invalid_input`** (시드 label(`doc-business`)과 UUID 문자열을 모두 받으면서, 형식이 틀린 입력은 DB 조회 전에 거른다.)
- 대안: 길이만 검사 — 단순하지만 이상한 문자열도 조회까지 가서 404가 된다.
- 대안: 검사 없음 — spec의 "400(id 형식)"과 맞지 않는다.

### D-39. params가 없거나 선택인 method의 입력

결정: **`getDataroom`·`listDocuments`·`listCriteria`·`listMyReviews`는 `null`과 `{}`를 모두 허용, 모르는 필드는 400** (호출 측이 편하고 [스켈레톤] `host.call`의 기본값 `params ?? null`과 맞으며, 잘못 보낸 params를 조용히 무시하지 않는다.)
- 대안: `getDataroom`은 params 무시 — 단순하지만 잘못된 params가 조용히 통과한다.

review Plugin의 params 없는 method(`listCriteria`·`listMyReviews`)에도 같은 규칙을 적용한다 (5단계에서 확장, 본체·Plugin 규칙 일치).

### 3.2 review (Plugin, `POST /api/plugins/rpc`, `pluginId: "review"`)

#### `listCriteria`
- params: `null` 또는 `{}`. 모르는 필드가 있으면 400 (D-39)
- result: `{ items: Criterion[] }`, `Criterion = { id, title, reviewQuestion, displayOrder }`, `display_order` 오름차순
- 권한: 둘 다 허용 (D-20)

#### `listMyReviews`
- params: `null` 또는 `{}`. 모르는 필드가 있으면 400 (D-39)
- result: `{ items: Review[] }`. 본인 것만. 기준 `display_order` 순 (D-50)
  - `Review = { id, criterionId, decision, comment, evidence: EvidenceRef[], createdAt, updatedAt }`
  - `EvidenceRef = { documentId, title, fileName, status }` (자료 정렬 규칙 순, D-13). 근거 자료는 저장 후 상태가 바뀌었어도 현재 상태 그대로 표시 (D-52)
- company: `{ items: [] }` [README]. params를 검사하기 전에 반환 (D-51)
- 오류: 401, 403(D-1)

#### `getMyReview`
- params: `{ criterionId }` — 1~100자, `[A-Za-z0-9-]`만 허용. 아니면 400 (D-49)
- result: `Review | null` (미작성이면 `null`)
- company: `403` (D-6)
- 오류: 400(형식), **404**(없는 기준, D-3)

#### `saveReview` (작성·수정 공용, upsert)
- params: `{ criterionId, decision, comment, evidenceDocumentIds: string[] }`
- 권한: investor만. company는 **검증 전에** 403 [README]
- 검증 ([README] + 4장):
  - `decision` ∈ {`satisfied`, `needs_information`}
  - `comment` 1~2000자, 공백만 불가 (D-24)
  - `evidenceDocumentIds` 1~20개, 중복 불가 (D-23)
  - 각 ID: 존재 + 같은 룸 + `ready`. 다른 룸 자료는 "없음"과 같게 취급한다(존재 비노출).
- 오류 (D-3): 형식 400 / 없는 기준 404 / 없는 자료·다른 룸 자료 404 / `processing`·`failed` 자료 400. 여러 개면 기준 404 → 자료 404 → 자료 400 순 (D-53)
- result: 저장된 `Review` (수정 시 `id`·`createdAt` 동일, `updatedAt` 갱신)
- 동시 저장은 마지막 저장 우선 (D-14)

#### `getMyProgress` (현황)
- params: 없음
- result:
  ```
  {
    totalCount,             // 기준 수 (3)
    writtenCount,           // 작성한 기준 수 (satisfied + needs_information)
    unwrittenCount,
    satisfiedCount,
    needsInformationCount,
    items: [{ criterion: Criterion, review: Review | null }]   // 기준 순서
  }
  ```
- [README] `needs_information`은 "작성함"에 포함되지만 "확인함"과 구분한다. 모두 작성했다고 모두 확인된 것은 아니다.
- [README] 개인 진행 상태이며 회사 합의·투자 승인이 아니다. 화면 문구에 반영한다.
- [README] 현황은 **`reviews`(와 연결된 `review_evidence`)만으로 계산**한다. 자료 등록 여부나 `documents`의 개수·상태로 기준 충족을 판단하지 않는다. 기준에 맞는 자료가 등록돼 있어도 검토를 작성하지 않았으면 `미작성`이다.
- company: `403` (D-6)

### D-3. 근거·기준 오류의 HTTP 코드

결정: **A** ([README] "없는 자료·기준"을 별도 오류 종류로 구분하라는 요구에 직접 대응한다.)

| 경우 | 응답 |
|---|---|
| 형식 오류(빈 배열, 중복 ID, 의견 길이) | `400 invalid_input` |
| 없는 기준 `criterionId` | `404 not_found` |
| 없는 자료 / 다른 룸 자료 | `404 not_found` |
| `processing`·`failed` 자료 | `400 invalid_input` |

- 대안: B. 잘못된 참조를 모두 400 — "params 안 잘못된 참조 = 입력 오류"로 일관되지만 없는 자료가 메시지로만 구분된다.

### D-20. `listCriteria` 위치

결정: **A. review Plugin method** (기준은 검토 도메인이라 Plugin이 자기 API로 완결된다.) company도 Plugin 화면에서 기준을 조회한다 (D-29).
- 대안: B. dataroom 본체 method — README 권한 표의 묶음과 맞지만 Plugin이 `target: "dataroom"`으로 가져와야 한다.
- 대안: C. 둘 다 — 중복 구현이 생긴다.

### 3.3 Plugin과 본체의 경계

- Plugin **UI**는 자료가 필요하면 `host.call("listDocuments" | "getDocument", params, { target: "dataroom" })`를 쓴다 (CLAUDE.md 규칙).
- Plugin **서버**는 `documents` 테이블을 직접 조회하지 않는다. 저장 시 근거 검증은 본체 모듈이 제공하는 함수를 같은 트랜잭션으로 호출한다 (D-21).
  - `crate::dataroom::lock_ready_documents(실행기, workspace_id, ids) -> Result<(), ApiError>` (6단계에서 이름 확정)

### D-21. Plugin 서버의 `documents` 접근

결정: **B. 본체 모듈(`crate::dataroom`)에 트랜잭션을 받는 검증 함수를 두고 Plugin 서버가 호출** (자료 테이블 지식을 본체에 모으면서 검증·저장을 한 트랜잭션으로 유지한다.)
- 대안: A. Plugin 서버가 `documents`를 직접 `SELECT … FOR SHARE` — 단순하지만 Plugin이 본체 테이블을 알게 된다.

### D-48. 근거 자료 정보 조회 (D-21 확장)

결정: **본체 `crate::dataroom::document_refs(실행기, workspace_id, ids)`를 호출** (Plugin은 `review_evidence`에서 자료 ID만 읽고 제목·파일명·상태는 본체에서 받는다. D-21처럼 자료 테이블 지식을 본체에 모으고, 실행기(`PgExecutor`)를 받으므로 저장 트랜잭션 안에서도 쓸 수 있다.) 다른 룸·없는 ID는 결과에서 빠지고, 결과는 자료 정렬 규칙(D-13) 순이다.
- 대안: Plugin SQL에서 `documents` JOIN — 쿼리 1번으로 단순하지만 D-21 취지와 어긋난다.

### D-49. `criterionId` 형식

결정: **D-38과 같은 규칙: 1~100자, `[A-Za-z0-9-]`만, 아니면 `400 invalid_input`** (자료 ID와 기준이 같고 시드 ID(`business` 등)와 맞으며, 형식이 틀린 입력은 조회 전에 거른다.)
- 대안: 길이만 검사 — 단순하지만 이상한 문자열도 조회까지 가서 404가 된다.

### D-50. `listMyReviews` 정렬

결정: **기준 `display_order` 오름차순** (기준 목록·현황 화면과 순서가 같다.)
- 대안: `updated_at DESC` — 최근 수정순이지만 기준 화면과 순서가 달라진다.

### D-51. company의 `listMyReviews` 검사 순서

결정: **params를 검사하기 전에 빈 목록 반환** (1.2 순서(역할 → params)를 따른다. README가 정한 company 응답은 빈 목록이다.)
- 대안: params를 먼저 검사해 400 — 입력 오류를 알리지만 1.2 순서와 달라진다.

### D-52. 근거 자료의 현재 상태 표시

결정: **저장 후 `processing`·`failed`로 바뀐 근거도 현재 상태 그대로 응답에 포함** (상태는 바뀔 수 있는 값이라 숨기면 근거가 조용히 사라진 것처럼 보인다. 화면이 상태 배지로 구분한다.) 저장 시 검증(ready만)은 그대로다.
- 대안: ready가 아닌 근거는 응답에서 제외 — 화면이 단순하지만 저장한 근거가 사라져 보인다.

### D-53. `saveReview` 오류 우선순위

결정: **없는 것 먼저: 기준 404 → 없는·다른 룸 자료 404 → `processing`·`failed` 자료 400** (근거 순서를 바꿔 보내도 같은 요청이면 같은 오류가 나오고, 1.2의 "대상 존재 → 상태" 순서와 맞는다.)
- 대안: 입력 순서대로 첫 문제 — 직관적이지만 배열 순서에 따라 응답 코드가 달라진다.

### D-54. 의견 제어문자

결정: **`\n` `\r` `\t`만 허용, 그 밖의 제어문자(NUL 포함)는 `400 invalid_input`** (여러 줄 의견을 쓸 수 있으면서, PostgreSQL TEXT가 저장하지 못하는 NUL이 500으로 새지 않는다.)
- 대안: NUL만 거부 — 본문 규칙과 같고 단순하지만 보이지 않는 제어문자가 저장될 수 있다.

### D-55. 근거 자료 ID 형식

결정: **각 ID를 D-38 형식(1~100자, `[A-Za-z0-9-]`)으로 검사, 아니면 400** (D-38·D-49와 같은 규칙이고 형식이 틀린 ID는 잠금·조회 전에 거른다.)
- 대안: 형식 검사 없음 — 이상한 문자열도 조회까지 가서 404가 된다.

### D-56. 검토 조회의 일관성

결정: **`getMyReview`·`listMyReviews`·`getMyProgress`의 조회(기준, 검토 행, `review_evidence`, `document_refs`)를 `REPEATABLE READ READ ONLY` 트랜잭션 하나로 묶음** (동시 저장 중에도 옛 의견과 새 근거가 섞인 응답이 나오지 않는다.)
- 대안: 트랜잭션 없이 조회 — 단순하지만 저장과 겹치면 섞인 응답이 나올 수 있다.

---

## 4. 입력 규칙

[README] = README가 정함, 나머지는 D-23·D-24·D-25로 확정.
길이 "자"는 모두 **Unicode 코드 포인트** 기준 (서버 `chars().count()`, 클라이언트 `[...s].length`) (D-24).

| 필드 | 규칙 |
|---|---|
| 요청 전체 | ≤ 256KiB [스켈레톤] |
| `title` | 앞뒤 공백 제거 후 **1~200자**, 제어문자 불가, trim한 값 저장 |
| `fileName` | 앞뒤 공백 제거 후 **1~255자**, trim한 값 저장 (D-40), 확장자 `.txt`·`.md` [README] (**대소문자 무시**: `.MD`·`.Txt` 허용), `/` `\` 제어문자 불가, 확장자만 있는 이름(`.md`) 불가 |
| `content` | UTF-8 [README] (JSON 문자열이라 서버 도착 시 이미 유효 UTF-8), **1자 이상**, **NUL(`\0`) 불가**, **최대 200KB (UTF-8 바이트)** |
| `comment` | 1~2000자 [README], 공백만 불가 [README], **trim한 값 저장**, 공백 판정은 **Unicode 공백 전체**(`char::is_whitespace`, 전각 공백 `U+3000` 포함), 제어문자 불가(`\n` `\r` `\t`만 허용, D-54) |
| `decision` | `satisfied` \| `needs_information` [README] |
| `evidenceDocumentIds` | 1개 이상 [README], 중복 불가 [README], **최대 20개**, 각 ID는 D-38 형식 (D-55) |
| `criterionId` | 존재하는 기준 [README] |
| `query`(검색) | 앞뒤 공백 제거, 빈 문자열이면 전체, **최대 100자**, `%` `_` `\`는 LIKE 이스케이프 |

DB CHECK 제약은 길이·형식과 의견 "공백만 불가"까지만 건다(D-36, 목록은 2.8). 길이는 PostgreSQL `char_length`라 코드 포인트 기준으로 앱과 일치한다. 제어문자·앞뒤 공백 검사는 앱에서만 한다.

### D-23. 입력 규칙 수치

결정: 제목 1~200 / 파일명 1~255·확장자 대소문자 무시 / 본문 1자 이상·NUL 금지·최대 200KB / 근거 최대 20 / 검색어 최대 100 (본문 200KB는 JSON 이스케이프로 크기가 늘어나는 것을 감안해 256KiB 요청 제한보다 여유를 둔 값이다.)
- 대안: 제목 100자 — 목록 표시가 안정적이지만 자유도가 낮다.
- 대안: 확장자 소문자만 — 규칙은 단순하지만 `.MD` 파일 사용자가 불편하다.
- 대안: 근거 개수 무제한 — 자료 수만큼 허용되지만 요청 크기 상한이 불명확하다.

### D-40. 파일명 앞뒤 공백

결정: **trim 후 검증하고 trim한 값을 저장** (제목과 같은 규칙이라 일관되고, `" a.md"`처럼 보이지 않는 차이가 생기지 않는다.)
- 대안: 앞뒤 공백이 있으면 400 — 원본을 보존하지만 얻는 이득이 작다.
- 대안: 그대로 저장 — 단순하지만 보이지 않는 차이가 남는다.

### D-24. 의견 길이·공백 기준

결정: **길이 = 코드 포인트, 저장 값 = trim한 값, 공백 = Unicode 공백 전체** (서버·클라이언트가 같은 기준으로 계산할 수 있고, 저장 값과 검증 길이가 일치하며, 전각 공백만 입력한 경우도 거부된다.)
- 대안: 길이 UTF-16 단위 — `<textarea maxLength>`와 같지만 서버가 별도로 계산해야 한다.
- 대안: 원문 저장 — 사용자 서식을 보존하지만 저장 길이와 검증 길이가 달라진다.
- 대안: ASCII 공백만 — 전각 공백만 입력한 의견이 통과된다.

클라이언트 주의: `<textarea maxLength>`는 UTF-16 기준이라 쓰지 않거나 코드 포인트 카운터와 함께 쓴다.

### D-25. 파일 읽기 (브라우저)

결정: **`TextDecoder('utf-8', { fatal: true })`로 판정, BOM 제거 후 저장, 파일 선택 시 제목이 비어 있으면 파일명(확장자 제외)으로 자동 채움** (잘못된 바이트를 조용히 `U+FFFD`로 바꾸지 않고 거부하며, BOM이 본문 첫 글자에 섞이지 않는다.)
- 대안: `File.text()` — 간단하지만 잘못된 인코딩이 조용히 통과한다.
- 대안: BOM 유지 — 원본 그대로지만 본문 앞에 보이지 않는 문자가 남는다.
- 대안: 제목 자동 채움 없음 — 단순하지만 입력이 한 번 더 필요하다.

자동 채움 조건: 고른 파일이 검증(파일명·인코딩·본문)을 통과했고 제목이 비어 있을 때만 채운다. 거부된 파일이면 채우지 않고, 파일을 다른 것으로 바꿔도 이미 채워진 제목은 바꾸지 않는다 (사용자가 입력한 제목을 덮어쓰지 않는다).

### D-11. 제목 검색 방식

결정: **A. 부분 일치 `ILIKE '%' || $1 || '%' ESCAPE '\'`, 인덱스 없음** (직관적이고 과제 데이터 규모에 충분하다.)
- 대안: B. `pg_trgm` GIN 인덱스 — 대량 데이터에 유리하지만 확장 설치가 필요하다.
- 대안: C. 접두 일치 — btree를 쓸 수 있지만 중간 단어로 찾지 못한다.

---

## 5. 화면

[README] 공통: 로딩·오류·빈 결과 구분(조회 실패를 "자료 없음"·"검토 완료"로 표시 금지), 저장 성공 후 목록·현황·이동 반영, 실패 시 입력 보존·재시도, 상세 주소 새로고침·Plugin 재진입, 작은 화면·폼 레이블·키보드 접근, 한국어 UI·typed 문자열·스타일 토큰·UI scale 사용.
[프로젝트 규칙] 상태 처리는 `SessionGate` 패턴(pending → status / error → 재시도 / data), React Query 키는 `scopedKey(context, …)`(Plugin), 사용자 전환 시 이전 캐시·입력이 섞이지 않게 한다([스켈레톤] `Shell`·`RuntimePlugin`이 `user.id`로 key를 걸고 `expireSession`이 캐시를 지운다).

### 5.1 URL 구조

| 화면 | URL |
|---|---|
| 자료 목록(+검색) | `/workspace/:workspaceId` (검색어는 URL에 두지 않음, D-26) |
| 자료 등록 | `/workspace/:workspaceId/documents/new` (D-27) |
| 자료 상세 | `/workspace/:workspaceId/documents/:documentId` |
| 검토 현황 (Plugin 홈) | `/workspace/:workspaceId/plugins/review` |
| 검토 작성·수정 | `/workspace/:workspaceId/plugins/review/criteria/:criterionId` |
| Plugin 내 근거 자료 보기 | `/workspace/:workspaceId/plugins/review/documents/:documentId` (D-22) |

Plugin 내부 경로는 `context.location`(예: `/criteria/team`)으로 읽고 `host.navigate("/criteria/team")`로 이동한다 [스켈레톤].

### 5.2 자료 (본체 `DataroomApp`)

- **목록**: 위에 데이터룸 정보(이름·설명) 카드 (D-43). 항목은 제목·상태 배지(`준비 완료`/`처리 중`/`처리 실패`) [README]와 파일명·등록 시각 (D-46). 검색 입력(레이블 포함, 컴포넌트 상태, Enter·검색 버튼으로 실행, D-44), 등록 버튼(company만 노출). 로딩 / 오류+재시도 / 빈 목록("등록된 자료가 없습니다") / 검색 결과 없음("검색 결과가 없습니다")을 구분한다.
- **상세**: 제목·파일명·상태·본문 [README], 등록자·등록 시각. 본문은 원문 텍스트(`white-space: pre-wrap`)로 표시한다 (D-28). 없는 ID·다른 룸(404)과 형식이 틀린 ID(400)는 "자료를 찾을 수 없습니다"로 (D-47), 네트워크 오류와 구분해 표시한다.
- **등록**(company만, 별도 페이지): 제목, 파일 선택(`accept=".txt,.md"`, D-25 읽기 규칙), 4장과 같은 클라이언트 사전 검증(제출 시 필드별 표시, 파일 오류는 선택 즉시, D-45), 제출 중 버튼 비활성. 성공하면 목록 캐시를 무효화하고 상세로 이동한다. 실패하면 입력을 유지하고 오류를 표시해 재시도할 수 있게 한다. investor가 URL로 직접 들어오면 "자료 등록 권한이 없습니다"를 안내한다.

### 5.3 검토 (Review Plugin)

- **company 진입 시**: [README] "검토·현황은 투자자만 사용할 수 있습니다" 안내 + 검토 기준 목록(읽기 전용) (D-29). `getMyReview`·`getMyProgress`는 호출하지 않는다 (D-6에서 403).
- **현황(홈)**: 작성 n / 미작성 m [README]. `확인함`·`추가 확인 필요`·`미작성` 세 상태를 서로 다른 배지로 구분한다 [README]. 기준별 행에서 작성·수정 화면으로 이동하고, 연결된 근거 자료는 Plugin 내 자료 보기로 이동한다 (D-22). "개인 검토 진행 상태이며 투자 승인이 아닙니다" 문구를 넣는다 [README]. 배지 색은 D-61, 의견은 두 줄 미리보기(D-62), ready가 아닌 근거는 상태 배지를 함께 보인다 (D-52). company는 이 화면 대신 안내 + 기준 목록을 보고 `getMyProgress`를 호출하지 않는다 (D-6, D-29).
- **작성·수정**: 기준 이름·검토 질문, 판단 라디오 그룹(fieldset+legend), 의견 textarea(코드 포인트 글자 수 표시, 1~2000), 근거 자료 선택(전체 목록 중 ready만 선택 가능, 처리 중·실패는 비활성+사유, D-17), 저장. 기존 검토가 있으면 채워서 열린다 [README]. 성공하면 현황·목록 캐시를 무효화하고 현황으로 이동해 "저장했습니다: 기준 이름"을 보인다 (D-58). 기존 검토가 있으면 폼에 마지막 저장 시각을 보인다. 실패하면 입력을 보존하고, 400·404면 자료 목록을 다시 불러온다. 폼을 열 때 ready가 아닌 기존 근거는 선택에서 빼고 안내한다 (D-59). 폼 상태는 컴포넌트 메모리에만 둔다 (D-30). company가 URL로 들어오면 기준 존재 여부를 보기 전에 "검토는 투자자만 작성할 수 있습니다"를 보인다 (1.2 순서).
- **7단계 홈**: 기준 목록(이름·검토 질문)과 작성·수정 이동만 두었다 (D-57). 8단계부터 investor 홈은 위 현황이다.
- **Plugin 내부 이동**: `<a href>` 대신 버튼 + `host.navigate`를 쓴다 (D-60).
- **근거 자료 보기**: `host.call("getDocument", { id }, { target: "dataroom" })`로 제목·파일명·상태·본문을 표시하고, 새로고침·재진입을 지원한다.

### D-22. Plugin에서 근거 자료로 "이동"하는 방법

결정: **A. Plugin 안에 자료 보기 화면(`/plugins/review/documents/:id`)을 두고 `host.call("getDocument", …, { target: "dataroom" })`로 표시** ([스켈레톤] 호출 경계와 `host.navigate` 제약을 그대로 지키고, 새로고침·재진입이 가능하다.)
- 대안: B. 호스트 URL로 `<a href>` 링크 — 본체 화면을 재사용하지만 Plugin이 호스트 URL 구조를 알아야 한다.
- 대안: C. `PluginHost` 확장 — 본체 화면으로 정식 이동하지만 제공 SDK 계약을 바꿔야 한다.
- 대안: D. 펼침/모달 — "이동"이라는 요구 해석이 약하다.

### D-26. 검색어를 URL에 둘지

결정: **B. 컴포넌트 상태만** (단순하다.) 새로고침하면 검색이 초기화된다. 이 한계는 제출 문서에 적는다.
- 대안: A. `?q=` 반영 — 새로고침·뒤로가기에도 검색이 유지되지만 URL 동기화가 필요하다.

### D-27. 등록 폼 위치

결정: **A. 별도 페이지 `/documents/new`** (작은 화면에서 넓게 쓸 수 있고 URL로 들어올 수 있다.)
- 대안: B. 목록 상단 인라인 — 등록 반영이 한눈에 보이지만 작은 화면에서 목록이 밀린다.

### D-28. `.md` 본문 표시

결정: **A. 원문 텍스트(`white-space: pre-wrap`)** (라이브러리가 필요 없고 XSS 걱정이 없다.)
- 대안: B. 마크다운 렌더링 — 읽기 좋지만 새 라이브러리와 sanitize가 필요하다.

### D-29. company의 Plugin 화면에서 기준 목록 표시

결정: **A. 안내 문구 + 기준 목록(읽기 전용)** ([README] "검토 기준 조회 가능" 권한을 화면으로 보여준다. D-20에서 기준 API가 Plugin에만 있다.)
- 대안: B. 안내 문구만 — 단순하지만 기업이 기준을 볼 화면이 없다.

### D-30. 작성 중 입력 보존 범위

결정: **A. 폼 상태는 컴포넌트 메모리에만** (사용자 간 입력이 섞이는 경우를 원천 차단하고 단순하다.) 저장 실패 시에는 같은 화면에 입력이 남는다. 페이지를 이탈하면 작성분이 사라진다는 점은 제출 문서의 한계에 적는다.
- 대안: B. 이탈 경고 추가 — 유실을 막지만 Plugin 내 이동을 가로채야 한다.
- 대안: C. `sessionStorage` 임시 저장 — 새로고침에도 남지만 사용자 전환 시 정리가 필요하다.

### D-43. 데이터룸 정보 표시 위치

결정: **자료 목록 위에 이름·설명 카드** (D-10에서 저장한 설명이 화면에 쓰인다.) 조회 실패는 목록과 따로 오류+재시도로 표시한다.
- 대안: 표시 안 함 — 요청이 하나 줄지만 D-10 설명 컬럼이 화면에서 쓰이지 않는다.

### D-44. 제목 검색 실행 시점

결정: **Enter·검색 버튼으로 제출할 때** (키보드 접근이 자연스럽고 실행 시점이 분명해 E2E 테스트가 쉽다.) 검색 중에는 "검색 초기화" 버튼을 보인다.
- 대안: 입력 중 300ms debounce — 즉시 반응하지만 타이머 코드와 요청이 늘고 테스트에 대기가 필요하다.

### D-45. 등록 폼 검증 오류 표시 시점

결정: **제출 시 필드별 표시 + 첫 오류 필드로 포커스, 파일 오류는 파일 선택 즉시** (입력 중에 오류가 뜨지 않아 산만하지 않고, 파일 문제는 고른 순간 알 수 있다.)
- 대안: 입력 중 실시간 표시 — 즉시 피드백이지만 입력 중인 필드에도 오류가 떠서 산만하다.

### D-46. 목록 항목 정보와 시각 표시

결정: **제목·상태 + 파일명·등록 시각, 시각은 브라우저 시간대로 locale에 맞춰 표시** (응답에 이미 있는 값이라 추가 비용이 없고, 최신순 정렬과 같은 제목 재등록(D-19)을 구분할 수 있다.)
- 대안: 제목·상태만 — 단순하지만 같은 제목 자료를 구분하기 어렵다.

### D-47. 상세 화면에서 형식이 틀린 ID(400)

결정: **404와 같이 "자료를 찾을 수 없습니다"로 표시** (상세 ID는 URL에서 오므로 사용자에게는 "없는 자료"와 같은 의미이고, 재시도해도 결과가 바뀌지 않는다.)
- 대안: "입력값이 올바르지 않습니다" 표시 — 서버 응답과 문구가 일치하지만 URL을 고친 사용자에게 의미가 모호하다.

### D-57. 7단계 Plugin 홈

결정: **기준 목록 + 작성·수정 이동만** (기준별 상태·개수는 8단계 `getMyProgress`로 한 번에 만들어 중복 작업이 없다.)
- 대안: `listMyReviews`로 기준별 상태 배지 표시 — 바로 보이지만 8단계에서 데이터 출처를 다시 바꿔야 한다.

### D-58. 검토 저장 성공 후 화면

결정 (8단계에서 변경): **현황(홈)으로 이동하고 `?saved=기준ID`로 방금 저장한 기준을 "저장했습니다: 기준 이름"으로 안내** (README "저장 후 현황·화면 이동에 반영"에 직접 대응하고, 개수와 기준 행이 바로 갱신된 것을 볼 수 있다. `host.navigate`는 쿼리를 유지한다.)
- 7단계에서는 홈에 상태가 없어 "폼에 머물며 저장했습니다·시각 표시"로 두었다.
- 대안: 폼에 머물기 — 연이어 고치기 편하지만 현황 반영을 보려면 직접 돌아가야 한다.

### D-61. 판단 배지 색

결정: **`확인함` success, `추가 확인 필요` warning, `미작성` 기본색** (README "세 상태를 서로 다른 배지로" 요구에 맞고, 추가 확인 필요가 확인함과 다르게 눈에 띈다. 자료 상태 배지와 같은 색 체계다.)
- 대안: 추가 확인 필요를 danger — 더 눈에 띄지만 "실패"처럼 보인다.

### D-62. 현황의 의견 표시

결정: **의견은 두 줄까지만 미리보기(`line-clamp-2`)** (기준 행이 길어지지 않고, 전문은 작성·수정 화면에서 본다.)
- 대안: 전문 표시 — 다 보이지만 2000자 의견이면 현황이 길어진다.

### D-59. 수정 시 ready가 아니게 된 기존 근거

결정: **폼을 열 때 선택에서 빼고 "근거 선택에서 뺀 자료"로 안내. 선택된 채 상태가 바뀐 자료는 해제할 수 있게 체크박스를 비활성화하지 않음** (그대로 저장해도 서버 검증을 통과하고, 사용자가 해제하지 못해 저장이 막히는 일이 없다.)
- 대안: 선택된 채 두고 저장 시 서버가 거부 — 저장한 그대로 보이지만 해제하기 전까지 매번 400이 난다.

### D-60. Plugin 내부 이동 방식

결정: **버튼 + `host.navigate("/criteria/…")`** (Plugin은 호스트 URL 구조를 모르므로 `<a href>`를 만들 수 없다. D-22와 같은 이유.)
- 대안: 호스트 URL을 조합한 `<a href>` — 새 탭 열기가 되지만 Plugin이 호스트 URL을 알아야 한다.

---

## 6. 테스트 범위

[README] 권한·입력·오류·영속화·롤백·동시성·사용자 흐름을 검증한다. 프론트엔드 브라우저 테스트는 Playwright. mock과 실제 API·DB 검증을 구분한다. DB 연결 실패를 메모리 저장소 성공으로 대체하지 않는다. 테스트 데이터·DB 격리·초기화 방법, 실행 명령·결과·미검증 범위를 제출한다.

### 6.1 백엔드 (실제 PostgreSQL, `#[sqlx::test]` + dispatch 직접 호출, D-32)

| 영역 | 케이스 |
|---|---|
| 권한 | 다른 `workspaceId` → 403 / investor `createDocument` 403: **잘못된 입력과 함께 보내도 403**(검증 전 거부 확인) / company `saveReview` 403 (잘못된 입력 포함) / company `listMyReviews` 빈 목록 / company `getMyReview`·`getMyProgress` 403 / investor A가 B의 검토를 못 봄 |
| 입력 | 제목·파일명·본문·의견 경계값(0, 1, 최대, 최대+1), 공백만 의견, 전각 공백만 의견, 이모지 포함 길이, NUL, 확장자 대소문자, 중복 근거 ID, 빈 근거, 근거 21개, 잘못된 `decision`, 알 수 없는 필드, 알 수 없는 method → 400 |
| 오류 구분 | 없는 자료·다른 룸 자료 → 404 / processing·failed 근거 → 400 / 없는 기준 → 404 / 없는 자료 상세 → 404 (D-3) |
| 영속화 | 저장 후 새 커넥션으로 다시 조회 |
| 수정 | 두 번 저장해도 검토 1행, `id`·`createdAt` 유지, 근거 교체 |
| 롤백 | 테스트 DB에만 건 트리거로 `review_evidence` INSERT를 실패시켜 기존 의견·근거 보존 확인 (D-31) |
| 동시성 | 같은 (투자자, 기준)에 서로 다른 근거로 동시 저장 N회 → 검토 1행, 근거 목록은 어느 한 요청의 것과 정확히 일치 |
| 정렬·검색 | 동일 시각 ID 오름차순, `%`·`_` 검색어 이스케이프 |
| 현황 | 미작성/확인함/추가 확인 필요 카운트 |

**백엔드 미검증 범위** (D-32 A의 결과): HTTP 계층(`CurrentUser` 추출기의 401, 쿠키, 256KiB 본문 제한)은 백엔드 테스트로 검증하지 않는다. 미인증 흐름은 E2E에서 확인하고 나머지는 제출 문서에 적는다.

### D-32. 백엔드 테스트 방식

결정: **A. `#[sqlx::test]`(테스트마다 임시 DB + 마이그레이션 자동 적용) + dispatch 함수 직접 호출** ([스켈레톤] 이미 있는 sqlx `migrate` feature로 가능하고, 새 의존성 없이 테스트 간 완전히 격리된다.)
- 대안: B. `tower::ServiceExt::oneshot` HTTP 테스트 추가 — 401·본문 제한까지 검증하지만 dev-dependency가 늘어난다.
- 대안: C. 실행 중인 API에 외부 HTTP 요청 — 실제에 가깝지만 DB 격리가 어렵고 느리다.

### D-31. 롤백 테스트에서 실패를 유도하는 방법

결정: **A. 테스트 DB에만 트리거를 걸어 `review_evidence` INSERT를 실패시킴** (운영 코드를 바꾸지 않고 실제 DB 롤백을 확인한다. `#[sqlx::test]` 임시 DB라 다른 테스트에 영향이 없다.)
- 대안: B. 저장 함수에 테스트 전용 실패 주입 훅 — 정확하지만 운영 코드에 테스트용 분기가 생긴다.
- 대안: C. DB 제약 위반 입력 — 앱 검증이 먼저 막으면 재현되지 않는다.

### 6.2 Playwright E2E (실제 API·DB, 아래 오류 케이스만 네트워크 mock)

| 흐름 | 케이스 |
|---|---|
| 인증 | 로그인 없이 상세 URL 진입 → 로그인 화면 |
| 자료 | company 로그인 → 등록(`samples/revenue-update.md`, 고유 제목) → 상세 이동·표시 → 목록 최상단 반영 → 검색 → 상세 URL 새로고침 |
| 권한 화면 | investor에게 등록 버튼 없음 · 등록 URL 직접 진입 시 안내 / company Plugin 진입 시 안내 + 기준 목록 |
| 검토 | investor 로그인 → 현황 → 작성(ready만 선택 가능) → 저장 → 현황 반영 → 다시 열어 수정 → 근거 자료 보기 이동·새로고침 |
| 상태 구분 | `확인함`·`추가 확인 필요`·`미작성` 배지 구분 |
| 오류·재시도 | `page.route`로 API 실패를 흉내 → 오류 표시·입력 보존·재시도 성공 (**이 케이스만 mock**) |
| 사용자 전환 | investor 작성 중 → 로그아웃 → peer 로그인 → 이전 입력·검토가 보이지 않음 |
| 접근성·반응형 | [스켈레톤] desktop·mobile 두 프로젝트, 레이블로 입력 찾기(`getByLabel`), 키보드로 저장 |

### D-33. E2E 데이터 격리

결정: **A. 실행 전 `make reset-db` 필수 + 테스트가 누적 상태에 견디게 작성** (앱 코드에 테스트용 경로를 만들지 않는다.) 작성 규칙:
- 등록 자료는 실행마다 고유한 제목(타임스탬프 등)을 쓰고, 그 제목으로 찾아 검증한다.
- 검토는 "덮어쓰기 후 방금 저장한 값이 보이는지"로 검증한다([스켈레톤] desktop → mobile 순서로 같은 DB를 쓰므로 이전 프로젝트가 남긴 검토가 있을 수 있다).
- "현황 0/3" 같은 초기 상태 검증은 테스트에 넣지 않거나, 넣는다면 reset 직후 첫 실행에서만 성립함을 제출 문서에 적는다.
- 대안: B. 테스트 전용 초기화 API — 매번 깨끗하지만 운영 코드에 위험한 엔드포인트가 생긴다.
- 대안: C. Playwright에서 DB 직접 정리(`pg`) — 정밀하지만 새 dev 의존성이 생긴다.

---

## 결정 목록

| ID | 주제 | 결정 |
|---|---|---|
| D-1 | workspaceId 불일치 응답 | A. 403 forbidden |
| D-2 | 알 수 없는 method 응답 | B. 400 invalid_input |
| D-3 | 근거·기준 오류 HTTP 코드 | A. 없는 기준·자료·다른 룸 404, 준비 안 된 자료 400 |
| D-4 | 자료·검토 ID 타입 | C. TEXT (시드 label / 신규 `gen_random_uuid()::text`) |
| D-5 | 시각 값 Rust 처리 | B. SQL에서 ISO 8601 문자열로 변환 |
| D-6 | company의 단건 검토·현황 요청 | B. 403 (목록만 빈 목록) |
| D-7 | 자료 등록자 저장 | A. `created_by` 저장 |
| D-8 | method 이름 규칙 | A. camelCase |
| D-9 | 같은 룸 근거 보장 방식 | A. 복합 FK |
| D-10 | 회사 설명 저장 | A. `workspaces.description` 추가 |
| D-11 | 제목 검색 방식 | A. ILIKE 부분 일치, 인덱스 없음 |
| D-12 | investor_id 참조 대상 | A. `workspace_users` 복합 FK |
| D-13 | 근거 순서 보존 | B. 보존 안 함, 자료 정렬 규칙 |
| D-14 | 동시 수정 충돌 처리 | A. last-write-wins |
| D-15 | 시드 세부 | 시각 A(JSON 순 1분 간격) · revenue-update A(업로드용) |
| D-16 | 오류 세부 정보 | A. 현행 유지, 정적 메시지 분리 |
| D-17 | 근거 선택용 자료 목록 | B. 전체 목록, 화면에서 ready만 선택 |
| D-18 | 목록 페이지네이션 | A. 없음 |
| D-19 | 같은 제목 재등록 | A. 허용 |
| D-20 | listCriteria 위치 | A. review Plugin |
| D-21 | Plugin 서버의 documents 접근 | B. 본체 모듈 함수 호출 |
| D-22 | Plugin → 근거 자료 이동 | A. Plugin 내 자료 보기 화면 |
| D-23 | 입력 규칙 수치 | 제목 1~200 / 파일명 1~255·대소문자 무시 / 본문 1자 이상·NUL 금지·최대 200KB / 근거 최대 20 / 검색어 최대 100 |
| D-24 | 의견 길이·저장·공백 | 코드 포인트 · trim 저장 · Unicode 공백 |
| D-25 | 브라우저 파일 읽기 | fatal 디코딩 · BOM 제거 · 제목 자동 채움 |
| D-26 | 검색어 URL 반영 | B. 컴포넌트 상태만 |
| D-27 | 등록 폼 위치 | A. 별도 페이지 |
| D-28 | .md 본문 표시 | A. 원문 텍스트 |
| D-29 | company Plugin 화면의 기준 목록 | A. 안내 + 읽기 전용 기준 목록 |
| D-30 | 작성 중 입력 보존 범위 | A. 컴포넌트 메모리만 |
| D-31 | 롤백 테스트 실패 유도 | A. 테스트 DB 트리거 |
| D-32 | 백엔드 테스트 방식 | A. `#[sqlx::test]` + dispatch 직접 호출 |
| D-33 | E2E 데이터 격리 | A. `make reset-db` + 누적 상태에 견디는 테스트 |
| D-34 | 본문 최대 크기 단위 | 200,000 바이트 (`octet_length`) |
| D-35 | 마이그레이션 파일 분할 | 두 파일 (0003 자료, 0004 검토) |
| D-36 | DB CHECK 범위 | 길이·형식 + 의견 공백만 불가, 제어문자·앞뒤 공백은 앱에서만 |
| D-37 | FK 삭제 동작 | 모두 NO ACTION (삭제 기능 범위 밖) |
| D-38 | getDocument id 형식 | 1~100자, `[A-Za-z0-9-]`만, 아니면 400 |
| D-39 | 선택 params 입력 | `null`·`{}` 허용, 모르는 필드 400 (본체·review 공통) |
| D-40 | 파일명 앞뒤 공백 | trim 후 검증, trim한 값 저장 |
| D-41 | 오류 메시지 언어 | 영어 정적 문장 |
| D-42 | DB CHECK 위반 | 그대로 500 storage_error |
| D-43 | 데이터룸 정보 표시 | 자료 목록 위 카드 |
| D-44 | 검색 실행 시점 | Enter·검색 버튼 제출 |
| D-45 | 등록 검증 표시 시점 | 제출 시 필드별, 파일 오류는 선택 즉시 |
| D-46 | 목록 항목·시각 | 제목·상태·파일명·등록 시각(브라우저 시간대) |
| D-47 | 상세 400(ID 형식) | 404와 같이 "자료를 찾을 수 없습니다" |
| D-48 | 근거 자료 정보 조회 | 본체 `document_refs` 호출 (D-21 확장) |
| D-49 | criterionId 형식 | D-38과 같은 규칙 |
| D-50 | listMyReviews 정렬 | 기준 `display_order` 순 |
| D-51 | company listMyReviews | params 검사 전 빈 목록 |
| D-52 | 근거 현재 상태 | 바뀐 상태 그대로 표시 |
| D-53 | saveReview 오류 우선순위 | 기준 404 → 자료 404 → 자료 400 |
| D-54 | 의견 제어문자 | `\n` `\r` `\t`만 허용 |
| D-55 | 근거 자료 ID 형식 | D-38 형식 |
| D-56 | 검토 조회 일관성 | REPEATABLE READ READ ONLY 트랜잭션 (현황 포함) |
| D-57 | 7단계 Plugin 홈 | 기준 목록 + 이동만 (상태는 8단계) |
| D-58 | 저장 성공 후 화면 | 현황으로 이동 + "저장했습니다: 기준" (8단계에서 변경) |
| D-59 | 바뀐 기존 근거 | 선택 해제 후 안내, 해제 가능하게 둠 |
| D-60 | Plugin 내부 이동 | 버튼 + host.navigate |
| D-61 | 판단 배지 색 | 확인함 success · 추가 확인 필요 warning · 미작성 기본 |
| D-62 | 현황 의견 표시 | 두 줄 미리보기 |
