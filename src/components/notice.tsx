import { CircleAlert, X } from "lucide-react";
import { useEffect } from "react";

import { useStore } from "@/store";

/** How long an error from a call stays up when nobody closes it. */
const NOTICE_MS = 6000;

/** Errors from calls that have no pane banner to report them. */
export function Notice() {
  const notice = useStore((s) => s.notice);
  const dismiss = useStore((s) => s.dismissNotice);

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(dismiss, NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice, dismiss]);

  if (!notice) return null;
  return (
    <div
      role="alert"
      className="fixed bottom-4 left-1/2 z-50 flex max-w-md -translate-x-1/2 animate-fade-in items-start gap-2.5 rounded-lg border border-destructive/30 bg-popover px-4 py-3 shadow-lg"
    >
      <CircleAlert className="mt-0.5 size-4 shrink-0 text-destructive" />
      <p className="flex-1">{notice.message}</p>
      <button onClick={dismiss} aria-label="Đóng" className="text-muted-foreground hover:text-foreground">
        <X className="size-4" />
      </button>
    </div>
  );
}
