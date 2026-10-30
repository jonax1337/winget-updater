import type { ComponentProps, ReactNode } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";

/** Opens through `trigger`, or from outside via `open` and `onOpenChange`. */
export function ConfirmAction({
  trigger,
  title,
  description,
  confirm,
  onConfirm,
  ...dialog
}: {
  trigger?: ReactNode;
  title: string;
  description: string;
  confirm: string;
  onConfirm: () => void;
} & Pick<ComponentProps<typeof AlertDialog>, "open" | "onOpenChange">) {
  return (
    <AlertDialog {...dialog}>
      {trigger && <AlertDialogTrigger asChild>{trigger}</AlertDialogTrigger>}
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Abbrechen</AlertDialogCancel>
          <AlertDialogAction variant="destructive" onClick={onConfirm}>
            {confirm}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
