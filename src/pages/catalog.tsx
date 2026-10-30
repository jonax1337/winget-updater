import { useState } from "react";
import { Search, SearchX } from "lucide-react";
import { CatalogGrid, CatalogGridSkeleton, type CatalogActions } from "@/components/catalog-card";
import { ManagerErrors } from "@/components/manager-errors";
import { ResourceError } from "@/components/resource-error";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { useCatalogInstall } from "@/hooks/use-catalog-install";
import { useCatalogSearch } from "@/hooks/use-catalog-search";
import { useDebouncedValue } from "@/hooks/use-debounced-value";
import { useInstalled } from "@/hooks/use-installed";
import { useUpdater } from "@/hooks/use-updater";
import {
  appsInCategory,
  catalogStatus,
  CATEGORY_FILTERS,
  SEARCH_MAX_LENGTH,
  searchResultEntry,
  wingetIds,
  type CatalogFilter,
} from "@/lib/catalog";

const SEARCH_DEBOUNCE_MS = 400;

export function Catalog() {
  const { busy, ready, packages, settings } = useUpdater();
  const installed = useInstalled();
  const [search, setSearch] = useState("");
  const query = search.trim();
  const searchedQuery = useDebouncedValue(query, SEARCH_DEBOUNCE_MS);
  const { installingId, install } = useCatalogInstall(installed.reload);
  const installedIds = wingetIds(installed.installed?.packages ?? []);
  const updatableIds = wingetIds(packages);
  const actions: CatalogActions = {
    statusOf: (id) => catalogStatus(id, installedIds, updatableIds),
    installingId,
    installDisabled:
      busy ||
      !ready ||
      settings.saving ||
      installed.loading ||
      installed.reloading ||
      installingId !== undefined,
    install: (entry) => void install(entry),
  };

  return (
    <div className="flex flex-col gap-4 px-4 lg:px-6">
      <ResourceError error={installed.error} retry={installed.reload} />
      <ManagerErrors
        errors={installed.installed?.errors ?? []}
        failure="could not be read"
      />
      <InputGroup className="max-w-sm">
        <InputGroupAddon>
          <Search />
        </InputGroupAddon>
        <InputGroupInput
          aria-label="Search the WinGet catalog"
          placeholder="Search all WinGet packages …"
          maxLength={SEARCH_MAX_LENGTH}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
      </InputGroup>
      {query ? (
        <SearchResults query={searchedQuery} settled={searchedQuery === query} actions={actions} />
      ) : (
        <Browse actions={actions} />
      )}
    </div>
  );
}

function Browse({ actions }: { actions: CatalogActions }) {
  const [filter, setFilter] = useState<CatalogFilter>("all");
  return (
    <>
      <ToggleGroup
        type="single"
        variant="outline"
        className="flex-wrap"
        aria-label="Filter by category"
        value={filter}
        onValueChange={(value: CatalogFilter) => value && setFilter(value)}
      >
        {CATEGORY_FILTERS.map(({ value, label }) => (
          <ToggleGroupItem key={value} value={value}>
            {label}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
      <CatalogGrid entries={appsInCategory(filter)} actions={actions} />
    </>
  );
}

/** Results are keyed by query so every new search starts with the loading skeletons. */
function SearchResults({
  query,
  settled,
  actions,
}: {
  query: string;
  settled: boolean;
  actions: CatalogActions;
}) {
  if (!settled) return <CatalogGridSkeleton />;
  return <SearchResultsFor key={query} query={query} actions={actions} />;
}

function SearchResultsFor({ query, actions }: { query: string; actions: CatalogActions }) {
  const search = useCatalogSearch(query);
  if (search.error) return <ResourceError error={search.error} retry={search.refresh} />;
  if (search.loading) return <CatalogGridSkeleton />;
  const packages = search.result?.packages ?? [];
  return (
    <>
      <ManagerErrors errors={search.result?.errors ?? []} failure="search was incomplete" />
      {packages.length ? (
        <CatalogGrid entries={packages.map(searchResultEntry)} actions={actions} />
      ) : (
        <NoResults query={query} />
      )}
    </>
  );
}

function NoResults({ query }: { query: string }) {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <SearchX />
        </EmptyMedia>
        <EmptyTitle>No packages found</EmptyTitle>
        <EmptyDescription>Nothing in WinGet matches “{query}”. Try a different search term.</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
