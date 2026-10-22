import { useEffect, useRef, useState } from "react";

import type { Toast } from "@/lib/hosven";

/** How long a toast stays in the banner slot. */
const TOAST_MS = 3000;

/**
 * The toast to show now. Snapshots repeat the last toast until a newer one
 * replaces it, so a toast shows once, when its id is new, and hides after
 * three seconds. A toast already present when the pane mounts was shown
 * elsewhere and is skipped.
 */
export function useToast(toast: Toast | null): [Toast | null, () => void] {
  const [shown, setShown] = useState<Toast | null>(null);
  const seen = useRef<number | null>(toast?.id ?? null);

  useEffect(() => {
    if (!toast || toast.id === seen.current) return;
    seen.current = toast.id;
    setShown(toast);
    const timer = window.setTimeout(() => setShown((s) => (s?.id === toast.id ? null : s)), TOAST_MS);
    return () => window.clearTimeout(timer);
    // Only a new id starts a toast; the same toast in a later snapshot must
    // not restart or cancel its timer.
  }, [toast?.id]);

  return [shown, () => setShown(null)];
}
