import { describe, expect, it } from "vitest";

import {
  envStatus,
  isSecretKey,
  isValidEnvKey,
  maskSecret,
  maskVariant,
  matchesQuery,
  rowId,
  tableRows,
  variableCount,
} from "@/lib/env-table";
import type { EnvEntry } from "@/lib/hosven";

let seq = 0;
function variable(key: string, value: string, enabled = true, comment = ""): EnvEntry {
  return { id: `id-${++seq}`, key, value, comment, enabled, exported: false, isBlankOrComment: false };
}
function line(rawLine: string): EnvEntry {
  return {
    id: `id-${++seq}`,
    key: "",
    value: "",
    comment: "",
    enabled: true,
    exported: false,
    isBlankOrComment: true,
    rawLine,
  };
}

describe("secret keys", () => {
  it("matches the suffixes Hosven hides, ignoring case", () => {
    for (const key of ["API_KEY", "app_secret", "GITHUB_TOKEN", "DB_PASSWORD", "SENTRY_DSN", "GCP_CREDENTIALS"]) {
      expect(isSecretKey(key), key).toBe(true);
    }
  });

  it("leaves other keys visible", () => {
    for (const key of ["KEY", "SECRET", "KEYCLOAK_URL", "TOKEN_TTL", "PASSWORD_MIN_LENGTH", "PORT"]) {
      expect(isSecretKey(key), key).toBe(false);
    }
  });
});

describe("masking", () => {
  it("shows one bullet per character between 8 and 24", () => {
    expect(maskSecret("")).toBe("•".repeat(8));
    expect(maskSecret("abc")).toBe("•".repeat(8));
    expect(maskSecret("x".repeat(12))).toBe("•".repeat(12));
    expect(maskSecret("x".repeat(40))).toBe("•".repeat(24));
  });

  it("counts characters, not UTF-16 units", () => {
    expect(maskSecret("🔑".repeat(10))).toBe("•".repeat(10));
  });

  it("keeps the last four characters of a variant", () => {
    expect(maskVariant("sk_live_abcd1234")).toBe("••••••1234");
    expect(maskVariant("ab")).toBe("••••••ab");
    expect(maskVariant("")).toBe("••••••");
  });
});

describe("key validation", () => {
  it("accepts the parser's ASCII keys", () => {
    for (const key of ["FOO", "_PRIVATE", "a1_b2", "X"]) expect(isValidEnvKey(key), key).toBe(true);
  });

  it("rejects digits first, spaces, dashes and accented letters", () => {
    for (const key of ["", "1FOO", "FOO BAR", "FOO-BAR", "TÊN", "export FOO"]) {
      expect(isValidEnvKey(key), key).toBe(false);
    }
  });
});

describe("table rows", () => {
  it("leaves out blank and comment lines", () => {
    const rows = tableRows([line("# header"), variable("A", "1"), line(""), variable("B", "2")], "");
    expect(rows.map((r) => (r.kind === "entry" ? r.entry.key : r.key))).toEqual(["A", "B"]);
  });

  it("groups a key defined twice into one variants row where it first appears", () => {
    const local = variable("DB", "localhost", true);
    const staging = variable("DB", "staging", false);
    const rows = tableRows([local, variable("PORT", "3000"), staging], "");
    expect(rows).toHaveLength(2);
    const [first] = rows;
    expect(first.kind).toBe("variants");
    if (first.kind !== "variants") return;
    expect(first.entries).toEqual([local, staging]);
    expect(first.active).toBe(local);
    expect(first.enabledCount).toBe(1);
    expect(rowId(first)).toBe("variants:DB");
  });

  it("counts every definition that is on, and has no active one when all are off", () => {
    const both = tableRows([variable("A", "1"), variable("A", "2")], "")[0];
    expect(both.kind === "variants" && both.enabledCount).toBe(2);
    const none = tableRows([variable("A", "1", false), variable("A", "2", false)], "")[0];
    expect(none.kind === "variants" && none.active).toBeNull();
  });

  it("keeps a variants row whole when only one definition matches the search", () => {
    const rows = tableRows([variable("DB", "localhost"), variable("DB", "staging.internal")], "staging");
    expect(rows).toHaveLength(1);
    expect(rows[0].kind === "variants" && rows[0].entries).toHaveLength(2);
  });

  it("filters by key, value and comment, ignoring case", () => {
    const entries = [variable("API_URL", "http://x"), variable("PORT", "3000", true, "Cổng dev"), variable("MODE", "debug")];
    const keys = (q: string) => tableRows(entries, q).map((r) => (r.kind === "entry" ? r.entry.key : r.key));
    expect(keys("api")).toEqual(["API_URL"]);
    expect(keys("DEBUG")).toEqual(["MODE"]);
    expect(keys("cổng")).toEqual(["PORT"]);
    expect(keys("  ")).toEqual(["API_URL", "PORT", "MODE"]);
    expect(keys("nothing")).toEqual([]);
  });

  it("matches comment lines by their raw text", () => {
    expect(matchesQuery(line("# Database"), "data")).toBe(true);
    expect(matchesQuery(line("# Database"), "redis")).toBe(false);
  });
});

describe("status", () => {
  const entries = [line("# x"), variable("A", "1"), variable("B", "2", false)];

  it("counts variables, switched off ones included", () => {
    expect(variableCount(entries)).toBe(2);
    expect(envStatus(entries, false)).toEqual({ tone: "ok", label: "2 biến" });
  });

  it("says when edits are pending", () => {
    expect(envStatus(entries, true)).toEqual({ tone: "warning", label: "Chưa áp dụng" });
  });
});
