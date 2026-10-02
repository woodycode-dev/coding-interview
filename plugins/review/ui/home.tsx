import { Button } from "@biyard/components";
import { useCriteria, useMyProgress, usePluginEnv } from "./api";
import { DecisionBadge, DocumentStatusBadge } from "./badges";
import { LoadError, Loading } from "./status";
import { textFor } from "./text";

/** `savedCriterionId` comes from `?saved=` set by the review form after a successful save. */
export function Home({ savedCriterionId }: { savedCriterionId: string | null }) {
  const { context } = usePluginEnv();
  const text = textFor(context.locale);
  return (
    <section aria-labelledby="review-home-heading" className="space-y-6">
      <h1 id="review-home-heading" className="text-heading-4 font-semibold">
        {text.title}
      </h1>
      {context.user.role === "investor" ? (
        <InvestorProgress savedCriterionId={savedCriterionId} />
      ) : (
        <CompanyCriteria />
      )}
    </section>
  );
}

/** Company members see the criteria only; review APIs are not called (D-6, D-29). */
function CompanyCriteria() {
  const { context } = usePluginEnv();
  const text = textFor(context.locale);
  const criteria = useCriteria();
  return (
    <>
      <p role="note" className="rounded-lg border border-border bg-card p-4">
        {text.companyNotice}
      </p>
      <h2 className="text-heading-5 font-semibold">{text.criteriaHeading}</h2>
      {criteria.isPending ? (
        <Loading />
      ) : criteria.isError ? (
        <LoadError retry={() => criteria.refetch()} />
      ) : (
        <ul className="divide-y divide-border rounded-lg border border-border bg-card">
          {criteria.data.items.map((criterion) => (
            <li key={criterion.id} className="px-4 py-3">
              <p className="font-semibold">{criterion.title}</p>
              <p className="text-muted-foreground">{criterion.reviewQuestion}</p>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

function InvestorProgress({ savedCriterionId }: { savedCriterionId: string | null }) {
  const { host, context } = usePluginEnv();
  const text = textFor(context.locale);
  const progress = useMyProgress();

  if (progress.isPending) return <Loading />;
  if (progress.isError) return <LoadError retry={() => progress.refetch()} />;

  const { writtenCount, unwrittenCount, satisfiedCount, needsInformationCount, items } =
    progress.data;
  const saved = items.find((item) => item.criterion.id === savedCriterionId);
  return (
    <>
      <section
        aria-labelledby="progress-heading"
        className="space-y-2 rounded-lg border border-border bg-card p-4"
      >
        <h2 id="progress-heading" className="text-heading-5 font-semibold">
          {text.progressHeading}
        </h2>
        <p className="text-heading-5">
          {text.written} {writtenCount} / {text.unwritten} {unwrittenCount}
        </p>
        <p className="text-muted-foreground">
          {text.satisfied} {satisfiedCount} · {text.needsInformation} {needsInformationCount}
        </p>
        <p role="note" className="text-caption text-muted-foreground">
          {text.progressNotice}
        </p>
      </section>
      {saved ? (
        <p role="status" className="text-success">
          {text.savedCriterion} {saved.criterion.title}
        </p>
      ) : null}
      <ul className="divide-y divide-border rounded-lg border border-border bg-card">
        {items.map(({ criterion, review }) => (
          <li key={criterion.id} className="space-y-3 px-4 py-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <h3 className="font-semibold">{criterion.title}</h3>
                <DecisionBadge decision={review?.decision ?? null} />
              </div>
              <Button
                variant="outline"
                className="min-h-11"
                onClick={() => host.navigate(`/criteria/${encodeURIComponent(criterion.id)}`)}
                aria-label={`${criterion.title} ${text.writeReview}`}
              >
                {text.writeReview}
              </Button>
            </div>
            <p className="text-muted-foreground">{criterion.reviewQuestion}</p>
            {review ? (
              <>
                <p className="line-clamp-2 break-words whitespace-pre-wrap">{review.comment}</p>
                <div className="flex flex-wrap items-center gap-2">
                  <span className="text-caption font-semibold">{text.evidence}</span>
                  {review.evidence.map((evidence) => (
                    <Button
                      key={evidence.documentId}
                      variant="ghost"
                      size="sm"
                      className="min-h-11 gap-2 underline"
                      onClick={() =>
                        host.navigate(`/documents/${encodeURIComponent(evidence.documentId)}`)
                      }
                    >
                      {evidence.title}
                      {/* Evidence may have stopped being ready after saving (D-52). */}
                      {evidence.status === "ready" ? null : (
                        <DocumentStatusBadge status={evidence.status} />
                      )}
                    </Button>
                  ))}
                </div>
              </>
            ) : null}
          </li>
        ))}
      </ul>
    </>
  );
}
