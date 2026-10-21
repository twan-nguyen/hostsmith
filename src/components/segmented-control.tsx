import { cn } from "@/lib/utils";

/** A small pill of mutually exclusive choices, like the appearance picker in Settings. */
export function SegmentedControl<T extends string>({
  value,
  options,
  onChange,
  disabled,
  label,
}: {
  value: T;
  options: { value: T; label: string; testId?: string }[];
  onChange: (value: T) => void;
  disabled?: boolean;
  /** Read by screen readers; the choices speak for themselves on screen. */
  label: string;
}) {
  return (
    <div role="radiogroup" aria-label={label} className="flex shrink-0 rounded-lg bg-muted p-0.5">
      {options.map((option) => (
        <button
          key={option.value}
          role="radio"
          aria-checked={value === option.value}
          data-testid={option.testId}
          disabled={disabled}
          onClick={() => onChange(option.value)}
          className={cn(
            "h-6 rounded-md px-2.5 text-[13px] whitespace-nowrap transition-colors duration-100 disabled:pointer-events-none disabled:opacity-50",
            value === option.value
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
