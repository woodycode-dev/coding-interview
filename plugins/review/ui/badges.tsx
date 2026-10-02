import { Badge, type BadgeProps } from "@biyard/components";
import type { DocumentStatus } from "@interview/api-types/DocumentStatus";
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
