import { useRef } from "react";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { hostsCancelPendingSwitch, hostsConfirmPendingSwitch } from "@/lib/hosven";
import { useStore } from "@/store";

/**
 * A switch started from the window while other hosts edits are pending: one
 * write would carry both, so the user decides.
 */
export function PendingSwitchDialog() {
  const hosts = useStore((s) => s.hosts);
  const run = useStore((s) => s.run);
  const pending = hosts?.pendingSwitch ?? null;
  const target = pending?.target ? hosts?.profiles.find((p) => p.id === pending.target) : null;
  // Closing after "Áp dụng cùng" must not also cancel the switch it started.
  const confirming = useRef(false);

  const confirm = () => {
    confirming.current = true;
    void run(hostsConfirmPendingSwitch()).finally(() => (confirming.current = false));
  };
  const close = () => {
    if (!confirming.current) void run(hostsCancelPendingSwitch());
  };

  return (
    <AlertDialog open={pending !== null} onOpenChange={(o) => !o && close()}>
      <AlertDialogContent className="sm:max-w-sm">
        <AlertDialogHeader>
          <AlertDialogTitle>Có {pending?.pendingCount} thay đổi chưa áp dụng</AlertDialogTitle>
          <AlertDialogDescription>
            {target ? `Đổi sang ${target.name}` : "Tắt môi trường"} sẽ ghi cả {pending?.pendingCount} thay đổi này vào
            /etc/hosts.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel autoFocus>Huỷ</AlertDialogCancel>
          <AlertDialogAction onClick={confirm}>Áp dụng cùng</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
