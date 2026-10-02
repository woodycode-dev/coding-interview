# 제출 문서

데이터룸 자료 등록·조회(본체)와 투자자 검토·현황(Review Plugin)을 구현했습니다.
설계 결정의 전체 근거는 `docs/spec.md`(D-1~D-62, 결정·이유·대안), 진행 단계는 `docs/plan.md`에 있습니다.
구현 전 공유(이해한 과제, 확인이 필요한 점과 가정, 구현 순서)는 Draft PR로 했습니다.

목차

1. 실행·계정·DB 초기화 방법
2. 완료·미완료 범위
3. 작업 시간과 환경 대응 시간
4. 구현 과정에서 달라진 서비스 이해와 근거
5. 주요 설계 판단과 대안
6. Plugin·Gen-TS 활용과 API·DB·UI 연결
7. 테스트: 실행 명령·결과·한계
8. AI 활용

---

## 1. 실행·계정·DB 초기화 방법

> README: 실행·계정·DB 초기화 방법

### 실행

```sh
make dev            # db, api, web 실행. 준비되면 http://localhost:5178/
make check-docker   # 생성 결과 일치·lint·타입·빌드 검사
make gen-ts-docker  # Rust DTO를 바꾼 뒤 TypeScript 타입 재생성
```

이 환경에서는 제공된 `compose.yaml`이 `ui-kit`·`api-client`를 읽기 전용으로 마운트해서 `make dev`(web 컨테이너 종료, exit 254)와 `make gen-ts-docker`(EROFS)가 실패했습니다. 제공 설정 파일은 수정하지 않고, 로컬 전용 `compose.override.yaml`(git 제외)로 해결했습니다. 같은 문제가 생기면 저장소 루트에 아래 파일을 만들어 주세요.

```yaml
services:
  web:
    volumes:
      - ./ui-kit:/app/ui-kit
      # make gen-ts-docker writes generated types here; compose.yaml mounts it read-only.
      - ./api-client:/app/api-client:rw
  playwright:
    volumes:
      - ./ui-kit:/app/ui-kit
```

### 계정

README 제공 계정을 그대로 씁니다. 비밀번호는 모두 `dataroom`입니다.

| 역할 | 이메일 |
|---|---|
| 기업 담당자 | `company@lighthouse.test` |
| 투자자 | `investor@lighthouse.test` |
| 투자자 | `peer@lighthouse.test` |

### DB 초기화

```sh
make reset-db   # 서비스를 내리고 DB 볼륨 삭제
make dev        # 마이그레이션·시드가 다시 적용됨
```

`make reset-db`는 DB만이 아니라 서비스도 내리므로 이어서 `make dev`가 필요합니다. 세션도 지워져 다시 로그인해야 합니다.
시드는 새 마이그레이션(`api/migrations/0003_documents.sql`)에 있습니다: 회사 설명, 자료 4건(`doc-business`·`doc-team` 준비 완료, `doc-revenue` 처리 실패, `doc-pipeline` 처리 중). 검토 기준 3개는 제공된 `0001_review_criteria.sql`을 그대로 씁니다.

---

## 2. 완료·미완료 범위

> README: 완료·미완료 범위

### 완료 (README 필수 기능)

| README 항목 | 구현 |
|---|---|
| 권한 | 서버가 `AuthenticatedUser`의 역할·워크스페이스로 판단. 쓰기 권한이 없는 요청은 입력 검증 전에 403. 기업 담당자의 비공개 검토 목록 요청은 빈 목록, 화면에는 "검토·현황은 투자자만 사용할 수 있습니다" 안내 |
| 자료 등록·조회 | 브라우저에서 UTF-8 `.txt`·`.md`를 읽어 JSON으로 전송, 바로 `ready`로 저장. 목록(제목·상태·제목 검색, `created_at DESC, id ASC`), 상세(제목·파일명·상태·본문) |
| 검토 작성·수정 | 기준 3개 조회, 판단(`확인함`·`추가 확인 필요`)과 1~2000자 의견, 같은 룸의 `ready` 자료를 ID로 근거 연결. 투자자·기준별 검토 1개(upsert), 수정해도 같은 검토 |
| 검토 현황 | 작성 n / 미작성 m, 세 상태 배지 구분, 근거 자료로 이동, "개인 검토 진행 상태이며 회사 전체의 합의나 투자 승인이 아닙니다" 문구 |
| 저장 일관성 | 검토 수정과 근거 교체를 한 트랜잭션으로 처리, 실패 시 롤백. 동시 저장에도 검토 1행·근거가 한 요청의 것과 일치 |

### 미완료·축소

- 선택 구현(AI 검토 초안): 구현하지 않았습니다.
- Playwright E2E: spec 6.2 중 핵심 흐름 2개만 자동화했습니다. 나머지 항목은 7장 미검증 범위에 이유와 함께 적었습니다.
- 그 밖의 한계는 7장 "한계"에 있습니다.

---

## 3. 작업 시간과 환경 대응 시간

> README: 대략적인 작업 시간과 별도의 환경 대응 시간

| 날짜 | 작업 | 시간 |
|---|---|---|
| 09-28 | 환경 구성 (fork, clone, make dev, 환경 대응) | 약 1시간 (환경 대응 약 25분 포함) |
| 09-29~30 | 스켈레톤 분석, spec·plan·CLAUDE.md·명령어 작성, Draft PR 공유 | 약 3시간 30분 |
| 09-30 | DB 마이그레이션 (트랜잭션 롤백 검증 포함) | 약 40분 |
| 09-30 | 자료 API 계획 검토, 세부 결정 정리 | 약 20분 |
| 10-01 | 자료 API + 테스트 구현 | 약 20분 |
| 10-02 | 자료 API 검증(테스트, make check-docker, curl 확인) 및 커밋 | 약 20분 |
| 10-02 | 자료 화면 (경계값 확인, /check 포함) | 약 1시간 40분 |
| 10-02 | 검토 조회 API + 테스트, gen-ts 환경 대응 | 약 30분 |
| 10-03 | 검토 저장 API + 테스트, PR 설계 코멘트 | 약 40분 |
| 10-03 | 검토 화면 | 약 30분 |
| 10-03 | 검토 현황 API·화면 | 약 15분 |
| 10-03 | 백엔드 테스트 보강, Playwright E2E | 약 30분 |
| 10-03 | 제출 문서 | 약 30분 |

- 작업 시간 합계: **약 10시간 45분** (10-01 이후 시간은 대화 기록 기준 추정치)
- 환경 대응 시간 합계: **약 45분** (09-28 web 컨테이너 실행 실패 약 25분, `make gen-ts-docker` EROFS 약 10분, 작업 중 api 재빌드 충돌 약 10분)

### 환경 대응 내용

| 문제 | 원인 | 해결 |
|---|---|---|
| `make dev` 시 web 컨테이너 종료(exit 254) (약 25분) | `compose.yaml`이 `ui-kit`을 `:ro`로 마운트해 `pnpm install`이 `ui-kit/node_modules`를 만들지 못함 (`ENOENT: ... mkdir '/app/ui-kit/node_modules'`) | 로컬 `compose.override.yaml`로 `:ro`만 제외. `docker compose config`로 병합 확인 |
| `make gen-ts-docker` 실패(EROFS) (약 10분) | `api-client`가 `:ro` 마운트 | 같은 override에 `api-client` rw 마운트 추가 |
| `cargo fmt`로 파일 수정 불가 | `api`·`plugins`가 `:ro` 마운트 (`make check-docker`의 `--check`는 읽기만 해서 통과) | 같은 이미지를 `docker run`으로 실행하고 `api`·`plugins`만 쓰기 가능하게 마운트 |
| 6단계 커밋 후 `make dev`에서 api 컨테이너가 unhealthy로 실패 (`could not execute process /target/debug/dataroom-api`, exit 101) (약 10분) | 작업 중 AI가 컨테이너에서 `cargo test`·`cargo fmt`를 돌리는 동안 api 서비스의 `cargo watch`도 같은 빌드 디렉터리로 재빌드해 충돌("Blocking waiting for file lock on build directory"). 그 사이 실행 파일이 없어진 순간에 실행됨 | 작업이 끝난 뒤 `docker compose down` → `make dev`로 다시 띄움. 코드 변경 없음 |

---

## 4. 구현 과정에서 달라진 서비스 이해와 근거

> README: 구현 과정에서 달라진 서비스 이해와 그 근거

| 처음 이해 | 바뀐 이해 | 근거 |
|---|---|---|
| REST API를 설계한다 | RPC 엔드포인트 2개(본체 `POST /api/dataroom/rpc`, Plugin `POST /api/plugins/rpc`)에 method를 추가하는 구조. API 설계 = method·params·result 설계 | 스켈레톤 `api/src/handlers/dataroom_rpc.rs`, `plugin_rpc.rs`가 `dispatch`로 method를 넘김 |
| "처리 중·실패 자료는 근거 불가"를 DB 제약으로 막는다 | 상태는 바뀌는 값이라 FK·CHECK로 보장할 수 없음. DB가 보장하는 것(같은 룸, 중복 없음, 기준별 1개)과 앱이 저장 트랜잭션에서 보장하는 것(ready 여부)으로 나뉨 | 복합 FK·UNIQUE(`0004_reviews.sql`) + 저장 시 `FOR SHARE` 잠금 후 상태 검사 |
| 저장만 트랜잭션이면 근거가 섞이지 않는다 | 조회 쿼리가 따로 실행되면 저장과 겹칠 때 응답에서 섞일 수 있음 → 읽기도 `REPEATABLE READ READ ONLY` 트랜잭션으로 묶음 (D-56) | 조회 API 검토 중 발견. 검토 행·근거·자료 정보를 세 쿼리로 읽고 있었음 |
| 근거는 저장할 때 ready면 계속 ready | 저장 후 상태가 바뀔 수 있어 검토 조회와 화면은 ready가 아닌 근거도 표시해야 함 (D-52, D-59) | 수정 화면에서 바뀐 근거를 선택 해제하고 안내 |
| 현황은 자료 등록 여부와 관련 있다 | 현황은 검토(reviews)만으로 계산. 기준에 맞는 자료가 있어도 검토를 쓰지 않았으면 미작성 | README "자료 등록 성공만으로 충족 판단 금지". 시드 자료가 있어도 0/3인 테스트로 확인 |
| 권한 검사와 입력 검증 순서는 중요하지 않다 | 권한을 먼저 봐야 잘못된 입력을 보낸 투자자에게 400이 아니라 403이 나감 | README "쓰기 권한이 없는 요청은 입력값 검증 전에 거부". 잘못된 입력과 함께 보내도 403인 테스트 |
| 기준 존재는 DB FK가 막아 준다 | 저장 전에 확인하지 않으면 FK 위반이 500으로 새어 나감 → 트랜잭션 맨 앞에서 기준 404 확인 | D-42(앱 검증 통과 후 DB 제약 위반은 500)와 함께 설계 |
| 본문 200KB면 요청 256KiB 안에 들어간다 | JSON 변환에서 줄바꿈 등이 이스케이프되어 커짐. 200,000바이트 본문도 요청이 256KiB를 넘을 수 있음 | 정확히 200,000바이트 파일로 경계값을 확인하던 중 발견. 줄바꿈이 많은 본문은 요청 제한에 걸려 413 |
| Plugin도 라우터로 화면을 나눈다 | Plugin에는 라우터가 없음. 호스트가 넘긴 `context.location`을 직접 해석하고, 이동은 `host.navigate`로만. 쿼리도 넘길 수 있음 | `plugin-sdk/index.ts`, `web/src/plugins/navigation.ts`의 `pluginPath` |
| 캐시 키는 어디서나 `scopedKey` | `scopedKey`는 Plugin 컨텍스트 전용. 본체 화면은 `["dataroom", workspaceId, ...]` 키를 쓰고, 사용자 전환 시 캐시 분리는 스켈레톤의 `Shell key=user.id`·`expireSession`이 맡음 | `web/src/components/shell.tsx`, `web/src/auth/session.tsx` |

---

## 5. 주요 설계 판단과 대안

> README: 주요 설계 판단과 대안

전체 62개 결정은 `docs/spec.md`에 있습니다. 평가 항목과 관련이 큰 것만 옮깁니다.

| 번호 | 판단 | 선택하지 않은 대안 |
|---|---|---|
| D-9 | 같은 룸 자료만 근거로: 복합 FK `(workspace_id, review_id)`·`(workspace_id, document_id)`. 앱 검사에 빈틈이 있어도 DB가 룸 경계를 지킴 | 단순 FK + 앱 검사 (앱 검사가 빠지면 룸 경계가 깨짐) |
| D-12 | `reviews.(workspace_id, investor_id)` → `workspace_users` 복합 FK. 룸 멤버만 검토 가능함을 DB가 보장 | `users`·`workspaces` 개별 FK |
| D-21, D-48 | Plugin 서버는 `documents`를 직접 조회하지 않음. 본체 `crate::dataroom`의 `lock_ready_documents`(저장 검증)·`document_refs`(조회)를 호출. 둘 다 실행기를 받아 Plugin 트랜잭션 안에서 동작 | Plugin이 `documents`를 직접 `SELECT … FOR SHARE` 또는 JOIN (단순하지만 Plugin이 본체 테이블을 알게 됨) |
| 2.6 | `saveReview`는 한 트랜잭션: 기준 확인 → 근거 `FOR SHARE` 잠금·검사 → `INSERT … ON CONFLICT DO UPDATE` → 근거 DELETE·INSERT → 같은 트랜잭션에서 조회 → COMMIT | — |
| D-14 | 동시 수정은 마지막 저장 우선. UNIQUE와 트랜잭션만으로 중복·섞인 근거가 생기지 않음 | 낙관적 잠금 + 409 (덮어쓰기를 알려주지만 오류 종류·충돌 UX 추가) |
| D-3, D-53 | 형식 오류 400 / 없는 기준·없는 자료·다른 룸 자료 404 / 처리 중·실패 자료 400. 여러 개면 기준 404 → 자료 404 → 자료 400 | 잘못된 참조를 모두 400 (없는 자료가 메시지로만 구분됨) |
| D-6 | 기업 담당자의 단건 검토·현황 요청은 403, 목록만 README대로 빈 목록 | 단건 `null`·현황 0/3 성공 응답 (미작성으로 오해될 수 있음) |
| D-36 | DB CHECK는 길이·형식과 의견 "공백만 불가"까지. 제어문자·앞뒤 공백은 앱에서만 | 앱 규칙 전부를 CHECK로 (Rust와 PostgreSQL의 공백 판정이 드물게 다를 수 있음) |
| D-4 | ID는 `TEXT`: 시드는 샘플 label, 신규는 `gen_random_uuid()::text` | `BIGINT IDENTITY`, `UUID` 타입 (sqlx feature 추가 필요) |
| D-17 | 근거 선택 목록은 전체를 받고 ready만 선택 가능, 처리 중·실패는 비활성+사유 | `status` 필터 (선택할 수 없는 자료가 보이지 않음) |
| D-22 | Plugin 안에 근거 자료 보기 화면을 두고 `host.call("getDocument", …, { target: "dataroom" })`로 표시. 새로고침·재진입 가능 | 호스트 URL `<a href>` (Plugin이 호스트 URL 구조를 알아야 함), `PluginHost` 확장 |
| D-56 | 검토 조회·현황을 `REPEATABLE READ READ ONLY` 트랜잭션으로 | 트랜잭션 없이 조회 |
| D-58 | 저장 성공 후 현황으로 이동하고 저장한 기준 안내 (`?saved=기준ID`) | 폼에 머물기 |
| D-32 | 백엔드 테스트는 `#[sqlx::test]`(테스트마다 임시 DB) + `dispatch` 직접 호출 | HTTP `oneshot` 테스트 (401·본문 제한까지 검증하지만 dev-dependency 증가) |
| D-33 | E2E는 실행 전 `make reset-db` + 누적 상태에 견디게 작성(고유 제목, 덮어쓰기 후 검증) | 테스트 전용 초기화 API (운영 코드에 위험한 엔드포인트) |

---

## 6. Plugin·Gen-TS 활용과 API·DB·UI 연결

> README: Plugin·Gen-TS 활용과 API·DB·UI 연결을 실제 코드로 설명

검토 저장 하나를 DB부터 화면까지 따라갑니다.

### 6.1 DB: 규칙을 제약으로 (`api/migrations/0004_reviews.sql`)

```sql
CONSTRAINT reviews_investor_membership FOREIGN KEY (workspace_id, investor_id)
    REFERENCES workspace_users(workspace_id, user_id),
CONSTRAINT reviews_one_per_criterion UNIQUE (workspace_id, investor_id, criterion_id),
...
CONSTRAINT review_evidence_document_same_workspace FOREIGN KEY (workspace_id, document_id)
    REFERENCES documents(workspace_id, id)
```

기준별 1개, 같은 룸 자료만, 룸 멤버만을 DB가 보장합니다. "ready 자료만"은 바뀌는 값이라 6.3의 트랜잭션에서 검사합니다.

### 6.2 API 계약: Rust DTO 하나가 서버 입력과 TS 타입의 원본 (Gen-TS)

`plugins/review/server/types.rs`

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts-bridge", derive(ts_rs::TS), ts(export, export_to = "types/"))]
pub struct SaveReviewParams {
    pub criterion_id: String,
    pub decision: ReviewDecision,
    pub comment: String,
    pub evidence_document_ids: Vec<String>,
}
```

`make gen-ts-docker`가 생성한 `api-client/src/types/SaveReviewParams.ts`

```ts
export type SaveReviewParams = {
  criterionId: string;
  decision: ReviewDecision;
  comment: string;
  evidenceDocumentIds: Array<string>;
};
```

`deny_unknown_fields`로 클라이언트가 `investorId` 같은 필드를 보내면 400입니다(작성자는 항상 `AuthenticatedUser.id`). `make check-docker`의 Gen-TS 검사가 Rust 선언과 생성 파일의 일치를 확인합니다.

### 6.3 서버: Plugin dispatch → 본체 함수 호출 (경계 D-21)

`api/src/plugins/mod.rs`가 Plugin 서버 코드를 `#[path]`로 포함하고 `review::dispatch`로 넘깁니다. `plugins/review/server/mod.rs`의 `save_review`는 company면 params를 보기 전에 403을 반환하고, 입력 검증 뒤 한 트랜잭션에서 저장합니다. 자료 상태 검사는 본체 함수를 같은 트랜잭션으로 부릅니다.

```rust
dataroom::lock_ready_documents(&mut *tx, &user.workspace_id, &params.evidence_document_ids)
```

`api/src/dataroom/mod.rs`

```rust
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
```

없거나 다른 룸 자료는 404, ready가 아니면 400입니다. 잠금 덕분에 검사와 근거 INSERT 사이에 상태가 바뀌지 않습니다(9단계 테스트로 확인). 이어서 `INSERT … ON CONFLICT (workspace_id, investor_id, criterion_id) DO UPDATE`로 같은 검토 행을 유지합니다.

### 6.4 UI: 생성 타입 → React Query → `host.call`

`plugins/review/ui/api.ts`

```ts
// Every key starts with scopedKey(context, …) so users and workspaces never share cache.
mine: (context: PluginContext) => scopedKey(context, "review", "mine"),

export function useSaveReview() {
  ...
  return useMutation({
    mutationFn: (params: SaveReviewParams) => host.call<Review>("saveReview", params),
    onSuccess: async (review) => {
      queryClient.setQueryData(keys.myReview(context, review.criterionId), review);
      await queryClient.invalidateQueries({ queryKey: keys.mine(context) });
    },
  });
}
```

Plugin에 필요한 자료는 본체 API를 호출합니다.

```ts
host.call<ListDocumentsResponse>("listDocuments", {}, { target: "dataroom", signal })
```

현황(`getMyProgress`)도 `mine` 아래 키라서 저장하면 함께 무효화되고, 폼은 성공 시 `host.navigate("/?saved=…")`로 현황에 이동합니다. 근거 자료 보기는 Plugin 내부 경로 `/documents/:id`에서 `getDocument`(target: dataroom)로 표시합니다. 화면 상태는 스켈레톤 `SessionGate` 패턴(로딩 → 오류+재시도 → 데이터)을 따릅니다.

### 6.5 본체 화면

`web/src/dataroom/`의 목록·상세·등록 화면은 생성된 `dataroomRpcHandler`를 감싼 훅(`web/src/dataroom/api.ts`)으로 `getDataroom`·`listDocuments`·`getDocument`·`createDocument`를 호출합니다. 등록 성공 시 목록 캐시를 무효화하고 상세로 이동합니다. 파일은 `TextDecoder("utf-8", { fatal: true })`로 읽어 잘못된 인코딩을 거부하고 BOM을 제거합니다(D-25).

---

## 7. 테스트: 실행 명령·결과·한계

> README: 직접 작성한 테스트의 실행 명령·결과·한계

### 7.1 백엔드 (실제 PostgreSQL)

```sh
make db
docker compose run --rm api cargo test --locked --manifest-path api/Cargo.toml
```

- 결과 (2026-10-03): **47개 통과 / 0 실패** (`api/tests/dataroom.rs` 21, `api/tests/review.rs` 26). 동시성 테스트는 10회 반복해 10/10 통과
- 방식: mock 없음. `#[sqlx::test]`가 테스트마다 임시 DB를 만들고 마이그레이션(시드 포함)을 적용해 테스트끼리 섞이지 않음. HTTP를 거치지 않고 `dispatch`를 직접 호출
- 영역: 권한, 입력 경계값(0·1·최대·최대+1, 공백·전각 공백·이모지·NUL, 확장자 대소문자, 근거 빈 배열·중복·21개), 오류 구분(400·403·404), 영속화, 수정(`id`·`createdAt` 유지, 근거 교체), 정렬·검색, 현황
- 롤백: 테스트 DB에만 건 트리거로 근거 INSERT를 실패시켜, 기존 의견·근거가 보존되는지 확인 (`failed_save_rolls_back_to_previous_review`)
- 동시성: 같은 투자자·기준에 서로 다른 근거로 8개 요청을 동시에 저장 → 검토 1행, 근거와 의견이 같은 요청의 것과 일치 (`concurrent_saves_keep_one_review_with_one_requests_evidence`)
- 커밋 확인: 풀 밖에서 새로 연 커넥션으로 조회 (`created_document_is_committed`, `saved_review_and_evidence_are_committed`)
- 근거 잠금: 한 트랜잭션이 근거 자료를 잡은 동안 다른 커넥션의 상태 UPDATE가 `lock_timeout` 뒤 SQLSTATE `55P03`으로 실패하고, 잠금 해제 후 성공 (`evidence_lock_blocks_status_change_until_commit`)

### 7.2 Playwright E2E (실제 API·DB)

```sh
make reset-db && make test-e2e
```

- 결과 (2026-10-03): **4개 통과 / 0 실패** (테스트 2개 × desktop·mobile 프로젝트). 초기화 없이 한 번 더 실행해도 4개 통과(누적 상태에서도 동작)
- mock 없음. 업로드 파일은 테스트 안에서 내용을 만들어 `setInputFiles`로 전달
- 첫 실행은 4개 모두 실패했고 원인은 테스트 기대값 2개(Plugin 홈 주소 끝 `/`, `/documents/new`가 id 패턴에 맞음)였습니다. 앱 코드는 고치지 않았습니다.

| 테스트 | 흐름 |
|---|---|
| `tests/review-flow.spec.ts` | 투자자 로그인 → 현황 → "사업 이해" 작성(추가 확인 필요, 근거 회사 소개) → 처리 중·실패 자료 체크박스 비활성 → 저장 → 현황에 안내·배지·의견·근거 표시 → 새로고침 후 유지 → "매출 현황"을 확인함으로 저장 → 확인함·추가 확인 필요·미작성 배지가 보이고 배경색이 서로 다름 |
| `tests/document-flow.spec.ts` | 기업 담당자 로그인 → 자료 등록(고유 제목) → 상세에 제목·파일명·본문 → 목록 최상단 → 상세 URL 새로고침 후 유지 |

### 7.3 README 검사

`make check-docker` 통과 (Gen-TS 일치, prettier·eslint, tsc, 플러그인·웹 빌드, `cargo fmt --check`, `cargo check`).

### 7.4 미검증 범위

| 항목 | 이유 | 대신 확인한 것 |
|---|---|---|
| HTTP 계층: `CurrentUser`의 401, 세션 쿠키, 256KiB 본문 제한(413) | 백엔드 테스트가 `dispatch`를 직접 호출해 거치지 않음 (D-32) | 수동 확인 |
| 검토 조회 스냅샷(D-56)이 저장과 섞이지 않는지 | 저장과 조회를 정확히 엇갈리게 해야 하는 타이밍 의존 테스트라 불안정 | — |
| E2E 인증: 로그인 없이 상세 URL 진입 | 제출 일정상 핵심 흐름을 우선 | 스켈레톤 SessionGate, 수동 확인 |
| E2E 자료 검색 | 위와 같음 | 백엔드 검색·이스케이프 테스트, 수동 확인 |
| E2E 권한 화면 (투자자 등록 버튼 없음·등록 URL 안내, 기업 담당자 Plugin 안내) | 위와 같음 | 서버 권한은 백엔드 테스트, 화면은 수동 확인 |
| E2E 검토 재수정, 근거 자료 보기 이동·새로고침 | 위와 같음 | 수정은 백엔드 테스트, 화면은 수동 확인 |
| E2E 오류·재시도 (`page.route` mock) | mock 케이스는 작성 비용이 커서 제외 | 서버를 내린 상태에서 오류 표시·재시도 수동 확인 |
| E2E 사용자 전환 (로그아웃 → 다른 투자자) | 제출 일정상 핵심 흐름을 우선 | 백엔드 다른 투자자 검토 비노출 테스트. 화면은 스켈레톤 `user.id` key·캐시 삭제와 `scopedKey`에 의존 |
| E2E 키보드로 저장 | 제출 일정상 핵심 흐름을 우선 | E2E는 레이블·역할(`getByLabel`·`getByRole`)로 입력을 찾고 desktop·mobile로 실행 |

### 7.5 한계

- 동시 수정은 마지막 저장 우선(D-14). 다른 탭의 저장을 덮어쓸 수 있음
- 동시 저장 시 `updated_at`이 단조 증가하지 않을 수 있음 (`NOW()`가 트랜잭션 시작 시각)
- 목록 페이지네이션 없음 (D-18)
- 검색어는 새로고침 시 초기화 (D-26)
- 작성 중 페이지를 떠나면 입력이 사라짐 (D-30). 근거 "보기"로 이동해도 사라짐
- 줄바꿈·특수문자가 많은 200KB 이하 파일은 JSON 변환으로 요청이 256KiB를 넘어 413이 나며, 화면에는 일반 실패 문구만 표시
- 500 저장 오류도 화면에서는 연결 오류 문구로 표시 (재시도는 가능)
- 자료 삭제가 범위 밖이라 확인용으로 등록한 자료는 `make reset-db`로 정리
- E2E는 실행 전 DB 초기화가 필요 (D-33)

---

## 8. AI 활용

> README: AI 활용 사례 2~3개에는 제공한 맥락, 실제 입력·응답 일부 또는 관련 변경, 채택·수정 판단과 검증 근거

### 8.1 AI 활용 방식

Claude Code를 쓰되, 설계는 직접 하고 구현을 맡기는 방식으로 진행했습니다.

| 파일 | 역할 |
|---|---|
| `CLAUDE.md` | Claude가 매번 읽는 작업 규칙 (문서 위치, 작업 범위, 코드에만 있는 규칙) |
| `docs/spec.md` | 내가 확정한 설계 (DB, API 계약, 권한 규칙) |
| `docs/plan.md` | 구현 단계와 진행 상태 |
| `.claude/commands/step.md` | `/step N`: N단계 계획 제시 → 승인 → 구현 → 검사 → 보고 |
| `.claude/commands/check.md` | `/check N`: README·spec 기준으로 N단계 구현 검토 |

단계마다 `/step`으로 구현하고, 직접 화면과 DB 값을 확인한 뒤 `/check`로 누락을 점검하고 커밋했습니다.
README를 요구사항 원본으로 두고 CLAUDE.md에는 README에 없는 규칙만 적어, AI에게 준 맥락이 원본과 어긋나지 않게 관리했습니다.

### 8.2 사례 1: 환경 문제 해결 (AI 제안을 바꿈)

- 맥락: `make dev` 후 web 컨테이너가 exit 254로 종료된 로그 (`ENOENT: no such file or directory, mkdir '/app/ui-kit/node_modules'`)
- AI 응답: `ui-kit`의 `:ro` 마운트가 원인이라고 짚고, `compose.yaml`에서 `:ro`를 지우라고 제안
- 내 판단: 원인 분석은 채택했지만, 제공 설정 파일은 수정하지 않기로 하고 로컬 전용 `compose.override.yaml`로 `:ro`만 제외. 이 규칙을 CLAUDE.md에 "제공된 설정 파일(compose.yaml, Makefile, Dockerfile 등)은 수정하지 않는다. 로컬 실행은 compose.override.yaml을 쓴다."로 남겼고, 이후 같은 원인의 `make gen-ts-docker` EROFS도 같은 override 방식으로 해결
- 검증: `docker compose config`로 병합 결과 확인, `make dev` 정상 기동

### 8.3 사례 2: FK 삭제 동작 근거 정정 (AI 설명을 바로잡음, D-37)

- 맥락: 검토·근거 테이블의 FK 삭제 동작을 정하는 단계
- AI 응답: NO ACTION을 제안하면서 근거를 이렇게 설명
  > "기존 0002 스타일"
- 내 판단: `api/migrations/0002_auth.sql`을 직접 열어 보니 기존 FK는 `ON DELETE CASCADE`였음. 결론(NO ACTION)은 유지하되 이유를 "삭제 기능이 과제 범위 밖이라 연쇄 삭제가 필요 없다"로 바로잡아 spec D-37에 기록
- 검증: 0002 파일의 `REFERENCES workspaces(id) ON DELETE CASCADE` 확인

### 8.4 사례 3: 테스트 작성 시점 변경 (AI 계획을 바꿈)

- 맥락: 자료 API 단계의 구현 계획 검토
- AI 응답: 자료 API 계획에서 자동 테스트를 9단계(백엔드 테스트)로 미룸
  > "자동 테스트는 9단계에서 작성합니다. 이번에는 curl과 psql로 직접 확인합니다."
- 내 판단: 제출 일정을 앞당긴 상황에서 마지막 단계에 테스트가 몰리면 위험하다고 보고, API 단계마다 테스트를 함께 작성하도록 변경. 9단계는 spec 6.1 대비 빈틈을 채우는 단계로 바꿈
- 검증: 단계별 커밋에 테스트가 함께 들어감(자료 API 18개, 조회 API 12개, 저장 API 9개, 현황 4개). 9단계에서 spec 6.1과 대조해 커밋 확인·경계값·근거 잠금 4개를 추가해 47개

### 8.5 그 밖의 사례 (요약)

| 맥락 | AI 응답 | 내 판단·검증 |
|---|---|---|
| spec 초안 작성 (README, 스켈레톤, "README에 없는 건 선택지·장단점만 제시" 지시) | 결정 필요 33개와 선택지 | 대부분 채택, 기업 담당자 처리 등은 직접 이해한 뒤 확정. AI가 적은 사실(Plugin 이동 제한, sqlx feature 없음)은 코드를 직접 열어 확인 |
| Draft PR 공유문 | 확인 질문 3개가 포함된 초안 | README를 다시 읽고 2개는 스스로 결정할 문제로 보고 제외, spec에 결정 근거를 남기는 방식으로 변경 |
| 경계값 파일(정확히 200,000바이트)로 자료 등록을 시험하다 실패를 내가 발견 | 화면·서버·DB의 비교식은 모두 `≤ 200,000`으로 일치함을 확인하고, 원인(JSON 이스케이프로 요청이 256KiB를 넘어 413)을 재현으로 확인 | 시간 대비 효과를 따져 고치지 않고 한계로 기록 |
| 조회 API 검토(`/check 5`) | 조회 쿼리 3개가 트랜잭션 없이 실행돼 저장과 겹치면 섞인 근거가 응답될 수 있다고 지적 | README 조건에 해당한다고 보고 수정 결정, 저장 단계와 함께 처리(D-56) |
| 저장 API 계획 | spec에 의견 제어문자 규칙이 없고, NUL은 PostgreSQL TEXT에 저장되지 않아 500이 난다고 지적 | 여러 줄 의견을 위해 줄바꿈·탭만 허용하고 나머지 제어문자는 400으로 결정(D-54) |
| "never used" 경고 코드 삭제 요청 | 빌드 캐시를 지우고 다시 빌드해 경고가 재현되지 않음을 확인, 실제 사용처를 근거로 삭제하지 않음 | 지시를 그대로 따르지 않고 검증한 응답을 채택 |
| 검토 화면 확인 절차 | "없는 기준" 확인 절차에 계정 조건(투자자)을 빠뜨림 | 기업 담당자로 확인하다 "투자자만" 문구를 보고 원인을 물음 → 역할을 먼저 보는 것이 spec 1.2대로임을 확인하고 투자자로 재확인 |
| E2E 첫 실행 4개 실패 | 로그를 분석해 앱이 아니라 테스트 기대값 문제임을 구분 | 앱 코드는 그대로 두고 테스트만 수정, 초기화 없이 한 번 더 실행해 누적 상태 조건 확인 |
