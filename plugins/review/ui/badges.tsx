import { Badge, type BadgeProps } from "@biyard/components";
import type { DocumentStatus } from "@interview/api-types/DocumentStatus";
import type { ReviewDecision } from "@interview/api-types/ReviewDecision";
import { usePluginEnv } from "./api";
import { textFor, type TextKey } from "./text";

const documentStatusView: Record<
  DocumentStatus,
  { label: TextKey; variant: NonNullable<BadgeProps["variant"]> }
> = {
  ready: { label: "statusReady", variant: "success" },
  processing: { label: "statusProcessing", variant: "warning" },
  failed: { label: "statusFailed", variant: "danger" },
};

export function DocumentStatusBadge({ status }: { status: DocumentStatus }) {
  const { context } = usePluginEnv();
  const view = documentStatusView[status];
  return <Badge variant={view.variant}>{textFor(context.locale)[view.label]}</Badge>;
}

const decisionView: Record<
  ReviewDecision | "unwritten",
  { label: TextKey; variant: NonNullable<BadgeProps["variant"]> }
> = {
  satisfied: { label: "satisfied", variant: "success" },
  needs_information: { label: "needsInformation", variant: "warning" },
  unwritten: { label: "unwritten", variant: "default" },
};

/** Three distinct badges: satisfied, needs information, not written (README). */
export function DecisionBadge({ decision }: { decision: ReviewDecision | null }) {
  const { context } = usePluginEnv();
  const view = decisionView[decision ?? "unwritten"];
  return <Badge variant={view.variant}>{textFor(context.locale)[view.label]}</Badge>;
}
