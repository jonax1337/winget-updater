import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

export function EmptyState({
  icon: Icon,
  title,
  description,
  children,
}: {
  icon: LucideIcon;
  title: string;
  description: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-center px-6 py-16 text-center">
      <div className="mb-5 flex size-16 items-center justify-center rounded-2xl border bg-muted/50 text-muted-foreground">
        <Icon className="size-7" strokeWidth={1.4} />
      </div>
      <h3 className="text-base font-semibold tracking-tight">{title}</h3>
      <p className="mt-2 max-w-md text-sm leading-relaxed text-muted-foreground">
        {description}
      </p>
      {children && <div className="mt-5">{children}</div>}
    </div>
  );
}
