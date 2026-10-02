import { Route, Routes } from "react-router-dom";
import type { HostSession } from "../../../shared/platform";
import { useI18n } from "../i18n";
import { DocumentCreate } from "./document-create";
import { DocumentDetail } from "./document-detail";
import { DocumentList } from "./document-list";

export function DataroomApp({ session }: { session: HostSession }) {
  const { t } = useI18n();
  const workspaceId = session.workspace.id;
  // UI hint only; the server enforces upload permission (spec 1.3).
  const canUpload = session.user.role === "company";
  return (
    <Routes>
      <Route index element={<DocumentList workspaceId={workspaceId} canUpload={canUpload} />} />
      <Route
        path="documents/new"
        element={<DocumentCreate workspaceId={workspaceId} canUpload={canUpload} />}
      />
      <Route path="documents/:documentId" element={<DocumentDetail workspaceId={workspaceId} />} />
      <Route path="*" element={<p role="alert">{t.pageNotFound}</p>} />
    </Routes>
  );
}
