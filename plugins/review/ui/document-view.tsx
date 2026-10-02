import { errorStatus, useDocument, usePluginEnv } from "./api";
import { DocumentStatusBadge } from "./badges";
import { BackToCriteria, LoadError, Loading } from "./status";
import { textFor } from "./text";

/** Evidence document inside the plugin (D-22); reloadable from its own URL. */
export function DocumentView({ documentId }: { documentId: string }) {
  const { context } = usePluginEnv();
  const text = textFor(context.locale);
  const document = useDocument(documentId);

  if (document.isPending) return <Loading />;
  if (document.isError) {
    // 400 (malformed id in the URL) and 404 both mean "no such document" (D-47).
    const status = errorStatus(document.error);
    return (
      <div className="space-y-4">
        {status === 404 || status === 400 ? (
          <p role="alert">{text.documentNotFound}</p>
        ) : (
          <LoadError retry={() => document.refetch()} />
        )}
        <BackToCriteria />
      </div>
    );
  }

  const { title, fileName, status, content } = document.data;
  return (
    <article className="space-y-6">
      <BackToCriteria />
      <header className="flex flex-wrap items-center gap-3">
        <h1 className="min-w-0 text-heading-4 font-semibold break-words">{title}</h1>
        <DocumentStatusBadge status={status} />
      </header>
      <dl className="grid gap-x-6 gap-y-2 rounded-lg border border-border bg-card p-4 sm:grid-cols-[max-content_1fr]">
        <dt className="font-semibold">{text.fileName}</dt>
        <dd className="break-all">{fileName}</dd>
      </dl>
      <section aria-labelledby="evidence-content-heading" className="space-y-2">
        <h2 id="evidence-content-heading" className="text-heading-5 font-semibold">
          {text.content}
        </h2>
        {/* Raw text, never rendered as Markdown/HTML (D-28). */}
        <div className="rounded-lg border border-border bg-card p-4 break-words whitespace-pre-wrap">
          {content}
        </div>
      </section>
    </article>
  );
}
