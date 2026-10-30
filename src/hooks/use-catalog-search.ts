import { useCallback } from "react";
import { searchCatalog } from "@/lib/api";
import type { CatalogSearchResult } from "@/lib/catalog";
import { useResource } from "./use-resource";

/** Mount it per query (`key`): a new query then starts in the loading state. */
export function useCatalogSearch(query: string) {
  const load = useCallback(() => searchCatalog(query), [query]);
  const resource = useResource<CatalogSearchResult | null>(load, null);
  return { ...resource, result: resource.data };
}
