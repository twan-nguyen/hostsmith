import type { ProfileColor } from "@/lib/hostsmith";

/**
 * Three environment hues and one neutral. `green` and `purple` stay readable
 * in stored profiles and both render as neutral.
 */
export const PROFILE_COLORS: Record<
  ProfileColor,
  { label: string; dot: string; chip: string; environment: boolean }
> = {
  blue: {
    label: "Local",
    dot: "bg-blue-500",
    chip: "bg-blue-500/12 text-blue-700 dark:text-blue-300",
    environment: true,
  },
  amber: {
    label: "Staging",
    dot: "bg-amber-500",
    chip: "bg-amber-500/15 text-amber-700 dark:text-amber-300",
    environment: true,
  },
  red: {
    label: "Production",
    dot: "bg-red-500",
    chip: "bg-red-500/12 text-red-700 dark:text-red-300",
    environment: true,
  },
  green: {
    label: "Neutral",
    dot: "bg-muted-foreground/60",
    chip: "bg-muted text-muted-foreground",
    environment: false,
  },
  purple: {
    label: "Neutral",
    dot: "bg-muted-foreground/60",
    chip: "bg-muted text-muted-foreground",
    environment: false,
  },
};

/** The choices the profile form offers, in this order. */
export const PICKER_COLORS: ProfileColor[] = ["blue", "amber", "red", "purple"];
