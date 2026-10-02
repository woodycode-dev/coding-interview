import { Link, useParams } from "react-router-dom";
import { ConnectionError } from "../components/connection-error";
import { useI18n } from "../i18n";
import { dataroomPath, errorStatus, useDocument } from "./api";
import { StatusBadge, formatDateTime } from "./document-meta";

export function DocumentDetail({ workspaceId }: { workspaceId: string }) {
  const { t, locale } = useI18n();
  const { documentId = "" } = useParams();
  const document = useDocument(workspaceId, documentId);
  const back = (
    <Link to={dataroomPath(workspaceId)} className="inline-flex min-h-11 items-center underline">
      {t.backToList}
    </Link>
  );

  if (document.isPending)
    return (
      <p role="status" className="text-muted-foreground">
        {t.loading}
      </p>
    );
  if (document.isError) {
    // 400 (malformed id in the URL) and 404 both mean "no such document" to the user.
    const status = errorStatus(document.error);
    if (status === 404 || status === 400)
      return (
        <div className="space-y-4">
          <p role="alert">{t.documentNotFound}</p>
          {back}
        </div>
      );
    return (
      <div className="space-y-4">
        <ConnectionError retry={() => document.refetch()} />
        {back}
      </div>
    );
  }

  const { title, fileName, status, content, createdAt, createdBy } = document.data;
  return (
    <article className="space-y-6">
      {back}
      <header className="flex flex-wrap items-center gap-3">
        <h1 className="min-w-0 text-heading-4 font-semibold break-words">{title}</h1>
        <StatusBadge status={status} />
      </header>
      <dl className="grid gap-x-6 gap-y-2 rounded-lg border border-border bg-card p-4 sm:grid-cols-[max-content_1fr]">
        <dt className="font-semibold">{t.fileName}</dt>
        <dd className="break-all">{fileName}</dd>
        <dt className="font-semibold">{t.createdBy}</dt>
        <dd>{createdBy.name}</dd>
        <dt className="font-semibold">{t.createdAt}</dt>
        <dd>
          <time dateTime={createdAt}>{formatDateTime(createdAt, locale)}</time>
        </dd>
      </dl>
      <section aria-labelledby="document-content-heading" className="space-y-2">
        <h2 id="document-content-heading" className="text-heading-5 font-semibold">
          {t.content}
        </h2>
        {/* Raw text, never rendered as Markdown/HTML (D-28). */}
        <div className="rounded-lg border border-border bg-card p-4 break-words whitespace-pre-wrap">
          {content}
        </div>
      </section>
    </article>
  );
}
