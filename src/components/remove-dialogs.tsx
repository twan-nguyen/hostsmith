import { ConfirmDialog } from "@/components/confirm-dialog";
import { type EnvRepo, type Profile, envRemoveRepo, profilesRemove } from "@/lib/hosven";
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

/** Asks before dropping a repo from the list; its files stay where they are. */
export function RemoveRepoDialog({ repo, onClose }: { repo: EnvRepo | null; onClose: () => void }) {
  function remove(target: EnvRepo) {
    const { run, select, selection } = useStore.getState();
    // Leave the repo's pane first, so the typing it hands over on the way out
    // still lands in a repo that exists.
    const open =
      (selection.kind === "repo" && selection.id === target.id) ||
      (selection.kind === "envFile" && selection.repo === target.id);
    if (open) select({ kind: "systemHosts" });
    void run(envRemoveRepo(target.id));
  }

  const saved = repo?.profiles.length ?? 0;
  return (
    <ConfirmDialog
      open={repo !== null}
      title="Xoá repo khỏi danh sách?"
      description={
        saved > 0
          ? `Các file trong ${repo?.path} được giữ nguyên. ${saved} profile env đã lưu của repo sẽ bị xoá.`
          : `Các file trong ${repo?.path} được giữ nguyên.`
      }
      confirmLabel="Xoá khỏi danh sách"
      onConfirm={() => repo && remove(repo)}
      onClose={onClose}
    />
  );
}
