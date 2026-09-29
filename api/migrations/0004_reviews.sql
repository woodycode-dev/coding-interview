CREATE TABLE reviews (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
    workspace_id TEXT NOT NULL,
    investor_id TEXT NOT NULL,
    criterion_id TEXT NOT NULL REFERENCES review_criteria(id),
    decision TEXT NOT NULL CHECK (decision IN ('satisfied', 'needs_information')),
    comment TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT reviews_investor_membership FOREIGN KEY (workspace_id, investor_id)
        REFERENCES workspace_users(workspace_id, user_id),
    CONSTRAINT reviews_one_per_criterion UNIQUE (workspace_id, investor_id, criterion_id),
    CONSTRAINT reviews_workspace_id_id_key UNIQUE (workspace_id, id),
    CONSTRAINT reviews_comment_length CHECK (char_length(comment) BETWEEN 1 AND 2000),
    CONSTRAINT reviews_comment_not_blank CHECK (comment ~ '[^[:space:]]')
);

CREATE TABLE review_evidence (
    review_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    PRIMARY KEY (review_id, document_id),
    CONSTRAINT review_evidence_review_same_workspace FOREIGN KEY (workspace_id, review_id)
        REFERENCES reviews(workspace_id, id),
    CONSTRAINT review_evidence_document_same_workspace FOREIGN KEY (workspace_id, document_id)
        REFERENCES documents(workspace_id, id)
);
