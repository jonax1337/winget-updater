import { useDeferredValue, useState } from "react";
import { Search, SearchX } from "lucide-react";
import { Combobox } from "@/components/combobox";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import {
  MANAGER_FILTERS,
  matchesFilter,
  type ManagerFilter,
  type NamedPackageRef,
} from "@/lib/packages";

export function usePackageFilters() {
  const [search, setSearch] = useState("");
  const [manager, setManager] = useState<ManagerFilter>("all");
  const query = useDeferredValue(search.trim().toLocaleLowerCase("de-DE"));
  return {
    search,
    setSearch,
    manager,
    setManager,
    matches: (entry: NamedPackageRef) => matchesFilter(entry, manager, query),
  };
}

export function PackageFilters({ filters }: { filters: ReturnType<typeof usePackageFilters> }) {
  return (
    <>
      <InputGroup className="max-w-sm">
        <InputGroupAddon>
          <Search />
        </InputGroupAddon>
        <InputGroupInput
          aria-label="Pakete durchsuchen"
          placeholder="Name oder Paket-ID suchen …"
          value={filters.search}
          onChange={(event) => filters.setSearch(event.target.value)}
        />
      </InputGroup>
      <Combobox
        label="Nach Quelle filtern"
        value={filters.manager}
        options={MANAGER_FILTERS}
        onChange={filters.setManager}
        className="w-44"
      />
    </>
  );
}

export function NoMatches() {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <SearchX />
        </EmptyMedia>
        <EmptyTitle>Keine passenden Pakete</EmptyTitle>
        <EmptyDescription>Anderen Suchbegriff oder eine andere Quelle wählen.</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
