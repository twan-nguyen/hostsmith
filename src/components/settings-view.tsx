import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useId, useRef, useState } from "react";

import { Panel } from "@/components/panel";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { type AppearanceMode, type Settings, settingsPreviewTransparency, settingsUpdate } from "@/lib/hosven";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

const MODES: { mode: AppearanceMode; label: string }[] = [
  { mode: "system", label: "Theo hệ thống" },
  { mode: "light", label: "Sáng" },
  { mode: "dark", label: "Tối" },
];

export function SettingsView() {
  const settings = useStore((s) => s.settings);
  const run = useStore((s) => s.run);
  const [version, setVersion] = useState("");
  const [transparency, setTransparency] = useState(settings?.transparency ?? 0);
  /** Whether the window blur is on, as last set while the slider moves. */
  const blurred = useRef<boolean | null>(null);
  const ids = { transparency: useId(), menuBar: useId() };

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);
  useEffect(() => {
    setTransparency(settings?.transparency ?? 0);
    blurred.current = null;
  }, [settings?.transparency]);

  if (!settings) return <Panel />;
  const update = (patch: Partial<Settings>) => void run(settingsUpdate(patch));
  // The canvas tint follows the slider while it moves; the file is written on release.
  const preview = (percent: number) => {
    setTransparency(percent);
    document.documentElement.style.setProperty("--canvas-opacity", `${100 - percent}%`);
    document.documentElement.classList.toggle("vibrancy", percent > 0);
    const blur = percent > 0;
    if (blur !== (blurred.current ?? settings.transparency > 0)) void run(settingsPreviewTransparency(percent));
    blurred.current = blur;
  };

  return (
    <Panel>
      <div className="flex min-h-0 flex-1 justify-center overflow-y-auto px-8 py-10">
        <div className="flex w-full max-w-xl flex-col gap-6">
          <div className="flex flex-col gap-1.5">
            <h2 className="text-xl font-semibold">Cài đặt</h2>
            <p className="text-muted-foreground">Giao diện, trình soạn thảo và menu bar.</p>
          </div>

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
        </div>
      </div>
    </Panel>
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
