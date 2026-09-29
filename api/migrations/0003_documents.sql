ALTER TABLE workspaces ADD COLUMN description TEXT;

UPDATE workspaces
SET description = '소규모 제조사를 위한 재고 관리 소프트웨어'
WHERE id = 'lighthouse';

CREATE TABLE documents (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    title TEXT NOT NULL,
    file_name TEXT NOT NULL,
    content TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('ready', 'processing', 'failed')),
    created_by TEXT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT documents_workspace_id_id_key UNIQUE (workspace_id, id),
    CONSTRAINT documents_title_length CHECK (char_length(title) BETWEEN 1 AND 200),
    CONSTRAINT documents_file_name_format CHECK (
        char_length(file_name) BETWEEN 1 AND 255
        AND file_name ~* '^[^/\\]+\.(txt|md)$'
    ),
    CONSTRAINT documents_content_size CHECK (
        char_length(content) >= 1
        AND octet_length(content) <= 200000
    )
);

CREATE INDEX documents_list_idx ON documents (workspace_id, created_at DESC, id ASC);

INSERT INTO documents (id, workspace_id, title, file_name, content, status, created_by, created_at) VALUES
    ('doc-business', 'lighthouse', '회사 소개', 'company-overview.md',
     '제조사 재고 관리 구독형 소프트웨어. 사업장당 월 15만 원, 2026년 8월 유료 고객 40개.',
     'ready', 'company-user', '2026-09-01 09:00:00+00'),
    ('doc-team', 'lighthouse', '팀 소개', 'team.md',
     '대표 제조업 운영 8년, 개발 책임자 B2B 개발 6년, 디자이너 4년. 전담 영업 담당자는 없음.',
     'ready', 'company-user', '2026-09-01 09:01:00+00'),
    ('doc-revenue', 'lighthouse', '매출 자료', 'revenue.txt',
     '자료를 읽지 못했습니다.',
     'failed', 'company-user', '2026-09-01 09:02:00+00'),
    ('doc-pipeline', 'lighthouse', '고객 인터뷰', 'customer-interviews.md',
     '아직 준비되지 않았습니다.',
     'processing', 'company-user', '2026-09-01 09:03:00+00');
