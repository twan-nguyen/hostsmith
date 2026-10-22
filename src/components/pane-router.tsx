import { EnvPane } from "@/components/env-pane";
import { HostsPane } from "@/components/hosts-pane";
import { Panel } from "@/components/panel";
import { PortsPane } from "@/components/ports-pane";
import { ProfilePane } from "@/components/profile-pane";
import { SettingsView } from "@/components/settings-view";
import { formatSelection } from "@/lib/selection";
import { useStore } from "@/store";

/** One selection drives the whole working pane. */
export function PaneRouter() {
  const selection = useStore((s) => s.selection);
  const hosts = useStore((s) => s.hosts);
  // Keyed so each pane starts fresh with its own scroll and search.
  const key = formatSelection(selection);

  switch (selection.kind) {
    case "settings":
      return <SettingsView />;
    case "ports":
      return <PortsPane key={key} />;
    case "repo":
    case "envFile":
      return <EnvPane key={key} />;
    case "profile": {
      // Until the first snapshot there is no telling a profile from a missing one.
      if (!hosts) return <Panel />;
      const profile = hosts?.profiles.find((p) => p.id === selection.id);
      return profile ? <ProfilePane key={key} profile={profile} /> : <HostsPane key="hosts" />;
    }
    case "hostsTag":
      return <HostsPane key={key} block={selection.name} />;
    case "systemHosts":
      return <HostsPane key="hosts" />;
  }
}
