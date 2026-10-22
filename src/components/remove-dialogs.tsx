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
      title="Xoá profile?"
      description={`Xoá metadata của profile ${profile?.name}. Các entry trong /etc/hosts được giữ nguyên.`}
      confirmLabel="Xoá profile"
      onConfirm={() => profile && void remove(profile)}
      onClose={onClose}
    />
  );
}
