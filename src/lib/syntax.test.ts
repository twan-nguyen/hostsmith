import { describe, expect, it } from "vitest";

import { type Token, toggleComment, tokenize } from "./syntax";

const kinds = (tokens: Token[]) => tokens.map((t) => t.kind);
const text = (line: string, tokens: Token[]) => tokens.map((t) => line.slice(t.from, t.to));

describe("hosts lines", () => {
  it("has nothing to colour on an empty line", () => {
    expect(tokenize("", "hosts")).toEqual([]);
    expect(tokenize("   ", "hosts")).toEqual([]);
  });

  it("colours the address and the hostnames differently", () => {
    const line = "127.0.0.1\tapi.test www.test # local";
    const tokens = tokenize(line, "hosts");
    expect(kinds(tokens)).toEqual(["ip", "host", "host", "comment"]);
    expect(text(line, tokens)).toEqual(["127.0.0.1", "api.test", "www.test", "# local"]);
  });

  it("colours a block marker as one span", () => {
    expect(kinds(tokenize("## [tag:Local]", "hosts"))).toEqual(["marker"]);
    expect(kinds(tokenize("## [/tag:Local]", "hosts"))).toEqual(["marker"]);
  });

  it("dims a disabled entry instead of treating it as a comment", () => {
    const tokens = tokenize("# 10.0.0.5\tapi.test", "hosts");
    expect(kinds(tokens)).toEqual(["comment", "ip", "host"]);
    expect(tokens.every((t) => t.disabled)).toBe(true);
  });

  it("keeps a plain comment a comment", () => {
    expect(tokenize("# Host Database", "hosts")).toEqual([{ from: 0, to: 15, kind: "comment" }]);
  });
});

describe("dotenv lines", () => {
  it("colours key, equals sign, value and comment differently", () => {
    const line = "API_URL=https://x.test # prod";
    const tokens = tokenize(line, "env");
    expect(kinds(tokens)).toEqual(["key", "punct", "value", "comment"]);
    expect(text(line, tokens)).toEqual(["API_URL", "=", "https://x.test", "# prod"]);
  });

  it("dims a disabled variable instead of treating it as a comment", () => {
    const tokens = tokenize("# DEBUG=true", "env");
    expect(kinds(tokens)).toEqual(["comment", "key", "punct", "value"]);
    expect(tokens.every((t) => t.disabled)).toBe(true);
  });

  it("marks export as a keyword", () => {
    const line = "export PORT=3000";
    expect(text(line, tokenize(line, "env"))).toEqual(["export", "PORT", "=", "3000"]);
    expect(kinds(tokenize(line, "env"))[0]).toBe("keyword");
  });

  it("keeps prose comments whole", () => {
    expect(kinds(tokenize("# Database settings", "env"))).toEqual(["comment"]);
  });

  it("has no value span for an empty value", () => {
    expect(kinds(tokenize("EMPTY=", "env"))).toEqual(["key", "punct"]);
  });
});

describe("toggleComment", () => {
  it("comments out every line with content", () => {
    expect(toggleComment(["a b", "", "  c d"])).toEqual(["# a b", "", "  # c d"]);
  });

  it("uncomments when every line already is", () => {
    expect(toggleComment(["# a b", "#c d"])).toEqual(["a b", "c d"]);
  });

  it("comments all when only some are", () => {
    expect(toggleComment(["# a", "b"])).toEqual(["# # a", "# b"]);
  });
});
