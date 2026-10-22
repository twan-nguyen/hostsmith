import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export type BannerTone = "success" | "info" | "warning" | "danger";

const TONES: Record<BannerTone, { box: string; icon: React.ReactNode }> = {
  success: { box: "border-success/30 bg-success/8", icon: <CircleCheck className="text-success" /> },
  info: { box: "border-border bg-muted/60", icon: <Info className="text-muted-foreground" /> },
  warning: { box: "border-warning/40 bg-warning/10", icon: <TriangleAlert className="text-warning" /> },
  danger: { box: "border-destructive/30 bg-destructive/8", icon: <CircleAlert className="text-destructive" /> },
};

export type BannerAction = { label: string; onClick: () => void; primary?: boolean; disabled?: boolean };

/**
 * The one message slot under a pane's header: a counted fact, why it matters
 * and what can be done about it.
 */
export function Banner({
  tone,
  title,
  children,
  actions = [],
  onDismiss,
  className,
}: {
  tone: BannerTone;
  title: React.ReactNode;
  children?: React.ReactNode;
  actions?: BannerAction[];
  onDismiss?: () => void;
  className?: string;
}) {
  const { box, icon } = TONES[tone];
  return (
    <div
      role={tone === "danger" ? "alert" : "status"}
      className={cn(
        "flex animate-fade-in items-start gap-3 rounded-lg border px-4 py-3 [&>svg]:mt-0.5 [&>svg]:size-4 [&>svg]:shrink-0",
        box,
        className,
      )}
    >
      {icon}
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <p className="font-medium">{title}</p>
        {children && <div className="text-muted-foreground">{children}</div>}
        {actions.length > 0 && (
          <div className="mt-1.5 flex flex-wrap gap-2">
            {actions.map((action) => (
              <Button
                key={action.label}
                size="sm"
                variant={action.primary ? "default" : tone === "danger" ? "destructive" : "outline"}
                onClick={action.onClick}
                disabled={action.disabled}
              >
                {action.label}
              </Button>
            ))}
          </div>
        )}
      </div>
      {onDismiss && (
        <button
          onClick={onDismiss}
          aria-label="Close"
          className="-mr-1 rounded-md p-0.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        >
          <X className="size-4" />
        </button>
      )}
    </div>
  );
}
