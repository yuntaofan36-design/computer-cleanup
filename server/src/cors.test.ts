import { describe, expect, it } from "vitest";
import { allowedOrigins, isOriginAllowed } from "./cors.js";

describe("CORS allowlist", () => {
  const origins = allowedOrigins(
    "https://licenses.example.com, https://admin.example.com",
  );

  it("allows desktop, web and Tauri application origins", () => {
    expect(isOriginAllowed("http://127.0.0.1:1420", origins)).toBe(true);
    expect(isOriginAllowed("http://127.0.0.1:3000", origins)).toBe(true);
    expect(isOriginAllowed("tauri://localhost", origins)).toBe(true);
    expect(isOriginAllowed("http://tauri.localhost", origins)).toBe(true);
  });

  it("allows configured production origins and native requests", () => {
    expect(isOriginAllowed("https://admin.example.com", origins)).toBe(true);
    expect(isOriginAllowed(undefined, origins)).toBe(true);
  });

  it("rejects unknown browser origins", () => {
    expect(isOriginAllowed("https://untrusted.example", origins)).toBe(false);
  });
});
