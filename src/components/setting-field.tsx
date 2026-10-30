import { useId, useState, type ReactNode } from "react";
import { Minus, Plus } from "lucide-react";
import { toast } from "sonner";
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldLabel,
} from "@/components/ui/field";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
  InputGroupText,
} from "@/components/ui/input-group";
import { Switch } from "@/components/ui/switch";

export function SettingField({
  label,
  description,
  children,
}: {
  label: string;
  description: string;
  children: (ids: { control: string; description: string }) => ReactNode;
}) {
  const control = useId();
  const descriptionId = useId();
  return (
    <Field orientation="horizontal">
      <FieldContent>
        <FieldLabel htmlFor={control}>{label}</FieldLabel>
        <FieldDescription id={descriptionId}>{description}</FieldDescription>
      </FieldContent>
      {children({ control, description: descriptionId })}
    </Field>
  );
}

export function SwitchField({
  label,
  description,
  checked,
  disabled,
  onChange,
}: {
  label: string;
  description: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <SettingField label={label} description={description}>
      {(ids) => (
        <Switch
          id={ids.control}
          aria-describedby={ids.description}
          checked={checked}
          disabled={disabled}
          onCheckedChange={onChange}
        />
      )}
    </SettingField>
  );
}

export function NumberStepper({
  id,
  value,
  min,
  max,
  unit,
  describedBy,
  disabled = false,
  onChange,
}: {
  id: string;
  value: number;
  min: number;
  max: number;
  unit: string;
  describedBy: string;
  disabled?: boolean;
  onChange: (value: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));
  const [synced, setSynced] = useState(value);
  if (value !== synced) {
    setSynced(value);
    setDraft(String(value));
  }
  const commit = (next: number) => {
    if (!Number.isSafeInteger(next) || next < min || next > max) {
      setDraft(String(value));
      toast.error(`Bitte eine ganze Zahl zwischen ${min} und ${max} eingeben.`);
      return;
    }
    setDraft(String(next));
    if (next !== value) onChange(next);
  };
  return (
    <InputGroup className="w-40 shrink-0">
      <InputGroupAddon>
        <InputGroupButton
          size="icon-xs"
          aria-label="Verringern"
          disabled={disabled || value <= min}
          onClick={() => commit(value - 1)}
        >
          <Minus />
        </InputGroupButton>
      </InputGroupAddon>
      <InputGroupInput
        id={id}
        inputMode="numeric"
        aria-describedby={describedBy}
        disabled={disabled}
        className="text-center tabular-nums"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => commit(draft.trim() ? Number(draft) : Number.NaN)}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
      />
      <InputGroupAddon align="inline-end">
        <InputGroupText>{unit}</InputGroupText>
        <InputGroupButton
          size="icon-xs"
          aria-label="Erhöhen"
          disabled={disabled || value >= max}
          onClick={() => commit(value + 1)}
        >
          <Plus />
        </InputGroupButton>
      </InputGroupAddon>
    </InputGroup>
  );
}
