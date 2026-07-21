import express from "express";
import cors from "cors";
import helmet from "helmet";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { openDb } from "./db.js";
import { allowedOrigins, isOriginAllowed } from "./cors.js";
import {
  hashDevice,
  hashLicense,
  id,
  newLicense,
  parseToken,
  token,
  verifyPassword,
} from "./security.js";
const PORT = Number(process.env.PORT || 8787),
  HOST = process.env.HOST || "127.0.0.1",
  SECRET = process.env.SERVER_SECRET || "development-only-change-me",
  PEPPER = process.env.LICENSE_PEPPER || SECRET,
  ADMIN_PASSWORD = process.env.ADMIN_PASSWORD || "change-me-now";
const db = openDb(process.env.DB_PATH || "./data/qingpan.db", ADMIN_PASSWORD),
  app = express(),
  origins = allowedOrigins(
    process.env.ALLOWED_ORIGINS || process.env.WEB_ORIGIN,
  ),
  webDist = resolve(process.env.WEB_DIST || "./public"),
  releasesDir = resolve(process.env.RELEASES_DIR || "./releases");
app.use(helmet());
app.use(
  cors({
    origin: (origin, callback) =>
      callback(
        isOriginAllowed(origin, origins)
          ? null
          : new Error("Origin 不在 CORS 白名单中"),
        isOriginAllowed(origin, origins),
      ),
  }),
);
app.use(express.json({ limit: "32kb" }));
const attempts = new Map<string, { count: number; reset: number }>();
app.use("/api/license", (req, res, next) => {
  const key = req.ip || "unknown",
    now = Date.now(),
    v = attempts.get(key);
  if (!v || v.reset < now) attempts.set(key, { count: 1, reset: now + 60_000 });
  else if (++v.count > 30) {
    res.status(429).json({ error: "请求过于频繁" });
    return;
  }
  next();
});
const requireAdmin: express.RequestHandler = (req, res, next) => {
  const p = parseToken(req.headers.authorization, SECRET);
  if (!p || p.role !== "admin") {
    res.status(401).json({ error: "未授权" });
    return;
  }
  (req as any).admin = p;
  next();
};
const audit = (
  actor: string,
  action: string,
  target: string,
  detail: string,
  ip?: string,
) =>
  db
    .prepare("INSERT INTO audit_logs VALUES(?,?,?,?,?,?,?)")
    .run(
      id(),
      actor,
      action,
      target,
      detail,
      ip || "",
      new Date().toISOString(),
    );
app.get("/api/health", (_req, res) =>
  res.json({ ok: true, time: new Date().toISOString() }),
);
app.post("/api/admin/login", (req, res) => {
  const row = db
    .prepare("SELECT * FROM admins WHERE email=?")
    .get(req.body.email) as any;
  if (
    !row ||
    !verifyPassword(String(req.body.password || ""), row.password_hash)
  ) {
    res.status(401).json({ error: "邮箱或密码错误" });
    return;
  }
  res.json({ token: token({ sub: row.id, role: "admin" }, SECRET, 8 * 3600) });
});
app.get("/api/admin/licenses", requireAdmin, (_req, res) => {
  const rows = db
    .prepare(
      `SELECT l.id,l.prefix,l.plan,l.duration_days AS durationDays,l.status,l.created_at AS createdAt,l.expires_at AS expiresAt,COUNT(a.id) AS deviceCount FROM licenses l LEFT JOIN activations a ON a.license_id=l.id AND a.revoked_at IS NULL GROUP BY l.id ORDER BY l.created_at DESC`,
    )
    .all();
  res.json(rows);
});
app.post("/api/admin/licenses", requireAdmin, (req, res) => {
  const count = Math.min(100, Math.max(1, Number(req.body.count || 1))),
    days = req.body.durationDays == null ? 365 : Number(req.body.durationDays),
    plan = String(req.body.plan || "pro"),
    keys: string[] = [];
  const insert = db.prepare(
    "INSERT INTO licenses(id,key_hash,prefix,plan,duration_days,status,max_devices,created_at) VALUES(?,?,?,?,?,?,?,?)",
  );
  for (let i = 0; i < count; i++) {
    const key = newLicense();
    insert.run(
      id(),
      hashLicense(key, PEPPER),
      key.slice(0, 7),
      plan,
      days || null,
      "unused",
      Math.min(10, Math.max(1, Number(req.body.maxDevices || 1))),
      new Date().toISOString(),
    );
    keys.push(key);
  }
  audit(
    (req as any).admin.sub,
    "license.create",
    "batch",
    JSON.stringify({ count, days }),
    req.ip,
  );
  res.status(201).json({ keys });
});
app.patch("/api/admin/licenses/:id", requireAdmin, (req, res) => {
  const status = String(req.body.status),
    licenseId = String(req.params.id);
  if (!["unused", "active", "revoked"].includes(status)) {
    res.status(400).json({ error: "无效状态" });
    return;
  }
  db.prepare("UPDATE licenses SET status=? WHERE id=?").run(status, licenseId);
  audit((req as any).admin.sub, "license.status", licenseId, status, req.ip);
  res.json({ ok: true });
});
app.delete(
  "/api/admin/licenses/:id/devices/:activationId",
  requireAdmin,
  (req, res) => {
    const licenseId = String(req.params.id),
      activationId = String(req.params.activationId);
    db.prepare(
      "UPDATE activations SET revoked_at=? WHERE id=? AND license_id=?",
    ).run(new Date().toISOString(), activationId, licenseId);
    audit((req as any).admin.sub, "device.revoke", activationId, "", req.ip);
    res.json({ ok: true });
  },
);
app.get("/api/admin/audit", requireAdmin, (_req, res) =>
  res.json(
    db
      .prepare("SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT 200")
      .all(),
  ),
);
app.post("/api/license/activate", (req, res) => {
  const key = String(req.body.key || ""),
    deviceId = String(req.body.deviceId || ""),
    deviceName = String(req.body.deviceName || "Windows 设备").slice(0, 80);
  if (key.length < 10 || deviceId.length < 8) {
    res.status(400).json({ error: "卡密或设备信息无效" });
    return;
  }
  const license = db
    .prepare("SELECT * FROM licenses WHERE key_hash=?")
    .get(hashLicense(key, PEPPER)) as any;
  if (!license || license.status === "revoked") {
    res.status(403).json({ error: "卡密无效或已撤销" });
    return;
  }
  if (license.expires_at && new Date(license.expires_at) < new Date()) {
    db.prepare("UPDATE licenses SET status='expired' WHERE id=?").run(
      license.id,
    );
    res.status(403).json({ error: "授权已过期" });
    return;
  }
  const dh = hashDevice(deviceId, PEPPER),
    existing = db
      .prepare(
        "SELECT * FROM activations WHERE license_id=? AND device_hash=? AND revoked_at IS NULL",
      )
      .get(license.id, dh) as any;
  const count = (
    db
      .prepare(
        "SELECT COUNT(*) n FROM activations WHERE license_id=? AND revoked_at IS NULL",
      )
      .get(license.id) as any
  ).n;
  if (!existing && count >= license.max_devices) {
    res.status(409).json({ error: "已达到设备数量上限" });
    return;
  }
  const now = new Date().toISOString();
  let activationId = existing?.id;
  if (!existing) {
    activationId = id();
    db.prepare("INSERT INTO activations VALUES(?,?,?,?,?,?,NULL)").run(
      activationId,
      license.id,
      dh,
      deviceName,
      now,
      now,
    );
  } else
    db.prepare("UPDATE activations SET last_seen_at=? WHERE id=?").run(
      now,
      existing.id,
    );
  let expires = license.expires_at;
  if (!expires && license.duration_days) {
    expires = new Date(
      Date.now() + license.duration_days * 86400000,
    ).toISOString();
    db.prepare("UPDATE licenses SET expires_at=? WHERE id=?").run(
      expires,
      license.id,
    );
  }
  db.prepare("UPDATE licenses SET status='active' WHERE id=?").run(license.id);
  res.json({
    token: token(
      {
        sub: activationId,
        licenseId: license.id,
        deviceHash: dh,
        role: "device",
      },
      SECRET,
      7 * 86400,
    ),
    plan: license.plan,
    expiresAt: expires,
  });
});
app.post("/api/license/validate", (req, res) => {
  const p = parseToken(req.headers.authorization, SECRET);
  if (!p || p.role !== "device") {
    res.status(401).json({ valid: false, error: "授权会话无效" });
    return;
  }
  const row = db
    .prepare(
      `SELECT l.status,l.expires_at,a.revoked_at FROM activations a JOIN licenses l ON l.id=a.license_id WHERE a.id=?`,
    )
    .get(p.sub) as any;
  const valid =
    !!row &&
    !row.revoked_at &&
    row.status === "active" &&
    (!row.expires_at || new Date(row.expires_at) > new Date());
  if (valid)
    db.prepare("UPDATE activations SET last_seen_at=? WHERE id=?").run(
      new Date().toISOString(),
      p.sub,
    );
  res.status(valid ? 200 : 403).json({ valid, expiresAt: row?.expires_at });
});
// The web dashboard and signed desktop-update artifacts share the same origin
// as the API in production.  Keeping releases outside the database makes
// image upgrades stateless and allows the data volume to be backed up alone.
app.use(
  "/releases",
  (req, res, next) => {
    if (req.path.endsWith("latest.json")) {
      res.setHeader("Cache-Control", "no-store, max-age=0");
    } else {
      res.setHeader("Cache-Control", "public, max-age=31536000, immutable");
    }
    next();
  },
  express.static(releasesDir, { index: false }),
);
if (existsSync(webDist)) {
  app.use(express.static(webDist, { index: "index.html", maxAge: "1h" }));
  app.get(/^(?!\/api(?:\/|$)|\/releases(?:\/|$)).*/, (_req, res) =>
    res.sendFile(resolve(webDist, "index.html")),
  );
}
app.use((_req, res) => res.status(404).json({ error: "接口不存在" }));
app.listen(PORT, HOST, () =>
  console.log(`Qingpan server: http://${HOST}:${PORT}`),
);
