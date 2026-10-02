import { useId, useRef, useState, type FormEvent } from "react";
import { Button, Label, Textarea } from "@biyard/components";
import type { Criterion } from "@interview/api-types/Criterion";
import type { DocumentSummary } from "@interview/api-types/DocumentSummary";
import type { EvidenceRef } from "@interview/api-types/EvidenceRef";
import type { Review } from "@interview/api-types/Review";
import type { ReviewDecision } from "@interview/api-types/ReviewDecision";
import {
  errorStatus,
  useCriteria,
  useDocuments,
  useMyReview,
  usePluginEnv,
  useSaveReview,
} from "./api";
import { DocumentStatusBadge } from "./badges";
import { commentLength, validateComment, validateEvidence } from "./review-input";
import { BackToCriteria, LoadError, Loading } from "./status";
import { textFor, type TextKey } from "./text";

const DECISIONS: { value: ReviewDecision; label: TextKey }[] = [
  { value: "satisfied", label: "satisfied" },
  { value: "needs_information", label: "needsInformation" },
];

export function ReviewForm({ criterionId }: { criterionId: string }) {
  const { context } = usePluginEnv();
  const text = textFor(context.locale);
  // UI hint only; the server rejects company saves before validation (spec 1.3).
  if (context.user.role !== "investor")
    return (
      <div className="space-y-4">
        <p role="alert">{text.investorOnly}</p>
        <BackToCriteria />
      </div>
    );
  return <InvestorReviewForm criterionId={criterionId} />;
}

function InvestorReviewForm({ criterionId }: { criterionId: string }) {
  const { context } = usePluginEnv();
  const text = textFor(context.locale);
  const criteria = useCriteria();
  const review = useMyReview(criterionId);
  const documents = useDocuments();

  const reviewStatus = errorStatus(review.error);
  if (reviewStatus === 404 || reviewStatus === 400)
    return (
      <div className="space-y-4">
        <p role="alert">{text.criterionNotFound}</p>
        <BackToCriteria />
      </div>
    );
  if (criteria.isPending || review.isPending || documents.isPending) return <Loading />;
  if (criteria.isError || review.isError || documents.isError)
    return (
      <LoadError
        retry={() => {
          if (criteria.isError) void criteria.refetch();
          if (review.isError) void review.refetch();
          if (documents.isError) void documents.refetch();
        }}
      />
    );
  const criterion = criteria.data.items.find((item) => item.id === criterionId);
  if (!criterion)
    return (
      <div className="space-y-4">
        <p role="alert">{text.criterionNotFound}</p>
        <BackToCriteria />
      </div>
    );
  return (
    <ReviewEditor
      key={criterionId}
      criterion={criterion}
      review={review.data}
      documents={documents.data.items}
      reloadDocuments={() => void documents.refetch()}
    />
  );
}

function saveErrorText(error: unknown): TextKey {
  const status = errorStatus(error);
  if (status === 400) return "saveInvalid";
  if (status === 404) return "saveNotFound";
  if (status === 403) return "saveForbidden";
  return "saveFailed";
}

function splitEvidence(review: Review | null, documents: DocumentSummary[]) {
  const status = new Map(documents.map((document) => [document.id, document.status]));
  const kept: string[] = [];
  const dropped: EvidenceRef[] = [];
  for (const evidence of review?.evidence ?? []) {
    if (status.get(evidence.documentId) === "ready") kept.push(evidence.documentId);
    else dropped.push(evidence);
  }
  return { kept, dropped };
}

function ReviewEditor({
  criterion,
  review,
  documents,
  reloadDocuments,
}: {
  criterion: Criterion;
  review: Review | null;
  documents: DocumentSummary[];
  reloadDocuments(): void;
}) {
  const { host, context } = usePluginEnv();
  const text = textFor(context.locale);
  const save = useSaveReview();
  const id = useId();
  const decisionRef = useRef<HTMLInputElement>(null);
  const commentRef = useRef<HTMLTextAreaElement>(null);
  const evidenceRef = useRef<HTMLUListElement>(null);
  // Form state lives in component memory only (D-30). Evidence that is no longer
  // ready is unselected up front so saving does not fail on it.
  const [initial] = useState(() => splitEvidence(review, documents));
  const [decision, setDecision] = useState<ReviewDecision | null>(review?.decision ?? null);
  const [comment, setComment] = useState(review?.comment ?? "");
  const [selected, setSelected] = useState<string[]>(initial.kept);
  const [errors, setErrors] = useState<{
    decision?: TextKey;
    comment?: TextKey;
    evidence?: TextKey;
  }>({});
  const [lastSaved, setLastSaved] = useState<string | null>(review?.updatedAt ?? null);
  const [justSaved, setJustSaved] = useState(false);
  const pending = save.isPending;

  function toggle(documentId: string, checked: boolean) {
    setSelected((current) =>
      checked ? [...current, documentId] : current.filter((value) => value !== documentId),
    );
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    const next = {
      decision: decision ? undefined : ("decisionRequired" as const),
      comment: validateComment(comment) ?? undefined,
      evidence: validateEvidence(selected) ?? undefined,
    };
    setErrors(next);
    setJustSaved(false);
    if (next.decision) decisionRef.current?.focus();
    else if (next.comment) commentRef.current?.focus();
    else if (next.evidence) evidenceRef.current?.querySelector<HTMLInputElement>("input")?.focus();
    if (!decision || next.comment || next.evidence) return;
    save.mutate(
      { criterionId: criterion.id, decision, comment, evidenceDocumentIds: selected },
      {
        onSuccess: (saved) => {
          setComment(saved.comment);
          setSelected(saved.evidence.map((evidence) => evidence.documentId));
          setLastSaved(saved.updatedAt);
          setJustSaved(true);
        },
        onError: (error) => {
          // Document statuses may have changed; show the current ones.
          const status = errorStatus(error);
          if (status === 400 || status === 404) reloadDocuments();
        },
      },
    );
  }

  const describedBy = (...ids: (string | false)[]) => ids.filter(Boolean).join(" ") || undefined;

  return (
    <form onSubmit={submit} noValidate className="max-w-3xl space-y-6">
      <BackToCriteria />
      <header className="space-y-2">
        <h1 className="text-heading-4 font-semibold">{criterion.title}</h1>
        <p>
          <span className="font-semibold">{text.reviewQuestion}: </span>
          {criterion.reviewQuestion}
        </p>
      </header>

      <fieldset
        className="space-y-2"
        aria-describedby={describedBy(!!errors.decision && `${id}-decision-error`)}
      >
        <legend className="text-body-sm font-semibold">{text.decision}</legend>
        <div className="flex flex-wrap gap-4">
          {DECISIONS.map((option, index) => (
            <label key={option.value} className="flex min-h-11 items-center gap-2">
              <input
                ref={index === 0 ? decisionRef : undefined}
                type="radio"
                name={`${id}-decision`}
                value={option.value}
                checked={decision === option.value}
                onChange={() => setDecision(option.value)}
                disabled={pending}
              />
              {text[option.label]}
            </label>
          ))}
        </div>
        {errors.decision ? (
          <p id={`${id}-decision-error`} className="text-destructive">
            {text[errors.decision]}
          </p>
        ) : null}
      </fieldset>

      <div className="flex flex-col gap-2">
        <Label htmlFor={`${id}-comment`}>{text.comment}</Label>
        {/* No maxLength: it counts UTF-16 units, the server counts code points (D-24). */}
        <Textarea
          ref={commentRef}
          id={`${id}-comment`}
          value={comment}
          onChange={(event) => setComment(event.target.value)}
          disabled={pending}
          rows={6}
          aria-invalid={errors.comment ? true : undefined}
          aria-describedby={describedBy(
            `${id}-comment-count`,
            !!errors.comment && `${id}-comment-error`,
          )}
        />
        <p id={`${id}-comment-count`} className="text-caption text-muted-foreground">
          {commentLength(comment)} / 2000 {text.commentCount}
        </p>
        {errors.comment ? (
          <p id={`${id}-comment-error`} className="text-destructive">
            {text[errors.comment]}
          </p>
        ) : null}
      </div>

      <fieldset
        className="space-y-3"
        aria-describedby={describedBy(
          `${id}-evidence-hint`,
          !!errors.evidence && `${id}-evidence-error`,
        )}
      >
        <legend className="text-body-sm font-semibold">{text.evidence}</legend>
        <p id={`${id}-evidence-hint`} className="text-caption text-muted-foreground">
          {text.evidenceHint}
        </p>
        {initial.dropped.length > 0 ? (
          <div role="note" className="rounded-lg border border-border bg-card p-3">
            <p>{text.droppedEvidence}</p>
            <ul className="list-disc pl-5">
              {initial.dropped.map((evidence) => (
                <li key={evidence.documentId}>{evidence.title}</li>
              ))}
            </ul>
          </div>
        ) : null}
        {documents.length === 0 ? (
          <p>{text.noDocuments}</p>
        ) : (
          <ul
            ref={evidenceRef}
            className="divide-y divide-border rounded-lg border border-border bg-card"
          >
            {documents.map((document, index) => {
              const checked = selected.includes(document.id);
              const ready = document.status === "ready";
              const inputId = `${id}-evidence-${index}`;
              return (
                <li key={document.id} className="flex flex-wrap items-center gap-3 px-4 py-3">
                  <input
                    id={inputId}
                    type="checkbox"
                    checked={checked}
                    // A selected document that stopped being ready can still be unselected.
                    disabled={pending || (!ready && !checked)}
                    onChange={(event) => toggle(document.id, event.target.checked)}
                    aria-describedby={ready ? undefined : `${inputId}-reason`}
                    className="size-5"
                  />
                  <label htmlFor={inputId} className="min-w-0 flex-1 basis-48">
                    <span className="block break-words">{document.title}</span>
                    <span className="block text-caption break-all text-muted-foreground">
                      {document.fileName}
                    </span>
                    {ready ? null : (
                      <span
                        id={`${inputId}-reason`}
                        className="block text-caption text-muted-foreground"
                      >
                        {text.notReadyReason}
                      </span>
                    )}
                  </label>
                  <DocumentStatusBadge status={document.status} />
                  <Button
                    type="button"
                    variant="ghost"
                    className="min-h-11"
                    onClick={() => host.navigate(`/documents/${encodeURIComponent(document.id)}`)}
                    aria-label={`${document.title} ${text.viewDocument}`}
                  >
                    {text.viewDocument}
                  </Button>
                </li>
              );
            })}
          </ul>
        )}
        {errors.evidence ? (
          <p id={`${id}-evidence-error`} className="text-destructive">
            {text[errors.evidence]}
          </p>
        ) : null}
      </fieldset>

      {save.isError ? (
        <p role="alert" className="text-destructive">
          {text[saveErrorText(save.error)]}
        </p>
      ) : null}
      <div className="flex flex-wrap items-center gap-4">
        <Button type="submit" disabled={pending} className="min-h-11">
          {pending ? text.saving : text.save}
        </Button>
        <p role="status" className="text-muted-foreground">
          {justSaved ? `${text.saved} ` : ""}
          {lastSaved ? `${text.lastSaved}: ${formatDateTime(lastSaved, context.locale)}` : ""}
        </p>
      </div>
    </form>
  );
}

function formatDateTime(value: string, locale: "ko" | "en") {
  return new Intl.DateTimeFormat(locale === "ko" ? "ko-KR" : "en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
