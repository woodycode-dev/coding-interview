# 프로젝트 규칙

## 문서
- 요구사항 원본: README.md (특히 "필수 기능", "공통 완료 조건")
- 내가 확정한 설계: docs/spec.md
- 진행 계획: docs/plan.md
- 셋이 충돌하면 추측하지 말고 나에게 묻는다.

## 작업 방식
- 나는 C#/ASP.NET 5년 경력이고 TypeScript·React와 SQL(Oracle·MSSQL·MySQL)도 실무로 썼다. Rust는 처음이다.
- Rust 코드는 핵심 문법을 짧게 설명하고, 가능하면 C#과 비교해서 설명한다. React·SQL은 설명하지 않아도 된다.
- 설계(DB, API 계약, 권한 규칙)는 내가 정한다. spec.md에 없으면 선택지와 장단점만 제시한다.
- plan.md의 한 단계만 구현하고, 요청하지 않은 기능·리팩터링·라이브러리 추가는 하지 않는다.
- git commit은 하지 않고 커밋 메시지만 제안한다.

## README에 없는 이 프로젝트 규칙
- 제공된 설정 파일(compose.yaml, Makefile, Dockerfile 등)은 수정하지 않는다. 로컬 실행은 compose.override.yaml을 쓴다.
- 권한은 api/src/auth.rs의 AuthenticatedUser만 신뢰한다.
- React Query 키는 scopedKey(context, ...)를 쓴다.
- Plugin에서 자료가 필요하면 host.call(..., { target: "dataroom" })로 본체 API를 호출한다.
- 화면 상태 처리는 web/src/auth/session.tsx의 SessionGate 패턴을 따른다.

## 검사
- 단계를 마치면 README의 검사 명령을 실행하고 결과를 나에게 보고한다.