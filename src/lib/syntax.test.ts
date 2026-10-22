import { describe, expect, it } from "vitest";

import { type Token, toggleComment, tokenize } from "./syntax";

const kinds = (tokens: Token[]) => tokens.map((t) => t.kind);
const text = (line: string, tokens: Token[]) => tokens.map((t) => line.slice(t.from, t.to));

describe("hosts lines", () => {
  it("has nothing to colour on an empty line", () => {
    expect(tokenize("")).toEqual([]);
    expect(tokenize("   ")).toEqual([]);
  });

  it("colours the address and the hostnames differently", () => {
    const line = "127.0.0.1\tapi.test www.test # local";
    const tokens = tokenize(line);
    expect(kinds(tokens)).toEqual(["ip", "host", "host", "comment"]);
    expect(text(line, tokens)).toEqual(["127.0.0.1", "api.test", "www.test", "# local"]);
  });

  it("colours a block marker as one span", () => {
    expect(kinds(tokenize("## [tag:Local]"))).toEqual(["marker"]);
    expect(kinds(tokenize("## [/tag:Local]"))).toEqual(["marker"]);
  });

  it("dims a disabled entry instead of treating it as a comment", () => {
    const tokens = tokenize("# 10.0.0.5\tapi.test");
    expect(kinds(tokens)).toEqual(["comment", "ip", "host"]);
    expect(tokens.every((t) => t.disabled)).toBe(true);
  });

  it("keeps a plain comment a comment", () => {
    expect(tokenize("# Host Database")).toEqual([{ from: 0, to: 15, kind: "comment" }]);
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
