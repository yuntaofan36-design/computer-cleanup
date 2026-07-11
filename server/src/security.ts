import {
  createHash,
  createHmac,
  randomBytes,
  scryptSync,
  timingSafeEqual,
} from "node:crypto";
export const hashLicense = (key: string, pepper: string) =>
  createHmac("sha256", pepper)
    .update(key.replace(/-/g, "").toUpperCase())
    .digest("hex");
export const hashDevice = (id: string, pepper: string) =>
  createHmac("sha256", pepper).update(id).digest("hex");
export function hashPassword(password: string) {
  const salt = randomBytes(16).toString("hex");
  return `${salt}:${scryptSync(password, salt, 32).toString("hex")}`;
}
export function verifyPassword(password: string, stored: string) {
  const [salt, hash] = stored.split(":");
  if (!salt || !hash) return false;
  const a = Buffer.from(hash, "hex"),
    b = scryptSync(password, salt, 32);
  return a.length === b.length && timingSafeEqual(a, b);
}
export function token(payload: object, secret: string, ttlSeconds: number) {
  const body = Buffer.from(
    JSON.stringify({
      ...payload,
      exp: Math.floor(Date.now() / 1000) + ttlSeconds,
    }),
  ).toString("base64url");
  return `${body}.${createHmac("sha256", secret).update(body).digest("base64url")}`;
}
export function parseToken(raw: string | undefined, secret: string) {
  if (!raw) return null;
  const [b, s] = raw.replace(/^Bearer /, "").split(".");
  if (!b || !s) return null;
  const expected = createHmac("sha256", secret).update(b).digest("base64url");
  if (
    s.length !== expected.length ||
    !timingSafeEqual(Buffer.from(s), Buffer.from(expected))
  )
    return null;
  try {
    const p = JSON.parse(Buffer.from(b, "base64url").toString());
    return p.exp > Date.now() / 1000 ? p : null;
  } catch {
    return null;
  }
}
export function newLicense() {
  const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  const part = () =>
    Array.from(
      { length: 4 },
      () => chars[Math.floor(Math.random() * chars.length)],
    ).join("");
  return `QP-${part()}-${part()}-${part()}-${part()}`;
}
export const id = () => randomBytes(16).toString("hex");
export const sha256 = (v: string) =>
  createHash("sha256").update(v).digest("hex");
