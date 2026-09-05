import { describe, expect, it } from "vitest";
import { formatDuration, greetingForHour } from "./datetime";

describe("greetingForHour", () => {
  it("maps hours to greetings at boundaries", () => {
    expect(greetingForHour(0)).toBe("夜深了");
    expect(greetingForHour(4)).toBe("夜深了");
    expect(greetingForHour(5)).toBe("早上好");
    expect(greetingForHour(11)).toBe("早上好");
    expect(greetingForHour(12)).toBe("下午好");
    expect(greetingForHour(17)).toBe("下午好");
    expect(greetingForHour(18)).toBe("晚上好");
    expect(greetingForHour(23)).toBe("晚上好");
  });
});

describe("formatDuration", () => {
  it("formats seconds, minutes and hours", () => {
    expect(formatDuration(0)).toBe("0秒");
    expect(formatDuration(45)).toBe("45秒");
    expect(formatDuration(60)).toBe("1分0秒");
    expect(formatDuration(125)).toBe("2分5秒");
    expect(formatDuration(3600)).toBe("1小时0分");
    expect(formatDuration(5430)).toBe("1小时30分");
  });

  it("clamps negative input", () => {
    expect(formatDuration(-10)).toBe("0秒");
  });
});
