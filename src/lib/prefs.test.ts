import { describe, expect, it } from "vitest";
import { resolveEnum } from "./prefs";

describe("resolveEnum", () => {
  const allowed = ["standard", "soft", "oled"] as const;

  it("accepts values from the allow-list", () => {
    expect(resolveEnum(allowed, "soft", "standard")).toBe("soft");
    expect(resolveEnum(allowed, "standard", "standard")).toBe("standard");
  });

  it("falls back on unknown, non-string and missing values", () => {
    expect(resolveEnum(allowed, "neon", "standard")).toBe("standard");
    expect(resolveEnum(allowed, 42, "standard")).toBe("standard");
    expect(resolveEnum(allowed, null, "oled")).toBe("oled");
    expect(resolveEnum(allowed, undefined, "oled")).toBe("oled");
  });

  it("is case-sensitive", () => {
    expect(resolveEnum(allowed, "Soft", "standard")).toBe("standard");
  });
});
