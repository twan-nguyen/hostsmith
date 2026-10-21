import { useEffect, useId, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  type Profile,
  type ProfileColor,
  profilesAdd,
  profilesRename,
  profilesSetColor,
  profilesSetExclusive,
} from "@/lib/hosven";
import { PICKER_COLORS, PROFILE_COLORS } from "@/lib/profile-color";
import { cn, errorMessage } from "@/lib/utils";

/** Creates a profile, or edits the name, colour and kind of one. */
export function ProfileDialog({
  open,
  profile,
  onClose,
  onCreated,
}: {
  open: boolean;
  /** `null` creates a new profile. */
  profile: Profile | null;
  onClose: () => void;
  onCreated?: (profile: Profile) => void;
}) {
  const [name, setName] = useState("");
  const [color, setColor] = useState<ProfileColor>("blue");
  const [exclusive, setExclusive] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const ids = { name: useId(), exclusive: useId() };

  useEffect(() => {
    if (!open) return;
    setName(profile?.name ?? "");
    // A neutral colour shows as the neutral choice the picker offers.
    setColor(profile ? (PICKER_COLORS.includes(profile.color) ? profile.color : "purple") : "blue");
    setExclusive(profile?.isExclusive ?? true);
    setError(null);
  }, [open, profile]);

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      if (profile) {
        if (name.trim() !== profile.name) await profilesRename(profile.id, name.trim());
        if (color !== profile.color) await profilesSetColor(profile.id, color);
        if (exclusive !== profile.isExclusive) await profilesSetExclusive(profile.id, exclusive);
      } else {
        onCreated?.(await profilesAdd(name.trim(), color, exclusive));
      }
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-sm">
        <form
          className="contents"
          onSubmit={(e) => {
            e.preventDefault();
            if (name.trim() && !busy) void submit();
          }}
        >
          <DialogHeader>
            <DialogTitle>{profile ? "Sửa profile" : "Profile mới"}</DialogTitle>
            <DialogDescription>
              Profile là một block <code className="font-mono text-xs">## [tag:Tên]</code> trong /etc/hosts.
            </DialogDescription>
          </DialogHeader>

          <div className="flex flex-col gap-2">
            <label htmlFor={ids.name} className="text-muted-foreground">
              Tên
            </label>
            <Input
              id={ids.name}
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
              spellCheck={false}
            />
          </div>

          <div className="flex flex-col gap-2">
            <span className="text-muted-foreground">Màu</span>
            <div className="flex flex-wrap gap-1.5">
              {PICKER_COLORS.map((c) => (
                <button
                  key={c}
                  type="button"
                  onClick={() => setColor(c)}
                  aria-pressed={color === c}
                  className={cn(
                    "flex h-7 items-center gap-1.5 rounded-md border px-2.5 transition-colors",
                    color === c ? "border-ring bg-accent" : "border-border hover:bg-muted",
                  )}
                >
                  <span className={cn("size-2 rounded-full", PROFILE_COLORS[c].dot)} />
                  {PROFILE_COLORS[c].label}
                </button>
              ))}
            </div>
          </div>

          <div className="flex items-start justify-between gap-4">
            <label htmlFor={ids.exclusive} className="flex flex-col gap-1">
              <span className="font-medium">Môi trường</span>
              <span className="text-muted-foreground">
                Bật một môi trường sẽ tắt các môi trường khác. Block thì bật tắt độc lập.
              </span>
            </label>
            <Switch
              id={ids.exclusive}
              checked={exclusive}
              onCheckedChange={setExclusive}
              data-testid="profile-exclusive-switch"
            />
          </div>

          {error && <p className="animate-fade-in text-destructive">{error}</p>}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Huỷ
            </Button>
            <Button type="submit" disabled={!name.trim() || busy}>
              {profile ? "Lưu" : "Tạo profile"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
