import { Cable, CircleOff, Folder, Layers, Network, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from "@/components/ui/command";
import { hostsSwitchEnvironment } from "@/lib/hosven";
import { type PaletteAction, type PaletteCommand, type PaletteIcon, paletteSections } from "@/lib/palette";
import { PROFILE_COLORS } from "@/lib/profile-color";
import { cn } from "@/lib/utils";
import { profileStats, useStore } from "@/store";

const ICONS: Record<PaletteIcon, React.ReactNode> = {
  network: <Network />,
  ports: <Cable />,
  folder: <Folder />,
  environment: <Layers />,
  off: <CircleOff />,
  search: <Search />,
};

export function CommandPalette() {
  const { paletteOpen, setPaletteOpen, hosts, env } = useStore();
  const [query, setQuery] = useState("");

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "k" && event.metaKey) {
        event.preventDefault();
        setPaletteOpen(!useStore.getState().paletteOpen);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [setPaletteOpen]);

  useEffect(() => {
    if (!paletteOpen) setQuery("");
  }, [paletteOpen]);

  const sections = useMemo(() => {
    const stats = profileStats(hosts).filter((s) => s.profile.isExclusive);
    return paletteSections(query, {
      environments: stats.map(({ profile, entryCount, active }) => ({
        id: profile.id,
        name: profile.name,
        color: profile.color,
        shortcutNumber: profile.shortcutNumber,
        entryCount,
        active: hosts?.active.kind === "active" && active,
      })),
      anyActive: hosts?.active.kind !== "none",
      repos: env?.repos.map((r) => ({ id: r.id, name: r.name, path: r.path })) ?? [],
    });
  }, [query, hosts, env]);

  const run = (action: PaletteAction) => {
    setPaletteOpen(false);
    const state = useStore.getState();
    switch (action.type) {
      case "select":
        state.select(action.selection);
        break;
      case "switch":
        state.select({ kind: "profile", id: action.profileId });
        void state.run(hostsSwitchEnvironment(action.profileId, "ask"));
        break;
      case "deactivate":
        void state.run(hostsSwitchEnvironment(null, "ask"));
        break;
      case "searchHosts":
        state.select({ kind: "systemHosts" });
        state.setHostsQuery(action.query);
        break;
      case "searchEnv": {
        const repo = state.env?.selectedRepo ?? state.env?.repos[0]?.id;
        if (repo) state.select({ kind: "repo", id: repo });
        state.setEnvQuery(action.query);
        break;
      }
    }
  };

  return (
    <CommandDialog open={paletteOpen} onOpenChange={setPaletteOpen}>
      <Command shouldFilter={false} loop>
        <CommandInput value={query} onValueChange={setQuery} placeholder="Tìm host, repo hoặc lệnh" />
        <CommandList>
          <CommandEmpty>Không có kết quả</CommandEmpty>
          {sections.map((section) => (
            <CommandGroup key={section.category} heading={section.title}>
              {section.items.map((command) => (
                <PaletteItem key={command.id} command={command} onRun={run} />
              ))}
            </CommandGroup>
          ))}
        </CommandList>
      </Command>
    </CommandDialog>
  );
}

function PaletteItem({ command, onRun }: { command: PaletteCommand; onRun: (action: PaletteAction) => void }) {
  const tint = command.tint ? PROFILE_COLORS[command.tint as keyof typeof PROFILE_COLORS]?.dot : undefined;
  return (
    <CommandItem value={command.id} onSelect={() => onRun(command.action)}>
      {tint ? (
        <span className="flex size-4 items-center justify-center">
          <span className={cn("size-2 rounded-full", tint)} />
        </span>
      ) : (
        ICONS[command.icon]
      )}
      <span className="truncate">{command.title}</span>
      {command.subtitle && <span className="ml-1 truncate text-xs text-muted-foreground">{command.subtitle}</span>}
      {command.shortcut && <CommandShortcut>{command.shortcut}</CommandShortcut>}
    </CommandItem>
  );
}
