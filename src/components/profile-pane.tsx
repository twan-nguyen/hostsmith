import { Panel, PanelHeader } from "@/components/panel";
import type { Profile } from "@/lib/hosven";

export function ProfilePane({ profile }: { profile: Profile }) {
  return (
    <Panel>
      <PanelHeader>
        <h1 className="font-medium">{profile.name}</h1>
      </PanelHeader>
    </Panel>
  );
}
