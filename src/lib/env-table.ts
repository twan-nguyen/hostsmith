/**
 * What the env table draws, worked out from the selected file's entries.
 * Pure, so grouping, masking and filtering are tested without a window.
 */
import type { EnvEntry } from "@/lib/hosven";

/** Keys whose values are hidden until revealed, as in Hosven 1.x. */
const SECRET_KEY = /_(KEY|SECRET|TOKEN|PASSWORD|DSN|CREDENTIALS)$/i;

/** The key rule the dotenv parser uses: ASCII letters, digits and `_`, no leading digit. */
const ENV_KEY = /^[A-Za-z_][A-Za-z0-9_]*$/;

export const INVALID_KEY_MESSAGE = "Khoá chỉ gồm chữ cái không dấu, chữ số và dấu _, không bắt đầu bằng chữ số";

/** The comment a variable added from `.env.example` carries into the file. */
export const REQUIRED_COMMENT = "bắt buộc theo .env.example";

const BULLET = "•";

export const isSecretKey = (key: string) => SECRET_KEY.test(key);

export const isValidEnvKey = (key: string) => ENV_KEY.test(key);

/** Counts characters, not UTF-16 units, so an emoji in a value is one bullet. */
const length = (value: string) => Array.from(value).length;

/** A hidden value: one bullet per character, at least 8 and at most 24. */
export function maskSecret(value: string): string {
  return BULLET.repeat(Math.max(8, Math.min(length(value), 24)));
}

/** A hidden variant: six bullets and the last four characters, enough to tell two apart. */
export function maskVariant(value: string): string {
  return BULLET.repeat(6) + Array.from(value).slice(-4).join("");
}

export type TableRow =
  | { kind: "entry"; entry: EnvEntry }
  | {
      kind: "variants";
      key: string;
      /** Every definition of the key, in file order. */
      entries: EnvEntry[];
      /** The first definition that is on, if any. */
      active: EnvEntry | null;
      enabledCount: number;
    };

export const rowId = (row: TableRow) => (row.kind === "entry" ? row.entry.id : `variants:${row.key}`);

/** Whether a variable matches the search, by key, value or comment, ignoring case. */
export function matchesQuery(entry: EnvEntry, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  if (entry.isBlankOrComment) return (entry.rawLine ?? "").toLowerCase().includes(q);
  return [entry.key, entry.value, entry.comment].some((field) => field.toLowerCase().includes(q));
}

/**
 * The table's rows. Blank and comment lines are left out (the backend keeps
 * them in the file). A key defined twice or more becomes one variants row,
 * placed where its first matching definition is.
 */
export function tableRows(entries: EnvEntry[], query: string): TableRow[] {
  const variables = entries.filter((e) => !e.isBlankOrComment);
  const byKey = new Map<string, EnvEntry[]>();
  for (const entry of variables) byKey.set(entry.key, [...(byKey.get(entry.key) ?? []), entry]);

  const rows: TableRow[] = [];
  const grouped = new Set<string>();
  for (const entry of variables) {
    if (!matchesQuery(entry, query)) continue;
    const definitions = byKey.get(entry.key) ?? [entry];
    if (definitions.length < 2) {
      rows.push({ kind: "entry", entry });
    } else if (!grouped.has(entry.key)) {
      grouped.add(entry.key);
      const enabled = definitions.filter((e) => e.enabled);
      rows.push({
        kind: "variants",
        key: entry.key,
        entries: definitions,
        active: enabled[0] ?? null,
        enabledCount: enabled.length,
      });
    }
  }
  return rows;
}

/** Keys `.env.example` defines and the file lacks, narrowed by the search. */
export function missingRows(missingKeys: string[], query: string): string[] {
  const q = query.trim().toLowerCase();
  return q ? missingKeys.filter((key) => key.toLowerCase().includes(q)) : missingKeys;
}

/** Variables, switched off ones included, as the write toast counts them. */
export const variableCount = (entries: EnvEntry[]) => entries.filter((e) => !e.isBlankOrComment).length;

export type EnvStatus = { tone: "warning" | "ok"; label: string };

/** The toolbar status: missing variables first, then pending edits, then the count. */
export function envStatus(entries: EnvEntry[], dirty: boolean, missingCount: number): EnvStatus {
  if (missingCount > 0) return { tone: "warning", label: `${missingCount} biến thiếu` };
  if (dirty) return { tone: "warning", label: "Chưa áp dụng" };
  return { tone: "ok", label: `${variableCount(entries)} biến` };
}
