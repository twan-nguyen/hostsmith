import { useEffect, useId, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { INVALID_KEY_MESSAGE, isValidEnvKey } from "@/lib/env-table";
import { type EnvEntry, envAddEntry, envUpdateEntry } from "@/lib/hosven";
import { errorMessage } from "@/lib/utils";

/**
 * Adds a variable to the selected file, or edits one. Nothing is written
 * until the pane applies; the backend checks the input again and its error
 * shows under the fields.
 */
export function EnvKeyDialog({
  open,
  entry,
  file,
  onClose,
}: {
  open: boolean;
  /** `null` adds a new variable. */
  entry: EnvEntry | null;
  /** The file's path in the repo, for the description. */
  file: string | null;
  onClose: () => void;
}) {
  const [key, setKey] = useState("");
  const [value, setValue] = useState("");
  const [comment, setComment] = useState("");
  const [touched, setTouched] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const ids = { key: useId(), value: useId(), comment: useId() };

  useEffect(() => {
    if (!open) return;
    setKey(entry?.key ?? "");
    setValue(entry?.value ?? "");
    setComment(entry?.comment ?? "");
    setTouched(false);
    setError(null);
  }, [open, entry]);

  const trimmedKey = key.trim();
  const keyProblem = trimmedKey && !isValidEnvKey(trimmedKey) ? INVALID_KEY_MESSAGE : null;
  // A value read from a quoted line may span lines; a one-line field would drop them.
  const multiline = value.includes("\n");

  async function submit() {
    setTouched(true);
    if (!trimmedKey || keyProblem) return;
    setBusy(true);
    setError(null);
    try {
      const input = { key: trimmedKey, value, comment: comment.trim() };
      await (entry ? envUpdateEntry(entry.id, input) : envAddEntry(input));
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-md">
        <form
          className="contents"
          onSubmit={(e) => {
            e.preventDefault();
            if (!busy) void submit();
          }}
        >
          <DialogHeader>
            <DialogTitle>{entry ? "Sửa biến" : "Biến mới"}</DialogTitle>
            <DialogDescription>
              {file ? `Không ghi gì vào ${file} cho đến khi bạn nhấn Áp dụng.` : "Không ghi gì cho đến khi bạn nhấn Áp dụng."}
            </DialogDescription>
          </DialogHeader>

          <div className="flex flex-col gap-2">
            <label htmlFor={ids.key} className="text-muted-foreground">
              Khoá
            </label>
            <Input
              id={ids.key}
              value={key}
              onChange={(e) => setKey(e.target.value)}
              onBlur={() => setTouched(true)}
              aria-invalid={touched && keyProblem !== null}
              placeholder="DATABASE_URL"
              autoFocus
              spellCheck={false}
              autoCapitalize="off"
              autoCorrect="off"
              className="font-mono"
            />
            {touched && keyProblem && <p className="animate-fade-in text-destructive">{keyProblem}</p>}
          </div>

          <div className="flex flex-col gap-2">
            <label htmlFor={ids.value} className="text-muted-foreground">
              Giá trị
            </label>
            {multiline ? (
              <Textarea
                id={ids.value}
                value={value}
                onChange={(e) => setValue(e.target.value)}
                spellCheck={false}
                className="max-h-48 font-mono"
              />
            ) : (
              <Input
                id={ids.value}
                value={value}
                onChange={(e) => setValue(e.target.value)}
                spellCheck={false}
                autoCapitalize="off"
                autoCorrect="off"
                className="font-mono"
              />
            )}
          </div>

          <div className="flex flex-col gap-2">
            <label htmlFor={ids.comment} className="text-muted-foreground">
              Ghi chú
            </label>
            <Input id={ids.comment} value={comment} onChange={(e) => setComment(e.target.value)} />
          </div>

          {error && <p className="animate-fade-in text-destructive">{error}</p>}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Huỷ
            </Button>
            <Button type="submit" disabled={!trimmedKey || keyProblem !== null || busy}>
              {entry ? "Lưu" : "Thêm biến"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
