/**
 * Splits a hosts line into coloured spans. Pure, so the rules are
 * tested without an editor; the code editor turns the spans into marks.
 */
export type TokenKind = "ip" | "host" | "comment" | "marker";

export type Token = {
  from: number;
  to: number;
  kind: TokenKind;
  /** Part of a commented-out entry: shown dimmed, not as a comment. */
  disabled?: boolean;
};

const TAG_MARKER = /^##\s*\[\/?tag:[^\]]+\]\s*$/;
const IPV4 = /^\d{1,3}(\.\d{1,3}){3}$/;

function leading(line: string) {
  return line.length - line.trimStart().length;
}

/** `# ` and friends in front of a disabled entry, and where its body starts. */
function disabledPrefix(line: string, start: number) {
  let i = start;
  while (i < line.length && (line[i] === "#" || line[i] === " " || line[i] === "\t")) i++;
  return i;
}

export function tokenize(line: string): Token[] {
  const trimmed = line.trim();
  if (!trimmed) return [];
  const start = leading(line);
  const end = start + trimmed.length;
  if (TAG_MARKER.test(trimmed)) return [{ from: start, to: end, kind: "marker" }];

  let bodyStart = start;
  let disabled = false;
  if (trimmed.startsWith("#")) {
    bodyStart = disabledPrefix(line, start);
    const fields = line.slice(bodyStart, end).split(/\s+/).filter(Boolean);
    const head = fields[0] ?? "";
    disabled = fields.length >= 2 && (IPV4.test(head) || head.includes(":"));
    if (!disabled) return [{ from: start, to: end, kind: "comment" }];
  }

  const tokens: Token[] = [];
  if (disabled) tokens.push({ from: start, to: bodyStart, kind: "comment", disabled });
  const hash = line.indexOf("#", bodyStart);
  const bodyEnd = hash >= 0 ? hash : end;
  let first = true;
  for (const match of line.slice(bodyStart, bodyEnd).matchAll(/\S+/g)) {
    const from = bodyStart + match.index;
    tokens.push({ from, to: from + match[0].length, kind: first ? "ip" : "host", disabled });
    first = false;
  }
  if (hash >= 0) tokens.push({ from: hash, to: end, kind: "comment", disabled });
  return tokens;
}

/** Comments out every selected line, or uncomments them when all already are. */
export function toggleComment(lines: string[]): string[] {
  const content = lines.filter((l) => l.trim());
  const allCommented = content.length > 0 && content.every((l) => l.trimStart().startsWith("#"));
  return lines.map((line) => {
    if (!line.trim()) return line;
    const indent = line.slice(0, leading(line));
    const body = line.trimStart();
    if (allCommented) return indent + body.replace(/^# ?/, "");
    return `${indent}# ${body}`;
  });
}
