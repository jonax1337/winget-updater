import { AlertCircle, RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { AppError } from "@/lib/types";

export function ResourceError({
  error,
  retry,
}: {
  error?: AppError;
  retry: () => void;
}) {
  if (!error) return null;
  return (
    <div
      role="alert"
      className="flex flex-wrap items-center gap-3 rounded-xl border border-destructive/25 bg-destructive/5 p-4 text-sm"
    >
      <AlertCircle className="size-5 shrink-0 text-destructive" />
      <p className="min-w-0 flex-1 break-words">{error.message}</p>
      <Button variant="outline" size="sm" onClick={retry}>
        <RotateCcw />
        Erneut versuchen
      </Button>
    </div>
  );
}
