import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { errorMessage } from "@/lib/utils";

/**
 * A dialog around a form: "Cancel" and a submit button. Submitting waits for
 * `onSubmit`, closes when it succeeds and shows its error otherwise.
 * `onSubmit` returns `false` to stay open without an error of its own, when
 * it flagged a field instead.
 */
export function FormDialog({
  open,
  title,
  description,
  submitLabel,
  canSubmit = true,
  onSubmit,
  onClose,
  className = "sm:max-w-sm",
  submitTestId,
  children,
}: {
  open: boolean;
  title: string;
  description?: React.ReactNode;
  submitLabel: string;
  canSubmit?: boolean;
  onSubmit: () => Promise<unknown>;
  onClose: () => void;
  className?: string;
  submitTestId?: string;
  children: React.ReactNode;
}) {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (open) setError(null);
  }, [open]);

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      if ((await onSubmit()) !== false) onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className={className}>
        <form
          className="contents"
          onSubmit={(e) => {
            e.preventDefault();
            if (canSubmit && !busy) void submit();
          }}
        >
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            {description && <DialogDescription>{description}</DialogDescription>}
          </DialogHeader>

          {children}

          {error && <p className="animate-fade-in break-words text-destructive">{error}</p>}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={!canSubmit || busy} data-testid={submitTestId}>
              {submitLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

/** A labelled field, with a hint under it or, once something is wrong, the error. */
export function Field({
  id,
  label,
  hint,
  error,
  children,
}: {
  id?: string;
  label: string;
  hint?: string;
  error?: string | null;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-2">
      <label htmlFor={id} className="text-muted-foreground">
        {label}
      </label>
      {children}
      {error ? (
        <p className="animate-fade-in text-destructive">{error}</p>
      ) : (
        hint && <p className="text-xs text-muted-foreground">{hint}</p>
      )}
    </div>
  );
}
