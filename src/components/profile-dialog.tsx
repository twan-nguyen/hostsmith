import { useEffect, useId, useState } from "react";

import { Field, FormDialog } from "@/components/form-dialog";
import { Input } from "@/components/ui/input";
import { type Profile, type ProfileColor, profilesAdd, profilesRename, profilesSetColor } from "@/lib/hostsmith";
import { PICKER_COLORS, PROFILE_COLORS } from "@/lib/profile-color";
import { cn } from "@/lib/utils";

/**
 * Creates an environment or a block, or edits one. The kind comes from where
 * it was created; turning one into the other is a menu action. Only
 * environments have a colour, since only they show one.
 */
export function ProfileDialog({
  open,
  profile,
  exclusive: creatingExclusive = true,
  onClose,
  onCreated,
}: {
  open: boolean;
  /** `null` creates a new profile. */
  profile: Profile | null;
  /** The kind a new profile gets. */
  exclusive?: boolean;
  onClose: () => void;
  onCreated?: (profile: Profile) => void;
}) {
  const [name, setName] = useState("");
  const [color, setColor] = useState<ProfileColor>("blue");
  const ids = { name: useId() };
  const exclusive = profile ? profile.isExclusive : creatingExclusive;

  useEffect(() => {
    if (!open) return;
    setName(profile?.name ?? "");
    // A neutral colour shows as the neutral choice the picker offers.
    setColor(profile ? (PICKER_COLORS.includes(profile.color) ? profile.color : "purple") : exclusive ? "blue" : "purple");
  }, [open, profile, exclusive]);

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
      title={profile ? `Edit ${profile.name}` : exclusive ? "New environment" : "New block"}
      description={
        exclusive ? (
          "Turning an environment on turns the others off."
        ) : (
          <>
            A block is a <code className="font-mono text-xs">## [tag:Name]</code> group that turns on and off on its own.
          </>
        )
      }
      submitLabel={profile ? "Save" : exclusive ? "Create environment" : "Create block"}
      canSubmit={name.trim() !== ""}
      onSubmit={submit}
      onClose={onClose}
    >
      <Field id={ids.name} label="Name">
        <Input id={ids.name} value={name} onChange={(e) => setName(e.target.value)} autoFocus spellCheck={false} />
      </Field>

      {exclusive && (
        <div className="flex flex-col gap-2">
          <span className="text-muted-foreground">Color</span>
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
      )}
    </FormDialog>
  );
}
