import { useEffect, useId, useState } from "react";

import { Field, FormDialog } from "@/components/form-dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { INVALID_KEY_MESSAGE, isValidEnvKey } from "@/lib/env-table";
import { type EnvEntry, envAddEntry, envUpdateEntry } from "@/lib/hosven";

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
  const ids = { key: useId(), value: useId(), comment: useId() };

  useEffect(() => {
    if (!open) return;
    setKey(entry?.key ?? "");
    setValue(entry?.value ?? "");
    setComment(entry?.comment ?? "");
    setTouched(false);
  }, [open, entry]);

  const trimmedKey = key.trim();
  const keyProblem = trimmedKey && !isValidEnvKey(trimmedKey) ? INVALID_KEY_MESSAGE : null;
  // A value read from a quoted line may span lines; a one-line field would drop them.
  const multiline = value.includes("\n");

  function submit() {
    const input = { key: trimmedKey, value, comment: comment.trim() };
    return entry ? envUpdateEntry(entry.id, input) : envAddEntry(input);
  }

  return (
    <FormDialog
      open={open}
      title={entry ? "Sửa biến" : "Biến mới"}
      description={
        file ? `Không ghi gì vào ${file} cho đến khi bạn nhấn Áp dụng.` : "Không ghi gì cho đến khi bạn nhấn Áp dụng."
      }
      submitLabel={entry ? "Lưu" : "Thêm biến"}
      canSubmit={trimmedKey !== "" && keyProblem === null}
      onSubmit={submit}
      onClose={onClose}
      className="sm:max-w-md"
    >
      <Field id={ids.key} label="Khoá" error={touched ? keyProblem : null}>
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
      </Field>

      <Field id={ids.value} label="Giá trị">
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
      </Field>

      <Field id={ids.comment} label="Ghi chú">
        <Input id={ids.comment} value={comment} onChange={(e) => setComment(e.target.value)} />
      </Field>
    </FormDialog>
  );
}
