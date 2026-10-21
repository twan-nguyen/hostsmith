/** A centred title, one sentence of why, and what to do about it. */
export function PaneEmpty({
  title,
  detail,
  children,
}: {
  title: string;
  detail?: React.ReactNode;
  /** Buttons under the text. */
  children?: React.ReactNode;
}) {
  return (
    <div className="flex animate-fade-in flex-col items-center gap-1 px-8 py-20 text-center">
      <div className="font-medium">{title}</div>
      {detail && <div className="max-w-md break-words text-muted-foreground">{detail}</div>}
      {children && <div className="mt-3 flex flex-wrap justify-center gap-2">{children}</div>}
    </div>
  );
}
