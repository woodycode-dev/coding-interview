import type { PluginProps } from "@interview/plugin-sdk/react";
import { PluginEnv } from "./api";
import { DocumentView } from "./document-view";
import { Home } from "./home";
import { ReviewForm } from "./review-form";
import { BackToCriteria } from "./status";
import { textFor } from "./text";

type Route =
  | { name: "home" }
  | { name: "criterion"; id: string }
  | { name: "document"; id: string }
  | { name: "notFound" };

/** Plugin-internal paths come from `context.location` (e.g. `/criteria/team`). */
function parseRoute(location: string): Route {
  const path = location.split(/[?#]/, 1)[0];
  let parts: string[];
  try {
    parts = path.split("/").filter(Boolean).map(decodeURIComponent);
  } catch {
    return { name: "notFound" };
  }
  if (parts.length === 0) return { name: "home" };
  if (parts.length === 2 && parts[0] === "criteria") return { name: "criterion", id: parts[1] };
  if (parts.length === 2 && parts[0] === "documents") return { name: "document", id: parts[1] };
  return { name: "notFound" };
}

export const App: React.FC<PluginProps> = ({ host, context }) => {
  const route = parseRoute(context.location);
  return (
    <PluginEnv.Provider value={{ host, context }}>
      {route.name === "home" ? (
        <Home />
      ) : route.name === "criterion" ? (
        <ReviewForm key={route.id} criterionId={route.id} />
      ) : route.name === "document" ? (
        <DocumentView key={route.id} documentId={route.id} />
      ) : (
        <div className="space-y-4">
          <p role="alert">{textFor(context.locale).pageNotFound}</p>
          <BackToCriteria />
        </div>
      )}
    </PluginEnv.Provider>
  );
};
