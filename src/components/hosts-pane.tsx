import { Panel, PanelHeader } from "@/components/panel";

export function HostsPane({ block }: { block?: string }) {
  return (
    <Panel>
      <PanelHeader>
        <h1 className="font-medium">{block ?? "Hosts hệ thống"}</h1>
      </PanelHeader>
    </Panel>
  );
}
