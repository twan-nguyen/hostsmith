import { ConfirmDialog } from "@/components/confirm-dialog";
import { type Profile, profilesRemove } from "@/lib/hostsmith";
import { useStore } from "@/store";

/**
 * Asks before removing a profile's metadata. The pane that showed it falls
 * back to the hosts file, so the sidebar and the remembered selection never
 * point at a profile that is gone.
 */
export function RemoveProfileDialog({ profile, onClose }: { profile: Profile | null; onClose: () => void }) {
  async function remove(target: Profile) {
    const { run, select } = useStore.getState();
    if (!(await run(profilesRemove(target.id).then(() => true)))) return;
    const { selection } = useStore.getState();
    if (selection.kind === "profile" && selection.id === target.id) select({ kind: "systemHosts" });
  }

  return (
    <ConfirmDialog
      open={profile !== null}
      title="Delete profile?"
      description={`Deletes the metadata of profile ${profile?.name}. Its entries in /etc/hosts stay as they are.`}
      confirmLabel="Delete profile"
      onConfirm={() => profile && void remove(profile)}
      onClose={onClose}
    />
  );
}
