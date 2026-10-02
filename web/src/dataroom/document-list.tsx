import { useId, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import {
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  Input,
  Label,
} from "@biyard/components";
import { ConnectionError } from "../components/connection-error";
import { useI18n, type MessageKey } from "../i18n";
import { dataroomPath, useDataroom, useDocuments } from "./api";
import { validateQuery } from "./document-input";
import { StatusBadge, formatDateTime } from "./document-meta";

export function DocumentList({
  workspaceId,
  canUpload,
}: {
  workspaceId: string;
  canUpload: boolean;
}) {
  const { t } = useI18n();
  return (
    <div className="space-y-6">
      <DataroomSummary workspaceId={workspaceId} />
      <section aria-labelledby="documents-heading" className="space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <h1 id="documents-heading" className="text-heading-4 font-semibold">
            {t.documents}
          </h1>
          {canUpload ? (
            <Button asChild className="min-h-11">
              <Link to={dataroomPath(workspaceId, "documents", "new")}>{t.uploadDocument}</Link>
            </Button>
          ) : null}
        </div>
        <DocumentSearch workspaceId={workspaceId} />
      </section>
    </div>
  );
}

function DataroomSummary({ workspaceId }: { workspaceId: string }) {
  const { t } = useI18n();
  const info = useDataroom(workspaceId);
  if (info.isPending)
    return (
      <p role="status" className="text-muted-foreground">
        {t.loading}
      </p>
    );
  if (info.isError) return <ConnectionError retry={() => info.refetch()} />;
  return (
    <Card>
      <CardHeader>
        <h2 className="text-heading-5 font-semibold">{info.data.name}</h2>
        {info.data.description ? (
          <CardDescription className="whitespace-pre-wrap">{info.data.description}</CardDescription>
        ) : null}
      </CardHeader>
    </Card>
  );
}

function DocumentSearch({ workspaceId }: { workspaceId: string }) {
  const { t, locale } = useI18n();
  const inputId = useId();
  const errorId = useId();
  const [draft, setDraft] = useState("");
  const [query, setQuery] = useState("");
  const [error, setError] = useState<MessageKey | null>(null);
  const documents = useDocuments(workspaceId, query);

  function submit(event: FormEvent) {
    event.preventDefault();
    const invalid = validateQuery(draft);
    setError(invalid);
    if (!invalid) setQuery(draft.trim());
  }
  function clear() {
    setDraft("");
    setQuery("");
    setError(null);
  }

  return (
    <>
      <form role="search" onSubmit={submit} className="flex flex-wrap items-end gap-2">
        <div className="flex min-w-0 flex-1 basis-60 flex-col gap-2">
          <Label htmlFor={inputId}>{t.searchLabel}</Label>
          <Input
            id={inputId}
            type="search"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? errorId : undefined}
            className="min-h-11"
          />
        </div>
        <Button type="submit" variant="outline" className="min-h-11">
          {t.search}
        </Button>
        {query ? (
          <Button type="button" variant="ghost" className="min-h-11" onClick={clear}>
            {t.clearSearch}
          </Button>
        ) : null}
      </form>
      {error ? (
        <p id={errorId} role="alert" className="text-destructive">
          {t[error]}
        </p>
      ) : null}
      {documents.isPending ? (
        <p role="status" className="text-muted-foreground">
          {t.loading}
        </p>
      ) : documents.isError ? (
        <ConnectionError retry={() => documents.refetch()} />
      ) : documents.data.items.length === 0 ? (
        <Card>
          <CardContent className="pt-6">
            <p>{query ? t.noSearchResults : t.noDocuments}</p>
          </CardContent>
        </Card>
      ) : (
        <ul className="divide-y divide-border rounded-lg border border-border bg-card">
          {documents.data.items.map((document) => (
            <li key={document.id}>
              <Link
                to={dataroomPath(workspaceId, "documents", document.id)}
                className="flex flex-wrap items-center justify-between gap-2 px-4 py-3 hover:bg-accent"
              >
                <span className="min-w-0 flex-1 basis-60">
                  <span className="block font-semibold break-words">{document.title}</span>
                  <span className="block text-caption break-all text-muted-foreground">
                    {document.fileName} · {formatDateTime(document.createdAt, locale)}
                  </span>
                </span>
                <StatusBadge status={document.status} />
              </Link>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
