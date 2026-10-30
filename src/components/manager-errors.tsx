import { TriangleAlert } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { MANAGER_NAMES } from "@/lib/packages";
import type { ManagerError } from "@/lib/types";

export function ManagerErrors({
  errors,
  failure,
}: {
  errors: ManagerError[];
  failure: string;
}) {
  return (
    <>
      {errors.map((error) => (
        <Alert key={`${error.manager}:${error.message}`}>
          <TriangleAlert className="text-warning" />
          <AlertTitle>
            {MANAGER_NAMES[error.manager]} {failure}
          </AlertTitle>
          <AlertDescription className="break-words">{error.message}</AlertDescription>
        </Alert>
      ))}
    </>
  );
}
