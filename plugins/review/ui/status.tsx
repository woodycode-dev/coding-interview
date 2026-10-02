import { Button } from "@biyard/components";
import { usePluginEnv } from "./api";
import { textFor } from "./text";

// Screen states follow the host SessionGate pattern: pending → status, error → retry, data.

export function Loading() {
  const { context } = usePluginEnv();
  return (
    <p role="status" className="text-muted-foreground">
      {textFor(context.locale).loading}
    </p>
  );
}

export function LoadError({ retry }: { retry(): void }) {
  const { context } = usePluginEnv();
  const text = textFor(context.locale);
  return (
    <div
      role="alert"
      className="flex flex-col items-start gap-4 rounded-lg border border-border bg-card p-6"
    >
      <p>{text.loadError}</p>
      <Button variant="outline" className="min-h-11" onClick={retry}>
        {text.retry}
      </Button>
    </div>
  );
}

export function BackToCriteria() {
  const { host, context } = usePluginEnv();
  return (
    <Button variant="ghost" className="min-h-11 px-0 underline" onClick={() => host.navigate("/")}>
      {textFor(context.locale).backToCriteria}
    </Button>
  );
}
