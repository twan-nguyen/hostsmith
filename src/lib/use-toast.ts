import { useEffect, useState } from "react";

import type { Toast } from "@/lib/hosven";

/** How long a toast stays in the banner slot, as in Hosven 1.x. */
export const TOAST_MS = 3000;

/**
 * The toast to show now. Snapshots repeat the last toast until a newer one
 * replaces it, so a toast shows once, when its id is new, and hides after
 * three seconds.
 */
export function useToast(toast: Toast | null): [Toast | null, () => void] {
  const [shown, setShown] = useState<Toast | null>(null);
  const [seen, setSeen] = useState<number | null>(toast?.id ?? null);

  useEffect(() => {
    if (!toast || toast.id === seen) return;
    setSeen(toast.id);
    setShown(toast);
    const timer = window.setTimeout(() => setShown((s) => (s?.id === toast.id ? null : s)), TOAST_MS);
    return () => window.clearTimeout(timer);
  }, [toast, seen]);

  return [shown, () => setShown(null)];
}
