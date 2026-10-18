import { useId, useState, type ReactNode } from "react";
import { toast } from "sonner";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

export function SettingRow({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: (describedBy: string) => ReactNode;
}) {
  const descriptionId = useId();
  return (
    <div className="flex items-center justify-between gap-6 py-5 first:pt-0 last:pb-0">
      <div className="min-w-0">
        <p className="text-sm font-medium">{title}</p>
        <p id={descriptionId} className="mt-1 max-w-xl text-xs leading-relaxed text-muted-foreground">
          {description}
        </p>
      </div>
      <div className="shrink-0">{children(descriptionId)}</div>
    </div>
  );
}

export function ToggleSetting({
  title,
  description,
  checked,
  disabled,
  onChange,
}: {
  title: string;
  description: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  const id = useId();
  return (
    <div className="flex items-center justify-between gap-6 py-5 first:pt-0 last:pb-0">
      <div>
        <Label htmlFor={id} className="text-sm font-medium">
          {title}
        </Label>
        <p
          id={`${id}-hint`}
          className="mt-1 max-w-xl text-xs leading-relaxed text-muted-foreground"
        >
          {description}
        </p>
      </div>
      <Switch
        id={id}
        aria-describedby={`${id}-hint`}
        checked={checked}
        disabled={disabled}
        onCheckedChange={onChange}
      />
    </div>
  );
}

export function NumberSetting({
  value,
  min,
  max,
  label,
  unit,
  describedBy,
  onChange,
}: {
  value: number;
  min: number;
  max: number;
  label: string;
  unit: string;
  describedBy?: string;
  onChange: (value: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));
  const [synced, setSynced] = useState(value);
  if (value !== synced) {
    setSynced(value);
    setDraft(String(value));
  }
  const save = () => {
    const next = Number(draft);
    if (
      !draft.trim() ||
      !Number.isSafeInteger(next) ||
      next < min ||
      next > max
    ) {
      setDraft(String(value));
      toast.error(`Bitte eine ganze Zahl zwischen ${min} und ${max} eingeben.`);
      return;
    }
    if (next !== value) onChange(next);
  };
  return (
    <div className="flex items-center gap-2">
      <Input
        type="number"
        inputMode="numeric"
        className="w-20"
        aria-label={label}
        aria-describedby={describedBy}
        min={min}
        max={max}
        step={1}
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={save}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
      />
      <span className="w-14 text-xs text-muted-foreground">{unit}</span>
    </div>
  );
}
