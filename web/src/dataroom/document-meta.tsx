import { Badge, type BadgeProps } from "@biyard/components";
import type { Locale } from "@interview/plugin-sdk";
import type { DocumentStatus } from "@interview/api-client/types/DocumentStatus";
import { useI18n, type MessageKey } from "../i18n";

const statusView: Record<
  DocumentStatus,
  { label: MessageKey; variant: NonNullable<BadgeProps["variant"]> }
> = {
  ready: { label: "statusReady", variant: "success" },
  processing: { label: "statusProcessing", variant: "warning" },
  failed: { label: "statusFailed", variant: "danger" },
};

export function StatusBadge({ status }: { status: DocumentStatus }) {
  const { t } = useI18n();
  const view = statusView[status];
  return <Badge variant={view.variant}>{t[view.label]}</Badge>;
}

/** ISO 8601 UTC from the API, shown in the browser's time zone. */
export function formatDateTime(value: string, locale: Locale) {
  return new Intl.DateTimeFormat(locale === "ko" ? "ko-KR" : "en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
