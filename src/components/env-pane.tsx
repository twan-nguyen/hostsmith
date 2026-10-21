import { Panel, PanelHeader } from "@/components/panel";
import { useStore } from "@/store";

export function EnvPane() {
  const snapshot = useStore((state) => state.env);
  const repo = snapshot?.repos.find((r) => r.id === snapshot.selectedRepo);
  return (
    <Panel>
      <PanelHeader>
        <h1 className="font-medium">{repo?.name ?? "Repo"}</h1>
      </PanelHeader>
    </Panel>
  );
}
