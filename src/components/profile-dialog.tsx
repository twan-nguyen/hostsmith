import { useEffect, useId, useState } from "react";

import { Field, FormDialog } from "@/components/form-dialog";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { type Profile, type ProfileColor, profilesAdd, profilesRename, profilesSetColor } from "@/lib/hosven";
import { PICKER_COLORS, PROFILE_COLORS } from "@/lib/profile-color";
import { cn } from "@/lib/utils";

/**
 * Creates a profile, or edits the name and colour of one. The kind is picked
 * when the profile is created; turning one into the other is a menu action.
 */
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
  const ids = { name: useId(), exclusive: useId() };

  useEffect(() => {
    if (!open) return;
    setName(profile?.name ?? "");
    // A neutral colour shows as the neutral choice the picker offers.
    setColor(profile ? (PICKER_COLORS.includes(profile.color) ? profile.color : "purple") : "blue");
    setExclusive(true);
  }, [open, profile]);

  async function submit() {
    if (!profile) {
      onCreated?.(await profilesAdd(name.trim(), color, exclusive));
      return;
    }
    if (name.trim() !== profile.name) await profilesRename(profile.id, name.trim());
    if (color !== profile.color) await profilesSetColor(profile.id, color);
  }

  return (
    <FormDialog
      open={open}
      title={profile ? "Sửa profile" : "Profile mới"}
      description={
        <>
          Profile là một block <code className="font-mono text-xs">## [tag:Tên]</code> trong /etc/hosts.
        </>
      }
      submitLabel={profile ? "Lưu" : "Tạo profile"}
      canSubmit={name.trim() !== ""}
      onSubmit={submit}
      onClose={onClose}
    >
      <Field id={ids.name} label="Tên">
        <Input id={ids.name} value={name} onChange={(e) => setName(e.target.value)} autoFocus spellCheck={false} />
      </Field>

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

      {!profile && (
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
      )}
    </FormDialog>
  );
}
