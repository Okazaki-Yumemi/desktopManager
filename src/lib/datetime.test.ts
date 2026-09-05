import { describe, expect, it } from "vitest";
import { formatDuration, greetingForHour } from "./datetime";

describe("greetingForHour", () => {
  it("maps hours to greetings at boundaries", () => {
    expect(greetingForHour(0)).toBe("Late night");
    expect(greetingForHour(4)).toBe("Late night");
    expect(greetingForHour(5)).toBe("Good morning");
    expect(greetingForHour(11)).toBe("Good morning");
    expect(greetingForHour(12)).toBe("Good afternoon");
    expect(greetingForHour(17)).toBe("Good afternoon");
    expect(greetingForHour(18)).toBe("Good evening");
    expect(greetingForHour(23)).toBe("Good evening");
  });
});

describe("formatDuration", () => {
  it("formats seconds, minutes and hours", () => {
    expect(formatDuration(0)).toBe("0s");
    expect(formatDuration(45)).toBe("45s");
    expect(formatDuration(60)).toBe("1m 0s");
    expect(formatDuration(125)).toBe("2m 5s");
    expect(formatDuration(3600)).toBe("1h 0m");
    expect(formatDuration(5430)).toBe("1h 30m");
  });

  it("clamps negative input", () => {
    expect(formatDuration(-10)).toBe("0s");
  });
});
