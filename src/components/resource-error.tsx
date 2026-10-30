import { AlertCircle, RotateCcw } from "lucide-react";
import { Alert, AlertAction, AlertDescription, AlertTitle } from "@/components/ui/alert";
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
    <Alert variant="destructive">
      <AlertCircle />
      <AlertTitle>Could not load data</AlertTitle>
      <AlertDescription>{error.message}</AlertDescription>
      <AlertAction>
        <Button variant="outline" size="sm" onClick={retry}>
          <RotateCcw data-icon="inline-start" />
          Retry
        </Button>
      </AlertAction>
    </Alert>
  );
}
