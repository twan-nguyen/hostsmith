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
import { Textarea } from "@/components/ui/textarea";
import { hostsImport } from "@/lib/hosven";
import { errorMessage } from "@/lib/utils";

/** Adds the entries of pasted hosts text whose hostnames are new. The model reports the count. */
export function ImportDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const id = useId();

  useEffect(() => {
    if (!open) return;
    setText("");
    setError(null);
  }, [open]);

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      await hostsImport(text);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  const blank = text.trim() === "";

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-xl">
        <form
          className="contents"
          onSubmit={(e) => {
            e.preventDefault();
            if (!blank && !busy) void submit();
          }}
        >
          <DialogHeader>
            <DialogTitle>Nhập entry</DialogTitle>
            <DialogDescription id={`${id}-hint`}>
              Dán nội dung dạng hosts. Chỉ những hostname chưa có mới được thêm.
            </DialogDescription>
          </DialogHeader>
          <Textarea
            id={id}
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              // ⌘↩ imports; a plain return is a new line.
              if (e.key === "Enter" && e.metaKey) e.currentTarget.form?.requestSubmit();
            }}
            aria-describedby={`${id}-hint`}
            placeholder={"127.0.0.1\tapi.local\n10.0.0.5\tdb.local # staging"}
            spellCheck={false}
            autoCapitalize="off"
            autoCorrect="off"
            autoFocus
            className="field-sizing-fixed h-60 resize-none font-mono text-xs md:text-xs"
            data-testid="import-text"
          />
          {error && <p className="animate-fade-in break-words text-destructive">{error}</p>}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Huỷ
            </Button>
            <Button type="submit" disabled={blank || busy} data-testid="import-submit">
              Nhập entry
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
