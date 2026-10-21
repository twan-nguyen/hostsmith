import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useId, useState } from "react";

import { ConfirmDialog } from "@/components/confirm-dialog";
import { Panel } from "@/components/panel";
import { SettingsHeading } from "@/components/settings-heading";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import {
  type AppearanceMode,
  type Profile,
  type Settings,
  profilesRemove,
  settingsPreviewTransparency,
  settingsUpdate,
} from "@/lib/hosven";
import { PROFILE_COLORS } from "@/lib/profile-color";
import { cn } from "@/lib/utils";
import { profileStats, useStore } from "@/store";

export function SettingsView() {
  const settingsPage = useStore((s) => s.settingsPage);
  return (
    <Panel>
      <div className="flex min-h-0 flex-1 justify-center overflow-y-auto px-8 py-10">
        <div key={settingsPage} className="flex w-full max-w-xl animate-view-in flex-col gap-6">
          {settingsPage === "general" ? <GeneralSettings /> : <ProfilesSettings />}
        </div>
      </div>
    </Panel>
  );
}

const MODES: { mode: AppearanceMode; label: string }[] = [
  { mode: "system", label: "Theo hệ thống" },
  { mode: "light", label: "Sáng" },
  { mode: "dark", label: "Tối" },
];

function GeneralSettings() {
  const { settings, run } = useStore();
  const [version, setVersion] = useState("");
  const [transparency, setTransparency] = useState(settings?.transparency ?? 0);
  const ids = { transparency: useId(), menuBar: useId() };

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);
  useEffect(() => setTransparency(settings?.transparency ?? 0), [settings?.transparency]);

  if (!settings) return null;
  const update = (patch: Partial<Settings>) => void run(settingsUpdate(patch));
  // The canvas tint follows the slider while it moves; the file is written on release.
  const preview = (percent: number) => {
    setTransparency(percent);
    document.documentElement.style.setProperty("--canvas-opacity", `${100 - percent}%`);
    document.documentElement.classList.toggle("vibrancy", percent > 0);
    if (percent > 0 !== settings.transparency > 0) void run(settingsPreviewTransparency(percent));
  };

  return (
    <>
      <SettingsHeading title="Chung" description="Giao diện, trình soạn thảo và menu bar." />

      <Group title="Giao diện">
        <Row label="Chế độ">
          <div className="flex rounded-lg bg-muted p-0.5">
            {MODES.map(({ mode, label }) => (
              <button
                key={mode}
                onClick={() => update({ appearanceMode: mode })}
                aria-pressed={settings.appearanceMode === mode}
                className={cn(
                  "h-7 rounded-md px-3 transition-colors",
                  settings.appearanceMode === mode
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {label}
              </button>
            ))}
          </div>
        </Row>
        <div className="flex flex-col gap-3">
          <div className="flex items-start justify-between gap-6">
            <div className="flex flex-col gap-1">
              <span id={ids.transparency} className="font-medium">
                Độ trong suốt
              </span>
              <span className="text-muted-foreground">
                Để nền phía sau cửa sổ hiện mờ qua thanh bên. Các pane vẫn giữ nền đặc để dễ đọc.
              </span>
            </div>
            <span className="w-10 shrink-0 text-right font-medium tabular-nums">
              {transparency === 0 ? "Tắt" : `${transparency}%`}
            </span>
          </div>
          <Slider
            aria-labelledby={ids.transparency}
            min={0}
            max={100}
            step={5}
            value={[transparency]}
            onValueChange={([value]) => preview(value)}
            onValueCommit={([value]) => update({ transparency: value })}
          />
        </div>
      </Group>

      <Group title="Trình soạn thảo">
        <Row label="Cỡ chữ" hint="Cỡ chữ của trình soạn thảo hosts và .env, độc lập với cỡ chữ hệ thống.">
          <div className="flex items-center gap-2">
            <Button
              size="icon-sm"
              variant="outline"
              aria-label="Giảm cỡ chữ"
              disabled={settings.editorFontSize <= 10}
              onClick={() => update({ editorFontSize: settings.editorFontSize - 1 })}
            >
              −
            </Button>
            <span className="w-12 text-center tabular-nums">{settings.editorFontSize} pt</span>
            <Button
              size="icon-sm"
              variant="outline"
              aria-label="Tăng cỡ chữ"
              disabled={settings.editorFontSize >= 20}
              onClick={() => update({ editorFontSize: settings.editorFontSize + 1 })}
            >
              +
            </Button>
          </div>
        </Row>
      </Group>

      <Group title="Menu bar">
        <Row
          label="Hiện ở menu bar"
          labelFor={ids.menuBar}
          hint="Bật tắt block và kích hoạt profile từ menu bar mà không cần mở cửa sổ."
        >
          <Switch
            id={ids.menuBar}
            checked={settings.showMenuBarExtra}
            onCheckedChange={(showMenuBarExtra) => update({ showMenuBarExtra })}
          />
        </Row>
      </Group>

      <Group title="Hệ thống">
        <p className="text-muted-foreground">Hosven {version}</p>
      </Group>
    </>
  );
}

function ProfilesSettings() {
  const { hosts, run } = useStore();
  const [deleting, setDeleting] = useState<Profile | null>(null);
  const stats = profileStats(hosts);

  return (
    <>
      <SettingsHeading
        title="Profiles"
        description={`${stats.length} profile. Xoá một profile chỉ xoá màu, phím tắt và loại; entry trong /etc/hosts giữ nguyên.`}
      />
      {stats.length === 0 ? (
        <p className="text-muted-foreground">Chưa có profile</p>
      ) : (
        <div className="flex flex-col divide-y divide-border rounded-lg border border-border">
          {stats.map(({ profile, entryCount }) => {
            const n = profile.shortcutNumber;
            const shortcut = profile.isExclusive && n && n >= 1 && n <= 9 ? `⌘${n} · ` : "";
            return (
              <div key={profile.id} className="flex items-center gap-3 px-4 py-2.5">
                <span className={cn("size-2 shrink-0 rounded-full", PROFILE_COLORS[profile.color].dot)} />
                <div className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate font-medium">{profile.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {profile.isExclusive ? "Môi trường" : "Block"} · {shortcut}
                    {entryCount} entry
                  </span>
                </div>
                <span className={cn("rounded px-1.5 py-px text-[11px] font-medium", PROFILE_COLORS[profile.color].chip)}>
                  {PROFILE_COLORS[profile.color].label}
                </span>
                <Button size="sm" variant="ghost" onClick={() => setDeleting(profile)}>
                  Xoá
                </Button>
              </div>
            );
          })}
        </div>
      )}
      <ConfirmDialog
        open={deleting !== null}
        title="Xoá profile?"
        description={`Xoá metadata của profile ${deleting?.name}. Các entry trong /etc/hosts được giữ nguyên.`}
        confirmLabel="Xoá profile"
        onConfirm={() => deleting && void run(profilesRemove(deleting.id))}
        onClose={() => setDeleting(null)}
      />
    </>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-4 rounded-lg border border-border px-4 py-4">
      <h3 className="text-xs font-medium text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}

function Row({
  label,
  labelFor,
  hint,
  children,
}: {
  label: string;
  labelFor?: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-6">
      <label htmlFor={labelFor} className="flex flex-col gap-1">
        <span className="font-medium">{label}</span>
        {hint && <span className="text-muted-foreground">{hint}</span>}
      </label>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
