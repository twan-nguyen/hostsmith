import { Banner, type BannerAction, type BannerTone } from "@/components/banner";
import {
  type Toast,
  type ToastKind,
  hostsDismissVerification,
  hostsReload,
  toolsOpenChromeDns,
} from "@/lib/hosven";
import { verificationSummary } from "@/lib/hosts-view";
import { useStore } from "@/store";

const TOAST_TONES: Record<ToastKind, BannerTone> = { success: "success", error: "danger", info: "info" };

/** A toast from the hosts model, in the banner slot for three seconds. */
export function ToastBanner({ toast, onDismiss }: { toast: Toast; onDismiss: () => void }) {
  return <Banner tone={TOAST_TONES[toast.kind]} title={toast.message} onDismiss={onDismiss} />;
}

/** Another tool wrote /etc/hosts; nothing is written until it is read again. */
export function ExternalChangeBanner() {
  const run = useStore((s) => s.run);
  const applying = useStore((s) => s.hosts?.applying ?? false);
  return (
    <Banner
      tone="warning"
      title="/etc/hosts đã thay đổi bên ngoài Hosven"
      actions={[{ label: "Tải lại", onClick: () => void run(hostsReload()), disabled: applying }]}
    >
      Tải lại để xem nội dung mới nhất. Thay đổi chưa áp dụng sẽ bị bỏ.
    </Banner>
  );
}

/** The file could not be read, so Hosven refuses to write over it. */
export function LoadErrorBanner({ error }: { error: string }) {
  const run = useStore((s) => s.run);
  const applying = useStore((s) => s.hosts?.applying ?? false);
  return (
    <Banner
      tone="danger"
      title="Không đọc được /etc/hosts"
      actions={[{ label: "Tải lại", onClick: () => void run(hostsReload()), disabled: applying }]}
    >
      {error}
    </Banner>
  );
}

/** What the resolver answered for the hostnames the last write changed. */
export function VerificationBanner() {
  const results = useStore((s) => s.hosts?.verification?.results ?? null);
  const chrome = useStore((s) => s.tools?.chrome ?? false);
  const run = useStore((s) => s.run);
  if (!results) return null;

  const summary = verificationSummary(results);
  const actions: BannerAction[] = [];
  if (chrome) actions.push({ label: "Xoá cache DNS của Chrome", onClick: () => void run(toolsOpenChromeDns()) });
  actions.push({ label: "Đóng", onClick: () => void run(hostsDismissVerification()) });

  return (
    <Banner tone={summary.tone} title={summary.title} actions={actions}>
      {summary.lines.map((line, i) => (
        <p key={i} className={summary.tone === "warning" ? "font-mono text-xs" : undefined}>
          {line}
        </p>
      ))}
    </Banner>
  );
}
