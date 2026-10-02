import { createContext, useContext } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  isPluginRpcError,
  scopedKey,
  type PluginContext,
  type PluginHost,
} from "@interview/plugin-sdk";
import type { DocumentDetail } from "@interview/api-types/DocumentDetail";
import type { ListCriteriaResponse } from "@interview/api-types/ListCriteriaResponse";
import type { ListDocumentsResponse } from "@interview/api-types/ListDocumentsResponse";
import type { MyProgressResponse } from "@interview/api-types/MyProgressResponse";
import type { Review } from "@interview/api-types/Review";
import type { SaveReviewParams } from "@interview/api-types/SaveReviewParams";

export const PluginEnv = createContext<{ host: PluginHost; context: PluginContext } | null>(null);

export function usePluginEnv() {
  const env = useContext(PluginEnv);
  if (!env) throw new Error("PluginEnv is missing");
  return env;
}

// Every key starts with scopedKey(context, …) so users and workspaces never share cache.
const keys = {
  criteria: (context: PluginContext) => scopedKey(context, "review", "criteria"),
  mine: (context: PluginContext) => scopedKey(context, "review", "mine"),
  myReview: (context: PluginContext, criterionId: string) =>
    [...keys.mine(context), "review", criterionId] as const,
  myProgress: (context: PluginContext) => [...keys.mine(context), "progress"] as const,
  documents: (context: PluginContext) => scopedKey(context, "review", "documents"),
  document: (context: PluginContext, id: string) =>
    [...keys.documents(context), "detail", id] as const,
};

export function errorStatus(error: unknown): number | null {
  return isPluginRpcError(error) ? error.status : null;
}

export function useCriteria() {
  const { host, context } = usePluginEnv();
  return useQuery({
    queryKey: keys.criteria(context),
    queryFn: ({ signal }) => host.call<ListCriteriaResponse>("listCriteria", null, { signal }),
  });
}

export function useMyReview(criterionId: string) {
  const { host, context } = usePluginEnv();
  return useQuery({
    queryKey: keys.myReview(context, criterionId),
    queryFn: ({ signal }) => host.call<Review | null>("getMyReview", { criterionId }, { signal }),
  });
}

// Invalidated together with every "mine" key after a save.
export function useMyProgress() {
  const { host, context } = usePluginEnv();
  return useQuery({
    queryKey: keys.myProgress(context),
    queryFn: ({ signal }) => host.call<MyProgressResponse>("getMyProgress", null, { signal }),
  });
}

// Documents come from the host API (CLAUDE.md: target "dataroom").
export function useDocuments() {
  const { host, context } = usePluginEnv();
  return useQuery({
    queryKey: keys.documents(context),
    queryFn: ({ signal }) =>
      host.call<ListDocumentsResponse>("listDocuments", {}, { target: "dataroom", signal }),
  });
}

export function useDocument(id: string) {
  const { host, context } = usePluginEnv();
  return useQuery({
    queryKey: keys.document(context, id),
    queryFn: ({ signal }) =>
      host.call<DocumentDetail>("getDocument", { id }, { target: "dataroom", signal }),
  });
}

export function useSaveReview() {
  const { host, context } = usePluginEnv();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (params: SaveReviewParams) => host.call<Review>("saveReview", params),
    onSuccess: async (review) => {
      queryClient.setQueryData(keys.myReview(context, review.criterionId), review);
      await queryClient.invalidateQueries({ queryKey: keys.mine(context) });
    },
  });
}
