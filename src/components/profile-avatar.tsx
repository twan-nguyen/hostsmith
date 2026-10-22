import type { Profile } from "@/lib/hostsmith";
import { cn } from "@/lib/utils";

/** Hues for profiles whose colour is neutral, picked from the name so each keeps its own. */
const HUES = [276, 220, 190, 155, 80, 50, 25, 330];

/** The three environment colours, as hues of the same lightness and chroma. */
const COLOR_HUES: Partial<Record<Profile["color"], number>> = { blue: 250, amber: 70, red: 25 };

export function profileHue(profile: Pick<Profile, "name" | "color">): number {
  const fixed = COLOR_HUES[profile.color];
  if (fixed !== undefined) return fixed;
  const hash = [...profile.name].reduce((sum, ch) => sum + ch.charCodeAt(0), 0);
  return HUES[hash % HUES.length];
}

/** A small tile with the profile's first letter, as a workspace avatar in Linear. */
export function ProfileAvatar({
  profile,
  className,
}: {
  profile: Pick<Profile, "name" | "color">;
  className?: string;
}) {
  return (
    <span
      aria-hidden
      className={cn(
        "flex size-4 shrink-0 items-center justify-center rounded text-[10px] leading-none font-semibold text-white",
        className,
      )}
      style={{ background: `oklch(0.58 0.13 ${profileHue(profile)})` }}
    >
      {profile.name.slice(0, 1).toUpperCase()}
    </span>
  );
}
