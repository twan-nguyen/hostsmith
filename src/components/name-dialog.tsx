import { useEffect, useId, useState } from "react";

import { Field, FormDialog } from "@/components/form-dialog";
import { Input } from "@/components/ui/input";

/**
 * Asks for one name: renaming a repo, saving an env profile. The backend
 * checks the name and its error is shown under the field.
 */
export function NameDialog({
  open,
  title,
  description,
  label,
  initial = "",
  submitLabel,
  onSubmit,
  onClose,
}: {
  open: boolean;
  title: string;
  description?: string;
  label: string;
  initial?: string;
  submitLabel: string;
  onSubmit: (name: string) => Promise<unknown>;
  onClose: () => void;
}) {
  const [name, setName] = useState(initial);
  const id = useId();

  useEffect(() => {
    if (open) setName(initial);
  }, [open, initial]);

  return (
    <FormDialog
      open={open}
      title={title}
      description={description}
      submitLabel={submitLabel}
      canSubmit={name.trim() !== ""}
      onSubmit={() => onSubmit(name.trim())}
      onClose={onClose}
    >
      <Field id={id} label={label}>
        <Input id={id} value={name} onChange={(e) => setName(e.target.value)} autoFocus spellCheck={false} />
      </Field>
    </FormDialog>
  );
}
