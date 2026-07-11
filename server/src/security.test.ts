import { describe, expect, it } from "vitest";
import { hashLicense, newLicense, parseToken, token } from "./security.js";
describe("license security", () => {
  it("generates readable non-ambiguous keys", () =>
    expect(newLicense()).toMatch(/^QP-[A-Z2-9]{4}(-[A-Z2-9]{4}){3}$/));
  it("normalizes separators", () =>
    expect(hashLicense("QP-ABCD-EFGH", "x")).toBe(
      hashLicense("qpabcdefgh", "x"),
    ));
  it("signs and verifies sessions", () =>
    expect(parseToken(token({ role: "device" }, "s", 60), "s")?.role).toBe(
      "device",
    ));
  it("rejects tampering", () =>
    expect(
      parseToken(token({ role: "device" }, "s", 60) + "x", "s"),
    ).toBeNull());
});
