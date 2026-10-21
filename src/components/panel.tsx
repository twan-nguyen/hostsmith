import { cn } from "@/lib/utils";

/**
 * The rounded content panel that floats above the window canvas. The frame
 * stays put when switching views; only its content eases in.
 */
export function Panel({ className, children, ...props }: React.ComponentProps<"main">) {
  return (
    <main
      className={cn(
        "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border border-border bg-card shadow-[0_1px_3px_oklch(0_0_0/30%)]",
        className,
      )}
      {...props}
    >
      <div className="flex min-h-0 flex-1 animate-view-in flex-col">{children}</div>
    </main>
  );
}

/** Panel title bar; also drags the window. */
export function PanelHeader({ className, ...props }: React.ComponentProps<"header">) {
  return (
    <header
      data-tauri-drag-region
      className={cn("flex h-12 shrink-0 items-center gap-3 border-b border-border px-5", className)}
      {...props}
    />
  );
}
