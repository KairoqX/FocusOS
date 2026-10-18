import { describe, expect, it } from "vitest";
import { formatClock, formatFocusTime } from "./format";

describe("formatClock", () => {
  it("formats minutes and seconds", () => {
    expect(formatClock(25 * 60_000)).toBe("25:00");
    expect(formatClock(0)).toBe("00:00");
  });
  it("never goes negative", () => expect(formatClock(-500)).toBe("00:00"));
  it("rounds up so 00:00 only appears when time is really up", () => {
    expect(formatClock(1)).toBe("00:01");
    expect(formatClock(59_001)).toBe("01:00");
  });
  it("switches to h:mm:ss from one hour", () => {
    expect(formatClock(3_600_000)).toBe("1:00:00");
    expect(formatClock(3_661_000)).toBe("1:01:01");
  });
});

describe("formatFocusTime", () => {
  it("handles tiny, minute and hour values", () => {
    expect(formatFocusTime(0)).toBe("0 min");
    expect(formatFocusTime(20_000)).toBe("<1 min");
    expect(formatFocusTime(25 * 60_000)).toBe("25 min");
    expect(formatFocusTime(85 * 60_000)).toBe("1 h 25 min");
    expect(formatFocusTime(120 * 60_000)).toBe("2 h");
  });
});
