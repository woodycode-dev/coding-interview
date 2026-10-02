import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ApiError } from "@interview/api-client/runtime/client";
import { dataroomRpcHandler } from "@interview/api-client/handlers/dataroomRpcHandler";
import type { CreateDocumentParams } from "@interview/api-client/types/CreateDocumentParams";
import type { DataroomInfo } from "@interview/api-client/types/DataroomInfo";
import type { DocumentDetail } from "@interview/api-client/types/DocumentDetail";
import type { ListDocumentsResponse } from "@interview/api-client/types/ListDocumentsResponse";
import { expireSession } from "../auth/session";

export const dataroomKeys = {
  info: (workspaceId: string) => ["dataroom", workspaceId, "info"] as const,
  documentLists: (workspaceId: string) => ["dataroom", workspaceId, "documents", "list"] as const,
  documentList: (workspaceId: string, query: string) =>
    [...dataroomKeys.documentLists(workspaceId), query] as const,
  document: (workspaceId: string, id: string) =>
    ["dataroom", workspaceId, "documents", "detail", id] as const,
};

export function dataroomPath(workspaceId: string, ...segments: string[]) {
  return [
    `/workspace/${encodeURIComponent(workspaceId)}`,
    ...segments.map(encodeURIComponent),
  ].join("/");
}

async function call<T>(workspaceId: string, method: string, params: unknown): Promise<T> {
  try {
    const response = await dataroomRpcHandler({ workspaceId, method, params: params ?? null });
    return response.result as T;
  } catch (error) {
    // Same as the plugin host: an expired session sends the user back to sign-in.
    if (error instanceof ApiError && error.status === 401) expireSession();
    throw error;
  }
}

export function errorStatus(error: unknown): number | null {
  return error instanceof ApiError ? error.status : null;
}

export function useDataroom(workspaceId: string) {
  return useQuery({
    queryKey: dataroomKeys.info(workspaceId),
    queryFn: () => call<DataroomInfo>(workspaceId, "getDataroom", null),
  });
}

export function useDocuments(workspaceId: string, query: string) {
  return useQuery({
    queryKey: dataroomKeys.documentList(workspaceId, query),
    queryFn: () =>
      call<ListDocumentsResponse>(workspaceId, "listDocuments", query ? { query } : {}),
  });
}

export function useDocument(workspaceId: string, id: string) {
  return useQuery({
    queryKey: dataroomKeys.document(workspaceId, id),
    queryFn: () => call<DocumentDetail>(workspaceId, "getDocument", { id }),
  });
}

export function useCreateDocument(workspaceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (params: CreateDocumentParams) =>
      call<DocumentDetail>(workspaceId, "createDocument", params),
    onSuccess: async (document) => {
      queryClient.setQueryData(dataroomKeys.document(workspaceId, document.id), document);
      await queryClient.invalidateQueries({ queryKey: dataroomKeys.documentLists(workspaceId) });
    },
  });
}
