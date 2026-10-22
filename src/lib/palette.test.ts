import { describe, expect, it } from "vitest";

import { MAX_PER_SECTION, paletteSections, type PaletteSources } from "./palette";

const env = (name: string, extra: Partial<PaletteSources["environments"][number]> = {}) => ({
  id: name.toLowerCase(),
  name,
  color: "blue",
  shortcutNumber: null,
  entryCount: 1,
  active: false,
  ...extra,
});

const sources = (overrides: Partial<PaletteSources> = {}): PaletteSources => ({
  environments: [env("Release"), env("Production"), env("Master")],
  anyActive: false,
  ...overrides,
});

const flat = (query: string, s = sources()) => paletteSections(query, s).flatMap((section) => section.items);

describe("paletteSections", () => {
  it("shows the profile actions and suggestions for an empty query", () => {
    expect(paletteSections("", sources()).map((s) => s.title)).toEqual(["Profile", "Gợi ý"]);
  });

  it("lists every environment, the hosts file and ports, and no deactivation when nothing is on", () => {
    const items = flat("");
    expect(items).toHaveLength(5);
    expect(items.some((c) => c.action.type === "deactivate")).toBe(false);
  });

  it("does not offer an environment without entries", () => {
    const items = flat("", sources({ environments: [env("Empty", { entryCount: 0 })] }));
    expect(items.some((c) => c.title === "Chuyển sang Empty")).toBe(false);
  });

  it("ranks Release first for 'rel'", () => {
    expect(flat("rel")[0].title).toContain("Release");
  });

  it("still offers the hosts search when nothing matches", () => {
    const items = flat("zzzqqq");
    expect(items.map((c) => c.action.type)).toEqual(["searchHosts"]);
  });

  it("surfaces the hosts file for 'hosts'", () => {
    expect(flat("hosts").some((c) => c.id === "open-systemHosts")).toBe(true);
  });

  it("marks the active environment and offers to turn it off", () => {
    const items = flat("", sources({ environments: [env("Release", { active: true })], anyActive: true }));
    expect(items.find((c) => c.title === "Chuyển sang Release")?.subtitle).toBe("Đang hoạt động");
    expect(items.find((c) => c.action.type === "deactivate")?.shortcut).toBe("⌘0");
  });

  it("shows ⌘1–⌘9 shortcuts only", () => {
    const items = flat("", sources({ environments: [env("A", { shortcutNumber: 2 }), env("B", { shortcutNumber: 12 })] }));
    expect(items.find((c) => c.title === "Chuyển sang A")?.shortcut).toBe("⌘2");
    expect(items.find((c) => c.title === "Chuyển sang B")?.shortcut).toBeUndefined();
  });

  it("offers the hosts search only for a non-empty query", () => {
    expect(flat("").some((c) => c.action.type === "searchHosts")).toBe(false);
    expect(flat("abc").filter((c) => c.action.type === "searchHosts")).toHaveLength(1);
  });

  it("caps each section", () => {
    const many = Array.from({ length: 100 }, (_, i) => env(`Profile-${i}`));
    const switches = flat("Profile", sources({ environments: many })).filter((c) => c.action.type === "switch");
    expect(switches).toHaveLength(MAX_PER_SECTION);
  });
});
