import { Search } from "lucide-react";
import { useEffect, useRef } from "react";

import { Kbd } from "@/components/ui/kbd";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

/**
 * The search field in a pane's header. ⌘F focuses it; Esc clears it, or
 * leaves it once empty.
 */
export function PaneSearch({
  value,
  onChange,
  onEnter,
  placeholder,
  disabled,
  testId,
  className,
}: {
  value: string;
  onChange: (value: string) => void;
  onEnter?: () => void;
  /** Also the field's accessible name. */
  placeholder: string;
  disabled?: boolean;
  testId?: string;
  className?: string;
}) {
  const input = useRef<HTMLInputElement>(null);
  const findRequest = useStore((s) => s.findRequest);

  // A request made before the pane opened is not one for this field.
  const seen = useRef(findRequest);
  useEffect(() => {
    if (findRequest === seen.current) return;
    seen.current = findRequest;
    input.current?.focus();
    input.current?.select();
  }, [findRequest]);

  return (
    <label
      className={cn(
        "flex h-7 min-w-32 shrink items-center gap-2 rounded-md px-2 text-muted-foreground transition-colors duration-100 focus-within:bg-accent hover:bg-accent",
        className,
      )}
    >
      <Search className="size-3.5 shrink-0" />
      <input
        ref={input}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            onEnter?.();
          } else if (e.key === "Escape") {
            if (value) onChange("");
            else e.currentTarget.blur();
          }
        }}
        disabled={disabled}
        placeholder={placeholder}
        aria-label={placeholder}
        spellCheck={false}
        data-testid={testId}
        className="min-w-0 flex-1 bg-transparent text-foreground outline-none placeholder:text-muted-foreground disabled:opacity-50"
      />
      <Kbd>⌘F</Kbd>
    </label>
  );
}
