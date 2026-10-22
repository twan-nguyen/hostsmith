import { cn } from "@/lib/utils";

export type StatusTone = "on" | "off" | "muted";

const TONES: Record<StatusTone, string> = {
  on: "border-success after:bg-success",
  muted: "border-muted-foreground after:bg-muted-foreground",
  off: "border-muted-foreground/60",
};

/** A ring, filled when on, like Linear's status icons. */
export function StatusIcon({
  on,
  tone = on ? "on" : "off",
  className,
}: {
  on?: boolean;
  tone?: StatusTone;
  className?: string;
}) {
  return (
    <span
      aria-hidden
      className={cn(
        "relative inline-block size-3 shrink-0 rounded-full border-[1.5px] transition-colors duration-150 after:absolute after:inset-[2px] after:rounded-full after:transition-colors after:duration-150",
        TONES[tone],
        className,
      )}
    />
  );
}
