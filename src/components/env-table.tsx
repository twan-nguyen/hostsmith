import { ChevronDown, KeyRound } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { PaneEmpty } from "@/components/pane-empty";
import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Switch } from "@/components/ui/switch";
import { type TableRow, isSecretKey, maskSecret, maskVariant, rowId, tableRows } from "@/lib/env-table";
import { type EnvEntry, envDuplicateEntry, envSelectVariant, envToggleEntry } from "@/lib/hosven";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

/** A revealed secret hides itself again after this long. */
const REVEAL_MS = 30_000;

const GRID = { gridTemplateColumns: "240px minmax(0,1fr) auto" };

/**
 * The selected file's variables as a key and value table. Keys defined more
 * than once fold into one row that picks which definition is on.
 */
export function EnvTable({
  entries,
  query,
  onAdd,
  onEdit,
  onDelete,
}: {
  entries: EnvEntry[];
  query: string;
  onAdd: () => void;
  onEdit: (entry: EnvEntry) => void;
  onDelete: (entry: EnvEntry) => void;
}) {
  const run = useStore((s) => s.run);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [revealed, setRevealed] = useState<string | null>(null);
  const revealTimer = useRef<number | undefined>(undefined);

  const rows = useMemo(() => tableRows(entries, query), [entries, query]);

  useEffect(() => () => window.clearTimeout(revealTimer.current), []);

  // One value at a time, and never for long.
  function toggleReveal(id: string) {
    window.clearTimeout(revealTimer.current);
    if (revealed === id) {
      setRevealed(null);
      return;
    }
    setRevealed(id);
    revealTimer.current = window.setTimeout(() => setRevealed((r) => (r === id ? null : r)), REVEAL_MS);
  }

  function toggleExpanded(key: string) {
    setExpanded((keys) => {
      const next = new Set(keys);
      if (!next.delete(key)) next.add(key);
      return next;
    });
  }

  if (rows.length === 0) {
    return query.trim() ? (
      <PaneEmpty title="Không có kết quả" detail={`Không có biến nào khớp ${query.trim()}.`} />
    ) : (
      <PaneEmpty title="File trống" detail="File này chưa có biến nào.">
        <Button onClick={onAdd}>Thêm biến</Button>
      </PaneEmpty>
    );
  }

  const rowProps = { revealed, onReveal: toggleReveal, onEdit, onDelete, run };

  return (
    <div className="min-h-0 flex-1 overflow-auto" data-testid="env-table">
      <div
        className="sticky top-0 z-10 grid h-8 items-center gap-4 border-b border-border bg-card px-5 text-xs text-muted-foreground"
        style={GRID}
      >
        <span>Khoá</span>
        <span>Giá trị</span>
        <span className="sr-only">Thao tác</span>
      </div>
      {rows.map((row) =>
        row.kind === "entry" ? (
          <VariableRow key={rowId(row)} entry={row.entry} {...rowProps} />
        ) : (
          <VariantRows
            key={rowId(row)}
            row={row}
            expanded={expanded.has(row.key)}
            onToggleExpanded={() => toggleExpanded(row.key)}
            {...rowProps}
          />
        ),
      )}
    </div>
  );
}

type RowProps = {
  revealed: string | null;
  onReveal: (id: string) => void;
  onEdit: (entry: EnvEntry) => void;
  onDelete: (entry: EnvEntry) => void;
  run: <T>(action: Promise<T>) => Promise<T | undefined>;
};

function VariableRow({ entry, revealed, onReveal, onEdit, onDelete, run }: { entry: EnvEntry } & RowProps) {
  const secret = isSecretKey(entry.key);
  const shown = !secret || revealed === entry.id;
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div
          onDoubleClick={() => onEdit(entry)}
          data-testid={`env-row-${entry.key}`}
          className="grid min-h-9 cursor-default items-center gap-4 border-b border-border/50 px-5 transition-colors duration-100 select-none hover:bg-accent data-[state=open]:bg-accent"
          style={GRID}
        >
          <KeyCell name={entry.key} dimmed={!entry.enabled} />
          <ValueCell
            value={shown ? entry.value : maskSecret(entry.value)}
            masked={!shown}
            title={entry.comment || undefined}
            dimmed={!entry.enabled}
          />
          <span className="flex items-center justify-end gap-2" onDoubleClick={(e) => e.stopPropagation()}>
            {secret && (
              <>
                <KeyRound className="size-3.5 text-muted-foreground" aria-label="Giá trị bí mật" />
                <Button size="xs" variant="ghost" onClick={() => onReveal(entry.id)}>
                  {shown ? "Ẩn" : "Hiện"}
                </Button>
              </>
            )}
            <Switch
              size="sm"
              checked={entry.enabled}
              onCheckedChange={() => void run(envToggleEntry(entry.id))}
              aria-label={`${entry.key}, ${entry.enabled ? "bật" : "tắt"}`}
            />
          </span>
        </div>
      </ContextMenuTrigger>
      <EntryMenu entry={entry} onEdit={onEdit} onDelete={onDelete} run={run} />
    </ContextMenu>
  );
}

function VariantRows({
  row,
  expanded,
  onToggleExpanded,
  revealed,
  onReveal,
  onEdit,
  onDelete,
  run,
}: {
  row: Extract<TableRow, { kind: "variants" }>;
  expanded: boolean;
  onToggleExpanded: () => void;
} & RowProps) {
  const id = rowId(row);
  const secret = isSecretKey(row.key);
  const shown = !secret || revealed === id;
  const display = (value: string) => (shown ? value : maskVariant(value));
  const { active } = row;

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <div
            role="button"
            tabIndex={0}
            aria-expanded={expanded}
            onClick={onToggleExpanded}
            onKeyDown={(e) => {
              if (e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return;
              e.preventDefault();
              onToggleExpanded();
            }}
            onDoubleClick={() => active && onEdit(active)}
            data-testid={`env-row-${row.key}`}
            className="grid min-h-9 cursor-default items-center gap-4 border-b border-border/50 px-5 transition-colors duration-100 outline-none select-none hover:bg-accent focus-visible:bg-accent data-[state=open]:bg-accent"
            style={GRID}
          >
            <KeyCell name={row.key} dimmed={!active} />
            {active ? (
              <ValueCell value={display(active.value)} masked={!shown} title={active.comment || undefined} />
            ) : (
              <span className="text-muted-foreground">Tắt</span>
            )}
            <span
              className="flex items-center justify-end gap-2 text-xs text-muted-foreground"
              onDoubleClick={(e) => e.stopPropagation()}
            >
              {row.enabledCount > 1 && (
                <span className="flex items-center gap-1.5 text-warning">
                  <span className="size-1.5 rounded-full bg-warning" />
                  {row.enabledCount} đang bật
                </span>
              )}
              {secret && (
                <Button
                  size="xs"
                  variant="ghost"
                  onClick={(e) => {
                    e.stopPropagation();
                    onReveal(id);
                  }}
                >
                  {shown ? "Ẩn" : "Hiện"}
                </Button>
              )}
              <span className="flex items-center gap-1 tabular-nums">
                {row.entries.length} giá trị
                <ChevronDown
                  className={cn("size-3 transition-transform duration-200 ease-out-quart", expanded && "rotate-180")}
                />
              </span>
            </span>
          </div>
        </ContextMenuTrigger>
        {active && (
          <ContextMenuContent>
            <ContextMenuItem onSelect={() => onEdit(active)}>Sửa giá trị đang dùng</ContextMenuItem>
            <ContextMenuSeparator />
            <ContextMenuItem variant="destructive" onSelect={() => onDelete(active)}>
              Xoá giá trị đang dùng
            </ContextMenuItem>
          </ContextMenuContent>
        )}
      </ContextMenu>

      {expanded &&
        row.entries.map((variant) => (
          <ContextMenu key={variant.id}>
            <ContextMenuTrigger asChild>
              <button
                role="radio"
                aria-checked={variant.enabled}
                onClick={() => void run(envSelectVariant(row.key, variant.id))}
                onDoubleClick={() => onEdit(variant)}
                title={variant.enabled ? "Nhấn để tắt khoá này" : "Nhấn để dùng giá trị này"}
                className="flex min-h-8 w-full animate-fade-in items-center gap-2.5 border-b border-border/50 bg-muted/30 pr-5 pl-[calc(240px+2.25rem)] text-left transition-colors duration-100 hover:bg-accent data-[state=open]:bg-accent"
              >
                <span
                  aria-hidden
                  className={cn(
                    "flex size-3.5 shrink-0 items-center justify-center rounded-full border",
                    variant.enabled ? "border-primary" : "border-muted-foreground/60",
                  )}
                >
                  {variant.enabled && <span className="size-1.5 rounded-full bg-primary" />}
                </span>
                <span
                  className={cn(
                    "min-w-0 truncate font-mono text-xs",
                    variant.enabled ? "text-foreground" : "text-muted-foreground",
                  )}
                >
                  {display(variant.value) || " "}
                </span>
                {variant.comment && (
                  <span className="min-w-0 truncate text-xs text-muted-foreground">{variant.comment}</span>
                )}
              </button>
            </ContextMenuTrigger>
            <EntryMenu entry={variant} onEdit={onEdit} onDelete={onDelete} run={run} />
          </ContextMenu>
        ))}
    </>
  );
}

function EntryMenu({
  entry,
  onEdit,
  onDelete,
  run,
}: { entry: EnvEntry } & Pick<RowProps, "onEdit" | "onDelete" | "run">) {
  return (
    <ContextMenuContent>
      <ContextMenuItem onSelect={() => onEdit(entry)}>Sửa</ContextMenuItem>
      <ContextMenuItem onSelect={() => void run(envDuplicateEntry(entry.id))}>Nhân đôi</ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem variant="destructive" onSelect={() => onDelete(entry)}>
        Xoá
      </ContextMenuItem>
    </ContextMenuContent>
  );
}

function KeyCell({ name, dimmed }: { name: string; dimmed?: boolean }) {
  return (
    <span title={name} className={cn("truncate font-mono text-xs font-medium", dimmed && "opacity-50")}>
      {name}
    </span>
  );
}

function ValueCell({
  value,
  masked,
  title,
  dimmed,
}: {
  value: string;
  masked: boolean;
  title?: string;
  dimmed?: boolean;
}) {
  return (
    <span
      title={title}
      className={cn(
        "truncate font-mono text-xs",
        masked ? "tracking-widest text-muted-foreground" : "text-foreground/90",
        dimmed && "opacity-50",
      )}
    >
      {value}
    </span>
  );
}
