import { useState } from "react";

import { ConfirmDialog } from "@/components/confirm-dialog";
import { NameDialog } from "@/components/name-dialog";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { type EnvRepo, envProfileDelete, envProfileRename, envProfileSave } from "@/lib/hosven";
import { useStore } from "@/store";

export type EnvProfileDialogMode = "save" | "manage";

type SavedProfile = EnvRepo["profiles"][number];

const DATE = new Intl.DateTimeFormat("vi-VN", { dateStyle: "medium", timeStyle: "short" });

/** When a profile was captured, in the reader's time zone. */
function capturedAt(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : DATE.format(date);
}

/**
 * Saved snapshots of a repo's dotenv files: save the files as they are on
 * disk, or list them to apply, rename or delete one. Applying is the pane's
 * call, since it asks first when there are edits to lose.
 */
export function EnvProfileDialog({
  mode,
  repo,
  onApply,
  onClose,
}: {
  mode: EnvProfileDialogMode | null;
  repo: EnvRepo;
  onApply: (profile: SavedProfile) => void;
  onClose: () => void;
}) {
  return (
    <>
      <NameDialog
        open={mode === "save"}
        title="Lưu trạng thái thành profile"
        description="Hosven chụp lại nội dung các file .env trên đĩa của repo này để khôi phục sau. Thay đổi chưa áp dụng không nằm trong profile."
        label="Tên profile"
        submitLabel="Lưu profile"
        onSubmit={(name) => envProfileSave(repo.id, name)}
        onClose={onClose}
      />
      <ManageProfiles open={mode === "manage"} repo={repo} onApply={onApply} onClose={onClose} />
    </>
  );
}

function ManageProfiles({
  open,
  repo,
  onApply,
  onClose,
}: {
  open: boolean;
  repo: EnvRepo;
  onApply: (profile: SavedProfile) => void;
  onClose: () => void;
}) {
  const run = useStore((s) => s.run);
  const [renaming, setRenaming] = useState<SavedProfile | null>(null);
  const [deleting, setDeleting] = useState<SavedProfile | null>(null);

  return (
    <>
      <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Profile của repo</DialogTitle>
            <DialogDescription>Các bản chụp file .env đã lưu của {repo.name}.</DialogDescription>
          </DialogHeader>

          {repo.profiles.length === 0 ? (
            <div className="flex flex-col items-center gap-1 rounded-lg border border-border px-6 py-10 text-center">
              <div className="font-medium">Chưa có profile</div>
              <div className="text-muted-foreground">Lưu trạng thái hiện tại của repo để tạo profile đầu tiên.</div>
            </div>
          ) : (
            <ul className="max-h-80 overflow-y-auto rounded-lg border border-border">
              {repo.profiles.map((profile) => (
                <li
                  key={profile.id}
                  className="flex items-center gap-3 border-b border-border/50 px-3 py-2 last:border-b-0"
                >
                  <div className="flex min-w-0 flex-1 flex-col">
                    <span className="truncate font-medium" title={profile.name}>
                      {profile.name}
                    </span>
                    <span className="truncate text-xs text-muted-foreground" title={profile.files.join(", ")}>
                      {profile.files.length} file · {capturedAt(profile.capturedAt)}
                    </span>
                  </div>
                  <Button size="sm" variant="outline" onClick={() => onApply(profile)}>
                    Áp dụng
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => setRenaming(profile)}>
                    Đổi tên
                  </Button>
                  <Button size="sm" variant="destructive" onClick={() => setDeleting(profile)}>
                    Xoá
                  </Button>
                </li>
              ))}
            </ul>
          )}

          <DialogFooter>
            <Button onClick={onClose}>Đóng</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <NameDialog
        open={renaming !== null}
        title="Đổi tên profile"
        label="Tên profile"
        initial={renaming?.name}
        submitLabel="Đổi tên"
        onSubmit={(name) => envProfileRename(repo.id, renaming!.id, name)}
        onClose={() => setRenaming(null)}
      />
      <ConfirmDialog
        open={deleting !== null}
        title="Xoá profile?"
        description={`Xoá bản chụp ${deleting?.name}. Các file trong repo được giữ nguyên.`}
        confirmLabel="Xoá profile"
        onConfirm={() => deleting && void run(envProfileDelete(repo.id, deleting.id))}
        onClose={() => setDeleting(null)}
      />
    </>
  );
}
