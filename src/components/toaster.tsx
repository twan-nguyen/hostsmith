import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  type Toast,
  hostsDismissVerification,
  hostsReload,
  hostsSwitchEnvironment,
  toolsOpenChromeDns,
} from "@/lib/hostsmith";
import { verificationSummary } from "@/lib/hosts-view";
import { type NoticeTone, TOAST_TONE, cleanVerification, hostsAlerts, noticeMs } from "@/lib/notices";
import { useStore } from "@/store";

type Action = { label: string; onClick: () => void; disabled?: boolean };

type Note = {
  key: string;
  tone: NoticeTone;
  title: string;
  detail?: React.ReactNode;
  actions?: Action[];
  onDismiss?: () => void;
};

const ICONS: Record<NoticeTone, React.ReactNode> = {
  success: <CircleCheck className="text-success" />,
  info: <Info className="text-muted-foreground" />,
  warning: <TriangleAlert className="text-warning" />,
  danger: <CircleAlert className="text-destructive" />,
};

/** Passing notes last this many at once; older ones leave first. */
const MAX_PASSING = 3;

/**
 * Every message the app has for the user, floating over the bottom right of
 * the window instead of pushing the panes around. States that block or
 * question the next write stay until they are resolved; results of an action
 * pass after a few seconds.
 */
export function Toaster() {
  const alerts = useAlerts();
  const passing = usePassing();
  if (alerts.length === 0 && passing.length === 0) return null;
  return (
    <div className="pointer-events-none fixed right-4 bottom-4 z-50 flex w-[22rem] flex-col gap-2" aria-live="polite">
      {[...alerts, ...passing].map((note) => (
        <NoteCard key={note.key} note={note} />
      ))}
    </div>
  );
}

function NoteCard({ note }: { note: Note }) {
  return (
    <div
      role={note.tone === "danger" ? "alert" : "status"}
      className="pointer-events-auto flex animate-toast-in items-start gap-2.5 rounded-lg border border-border bg-popover px-3 py-2.5 text-popover-foreground shadow-[0_8px_28px_oklch(0_0_0/22%)] [&>svg]:mt-px [&>svg]:size-4 [&>svg]:shrink-0"
    >
      {ICONS[note.tone]}
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <p className="font-medium text-pretty">{note.title}</p>
        {note.detail && <div className="text-xs text-muted-foreground">{note.detail}</div>}
        {note.actions && note.actions.length > 0 && (
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {note.actions.map((action, i) => (
              <Button
                key={action.label}
                size="xs"
                variant={i === 0 ? "secondary" : "ghost"}
                onClick={action.onClick}
                disabled={action.disabled}
              >
                {action.label}
              </Button>
            ))}
          </div>
        )}
      </div>
      {note.onDismiss && (
        <button
          onClick={note.onDismiss}
          aria-label="Đóng"
          className="-mt-0.5 -mr-1 rounded p-0.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          <X className="size-3.5" />
        </button>
      )}
    </div>
  );
}

/** The hosts file's standing problems, each with the way out of it. */
function useAlerts(): Note[] {
  const hosts = useStore((s) => s.hosts);
  const chrome = useStore((s) => s.tools?.chrome ?? false);
  const run = useStore((s) => s.run);
  const applying = hosts?.applying ?? false;
  const reload = { label: "Tải lại", onClick: () => void run(hostsReload()), disabled: applying };

  return hostsAlerts(hosts).map((alert): Note => {
    switch (alert) {
      case "loadError":
        return { key: alert, tone: "danger", title: "Không đọc được /etc/hosts", detail: hosts?.loadError, actions: [reload] };
      case "externalChange":
        return {
          key: alert,
          tone: "warning",
          title: "/etc/hosts vừa đổi bên ngoài Hosven",
          detail: "Tải lại trước khi ghi. Thay đổi chưa áp dụng sẽ bị bỏ.",
          actions: [reload],
        };
      case "conflict": {
        const ids = hosts?.active.kind === "conflict" ? hosts.active.ids : [];
        const environments = ids.flatMap((id) => hosts?.profiles.filter((p) => p.id === id) ?? []);
        return {
          key: alert,
          tone: "warning",
          title: `${environments.length} môi trường đang cùng bật`,
          detail: "Hostname trùng nhau trỏ theo dòng đứng trước. Giữ một môi trường để tắt phần còn lại.",
          actions: environments.map((p) => ({
            label: `Chỉ giữ ${p.name}`,
            onClick: () => void run(hostsSwitchEnvironment(p.id, "ask")),
            disabled: applying,
          })),
        };
      }
      case "mismatch": {
        const summary = verificationSummary(hosts?.verification?.results ?? []);
        const actions: Action[] = [];
        if (chrome) actions.push({ label: "Xoá cache DNS của Chrome", onClick: () => void run(toolsOpenChromeDns()) });
        return {
          key: alert,
          tone: "warning",
          title: summary.title,
          detail: summary.lines.map((line, i) => (
            <p key={i} className="font-mono break-all">
              {line}
            </p>
          )),
          actions,
          onDismiss: () => void run(hostsDismissVerification()),
        };
      }
    }
  });
}

/**
 * Results of actions: the toasts the snapshots carry, failed calls, and a
 * resolver check that came back clean. Anything already there when the
 * window opened was shown before and is skipped.
 */
function usePassing(): Note[] {
  const [notes, setNotes] = useState<(Note & { ms: number })[]>([]);
  const timers = useRef(new Map<string, number>());

  const push = (note: Note) => {
    const ms = noticeMs(note.tone);
    setNotes((all) => [...all.filter((n) => n.key !== note.key), { ...note, ms }].slice(-MAX_PASSING));
  };
  const drop = (key: string) => setNotes((all) => all.filter((n) => n.key !== key));

  useIncoming("hosts", useStore((s) => s.hosts?.toast ?? null), push);
  useIncoming("ports", useStore((s) => s.ports?.toast ?? null), push);
  useIncoming("call", useStore((s) => s.notice), push);

  // A clean check is news once, when it arrives.
  const verification = useStore((s) => s.hosts?.verification ?? null);
  const seenVerification = useRef(verification);
  useEffect(() => {
    if (verification === seenVerification.current) return;
    seenVerification.current = verification;
    const hosts = useStore.getState().hosts;
    if (!verification || !cleanVerification(hosts)) return;
    const summary = verificationSummary(verification.results);
    push({ key: "verified", tone: "success", title: summary.title, detail: summary.lines[0] });
  }, [verification]);

  // Each note leaves on its own clock; a newer note with the same key restarts it.
  useEffect(() => {
    const live = new Set(notes.map((n) => n.key));
    for (const [key, timer] of timers.current) {
      if (!live.has(key)) {
        window.clearTimeout(timer);
        timers.current.delete(key);
      }
    }
    for (const note of notes) {
      if (timers.current.has(note.key)) continue;
      timers.current.set(
        note.key,
        window.setTimeout(() => {
          timers.current.delete(note.key);
          drop(note.key);
        }, note.ms),
      );
    }
  }, [notes]);
  useEffect(() => () => timers.current.forEach((t) => window.clearTimeout(t)), []);

  return notes.map((note) => ({ ...note, onDismiss: () => drop(note.key) }));
}

/** Pushes `toast` once, when its id is new. */
function useIncoming(source: string, toast: Toast | null, push: (note: Note) => void) {
  const seen = useRef(toast?.id ?? null);
  useEffect(() => {
    if (!toast || toast.id === seen.current) return;
    seen.current = toast.id;
    push({ key: `${source}:${toast.id}`, tone: TOAST_TONE[toast.kind], title: toast.message });
    // A repeated snapshot carries the same toast; only a new id is news.
  }, [toast?.id]);
}
