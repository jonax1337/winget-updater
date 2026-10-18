import { AlertDialog } from "radix-ui";
import { Button, buttonVariants } from "@/components/ui/button";
import type { ReactNode } from "react";

export function ConfirmAction({
  trigger,
  title,
  description,
  confirm,
  onConfirm,
}: {
  trigger: ReactNode;
  title: string;
  description: string;
  confirm: string;
  onConfirm: () => void;
}) {
  return (
    <AlertDialog.Root>
      <AlertDialog.Trigger asChild>{trigger}</AlertDialog.Trigger>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="fixed inset-0 z-50 bg-black/45" />
        <AlertDialog.Content className="fixed left-1/2 top-1/2 z-50 w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 rounded-2xl border bg-popover p-6 text-popover-foreground shadow-xl">
          <AlertDialog.Title className="text-lg font-semibold tracking-tight">
            {title}
          </AlertDialog.Title>
          <AlertDialog.Description className="mb-6 mt-2 text-sm leading-relaxed text-muted-foreground">
            {description}
          </AlertDialog.Description>
          <div className="flex justify-end gap-2">
            <AlertDialog.Cancel asChild>
              <Button variant="outline">Abbrechen</Button>
            </AlertDialog.Cancel>
            <AlertDialog.Action
              className={buttonVariants({ variant: "destructive" })}
              onClick={onConfirm}
            >
              {confirm}
            </AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}
