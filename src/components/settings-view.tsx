import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useId, useRef, useState } from "react";

import { Panel } from "@/components/panel";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { type AppearanceMode, type Settings, settingsPreviewTransparency, settingsUpdate } from "@/lib/hostsmith";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

const MODES: { mode: AppearanceMode; label: string }[] = [
  { mode: "system", label: "System" },
  { mode: "light", label: "Light" },
  { mode: "dark", label: "Dark" },
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
            <h2 className="text-xl font-semibold">Settings</h2>
            <p className="text-muted-foreground">Appearance, editor and menu bar.</p>
          </div>

          <Group title="Appearance">
            <Row label="Mode">
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
                    Transparency
                  </span>
                  <span className="text-muted-foreground">
                    Lets the desktop behind the window show blurred through the sidebar. Panes keep a solid background so they stay easy to read.
                  </span>
                </div>
                <span className="w-10 shrink-0 text-right font-medium tabular-nums">
                  {transparency === 0 ? "Off" : `${transparency}%`}
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

          <Group title="Editor">
            <Row label="Font size" hint="The hosts editor's font size, independent of the system's.">
              <div className="flex items-center gap-2">
                <Button
                  size="icon-sm"
                  variant="outline"
                  aria-label="Decrease font size"
                  disabled={settings.editorFontSize <= 10}
                  onClick={() => update({ editorFontSize: settings.editorFontSize - 1 })}
                >
                  −
                </Button>
                <span className="w-12 text-center tabular-nums">{settings.editorFontSize} pt</span>
                <Button
                  size="icon-sm"
                  variant="outline"
                  aria-label="Increase font size"
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
              label="Show in menu bar"
              labelFor={ids.menuBar}
              hint="Switch environments and turn blocks on and off from the menu bar without opening the window."
            >
              <Switch
                id={ids.menuBar}
                checked={settings.showMenuBarExtra}
                onCheckedChange={(showMenuBarExtra) => update({ showMenuBarExtra })}
              />
            </Row>
          </Group>

          <Group title="About">
            <p className="text-muted-foreground">HostSmith {version}</p>
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
