import { Button } from "@biyard/components";
import { useCriteria, usePluginEnv } from "./api";
import { LoadError, Loading } from "./status";
import { textFor } from "./text";

export function Home() {
  const { host, context } = usePluginEnv();
  const text = textFor(context.locale);
  const criteria = useCriteria();
  const isInvestor = context.user.role === "investor";

  return (
    <section aria-labelledby="review-home-heading" className="space-y-6">
      <h1 id="review-home-heading" className="text-heading-4 font-semibold">
        {text.title}
      </h1>
      {isInvestor ? null : (
        <p role="note" className="rounded-lg border border-border bg-card p-4">
          {text.companyNotice}
        </p>
      )}
      <h2 className="text-heading-5 font-semibold">{text.criteriaHeading}</h2>
      {criteria.isPending ? (
        <Loading />
      ) : criteria.isError ? (
        <LoadError retry={() => criteria.refetch()} />
      ) : (
        <ul className="divide-y divide-border rounded-lg border border-border bg-card">
          {criteria.data.items.map((criterion) => (
            <li
              key={criterion.id}
              className="flex flex-wrap items-center justify-between gap-3 px-4 py-3"
            >
              <div className="min-w-0 flex-1 basis-60">
                <p className="font-semibold">{criterion.title}</p>
                <p className="text-muted-foreground">{criterion.reviewQuestion}</p>
              </div>
              {isInvestor ? (
                <Button
                  variant="outline"
                  className="min-h-11"
                  onClick={() => host.navigate(`/criteria/${encodeURIComponent(criterion.id)}`)}
                  aria-label={`${criterion.title} ${text.writeReview}`}
                >
                  {text.writeReview}
                </Button>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
