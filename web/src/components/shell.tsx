import { useMutation } from "@tanstack/react-query";
import { NavLink, Navigate, useParams } from "react-router-dom";
import { Button, cn } from "@biyard/components";
import type { HostSession } from "../../../shared/platform";
import type { Locale } from "@interview/plugin-sdk";
import { authAdapter } from "../auth/adapter";
import { expireSession } from "../auth/session";
import { useI18n } from "../i18n";
import { RuntimePlugin } from "../plugins/runtime";
import { usePlugins } from "../api/hooks/use-plugins";
import { DataroomApp } from "../dataroom/app";
import { ConnectionError } from "./connection-error";

export function Shell({ session }: { session: HostSession }) {
  const { t, locale, setLocale } = useI18n();
  const { workspaceId, pluginId, "*": rest = "" } = useParams();
  const { user, workspace } = session;
  const logout = useMutation({ mutationFn: () => authAdapter.logout(), onSuccess: expireSession });
  const catalog = usePlugins();
  const pluginCatalog = catalog.data ?? [];
  const plugin = pluginCatalog.find((item) => item.id === pluginId);
  if (!workspaceId)
    return <Navigate to={`/workspace/${encodeURIComponent(workspace.id)}`} replace />;
  return (
    <div className="min-h-screen md:grid md:grid-cols-[232px_1fr]">
      <aside className="flex flex-col gap-6 border-b border-border bg-card p-6 md:border-r md:border-b-0">
        <span className="text-heading-5 font-semibold">{t.appName}</span>
        <nav aria-label={t.menu} className="flex flex-wrap gap-2 md:flex-col">
          <NavLink
            end
            to={`/workspace/${encodeURIComponent(workspace.id)}`}
            className="min-h-11 rounded-md px-3 py-2"
          >
            {t.room}
          </NavLink>
          {pluginCatalog.map((item) => (
            <NavLink
              key={item.id}
              to={`/workspace/${encodeURIComponent(workspace.id)}/plugins/${item.id}`}
              className={({ isActive }) =>
                cn(
                  "min-h-11 rounded-md px-3 py-2",
                  isActive
                    ? "bg-secondary font-semibold text-secondary-foreground"
                    : "text-muted-foreground hover:bg-accent",
                )
              }
            >
              {item.name}
            </NavLink>
          ))}
          {catalog.isError ? <ConnectionError retry={() => catalog.refetch()} /> : null}
        </nav>
      </aside>
      <div className="min-w-0">
        <header className="flex flex-wrap items-center justify-between gap-4 border-b border-border bg-card px-6 py-4">
          <div>
            <p className="font-semibold">{workspace.name}</p>
            <p className="text-caption text-muted-foreground">
              {user.name} · {t[user.role]}
            </p>
          </div>
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-caption">
              {t.language}
              <select
                value={locale}
                onChange={(event) => setLocale(event.target.value as Locale)}
                className="min-h-11 rounded-md border border-input bg-card px-3"
              >
                <option value="ko">한국어</option>
                <option value="en">English</option>
              </select>
            </label>
            <Button variant="outline" onClick={() => logout.mutate()} disabled={logout.isPending}>
              {t.logout}
            </Button>
          </div>
          {logout.isError ? <p role="alert">{t.logoutError}</p> : null}
        </header>
        <main className="mx-auto max-w-6xl p-6 md:p-8">
          {workspaceId !== workspace.id ? (
            <p role="alert">{t.noWorkspace}</p>
          ) : !pluginId ? (
            <DataroomApp session={session} />
          ) : catalog.isPending ? (
            <p role="status">{t.loading}</p>
          ) : catalog.isError ? (
            <ConnectionError retry={() => catalog.refetch()} />
          ) : !plugin ? (
            <p role="alert">{t.noPlugin}</p>
          ) : (
            <RuntimePlugin
              key={`${user.id}:${workspace.id}:${plugin.id}`}
              plugin={plugin}
              context={{ user, workspaceId: workspace.id, locale, location: `/${rest}` }}
            />
          )}
        </main>
      </div>
    </div>
  );
}
