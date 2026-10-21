/**
 * Splits a hosts or dotenv line into coloured spans. Pure, so the rules are
 * tested without an editor; the code editor turns the spans into marks.
 */
export type TokenKind = "ip" | "host" | "comment" | "marker" | "key" | "value" | "punct" | "keyword";

export type Token = {
  from: number;
  to: number;
  kind: TokenKind;
  /** Part of a commented-out entry: shown dimmed, not as a comment. */
  disabled?: boolean;
};

export type Language = "hosts" | "env";

const TAG_MARKER = /^##\s*\[\/?tag:[^\]]+\]\s*$/;
const IPV4 = /^\d{1,3}(\.\d{1,3}){3}$/;
const ENV_ASSIGNMENT = /^(export\s+)?[A-Za-z_][A-Za-z0-9_]*\s*=/;

export function tokenize(line: string, language: Language): Token[] {
  return language === "hosts" ? hostsLine(line) : envLine(line);
}

function leading(line: string) {
  return line.length - line.trimStart().length;
}

/** `# ` and friends in front of a disabled entry, and where its body starts. */
function disabledPrefix(line: string, start: number) {
  let i = start;
  while (i < line.length && (line[i] === "#" || line[i] === " " || line[i] === "\t")) i++;
  return i;
}

function hostsLine(line: string): Token[] {
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

function envLine(line: string): Token[] {
  const trimmed = line.trim();
  if (!trimmed) return [];
  const start = leading(line);
  const end = start + trimmed.length;

  let bodyStart = start;
  let disabled = false;
  if (trimmed.startsWith("#")) {
    bodyStart = disabledPrefix(line, start);
    if (!ENV_ASSIGNMENT.test(line.slice(bodyStart, end))) return [{ from: start, to: end, kind: "comment" }];
    disabled = true;
  }

  const tokens: Token[] = [];
  if (disabled) tokens.push({ from: start, to: bodyStart, kind: "comment", disabled });
  const body = line.slice(bodyStart, end);
  const eq = body.indexOf("=");
  if (eq < 0) return tokens;

  let keyStart = bodyStart;
  const exported = /^export\s+/.exec(body);
  if (exported) {
    tokens.push({ from: bodyStart, to: bodyStart + 6, kind: "keyword", disabled });
    keyStart = bodyStart + exported[0].length;
  }
  const keyEnd = bodyStart + body.slice(0, eq).trimEnd().length;
  tokens.push({ from: keyStart, to: keyEnd, kind: "key", disabled });
  tokens.push({ from: bodyStart + eq, to: bodyStart + eq + 1, kind: "punct", disabled });

  const valueStart = bodyStart + eq + 1;
  const rest = line.slice(valueStart, end);
  const comment = rest.search(/[ \t]#/);
  const valueEnd = comment >= 0 ? valueStart + comment : end;
  if (valueEnd > valueStart) tokens.push({ from: valueStart, to: valueEnd, kind: "value", disabled });
  if (comment >= 0) tokens.push({ from: valueStart + comment + 1, to: end, kind: "comment", disabled });
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
