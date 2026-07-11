const DEFAULT_ORIGINS = [
  "http://127.0.0.1:1420",
  "http://localhost:1420",
  "http://127.0.0.1:3000",
  "http://localhost:3000",
  "tauri://localhost",
  "http://tauri.localhost",
  "https://tauri.localhost",
];

export function allowedOrigins(configured?: string): Set<string> {
  const extra = (configured || "")
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
  return new Set([...DEFAULT_ORIGINS, ...extra]);
}

export function isOriginAllowed(
  origin: string | undefined,
  origins: Set<string>,
): boolean {
  // Requests without an Origin header are native/server-to-server calls.
  return !origin || origins.has(origin);
}
