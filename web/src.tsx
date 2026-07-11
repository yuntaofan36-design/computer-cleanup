import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  Activity,
  Copy,
  KeyRound,
  LayoutDashboard,
  Plus,
  Search,
  Settings,
  ShieldCheck,
  Users,
  History,
  X,
} from "lucide-react";
import "./style.css";
import { api } from "./licenseApi";
import { Login } from "./Login";
type License = {
  id: string;
  prefix: string;
  plan: string;
  status: "active" | "unused" | "revoked" | "expired";
  expiresAt: string;
  device: string;
  createdAt: string;
};
const seed: License[] = [
  {
    id: "1",
    prefix: "QP-9K4M",
    plan: "专业版 · 1年",
    status: "active",
    expiresAt: "2027-07-01",
    device: "DESKTOP-7A2F",
    createdAt: "2026-07-01",
  },
  {
    id: "2",
    prefix: "QP-H8T2",
    plan: "专业版 · 永久",
    status: "unused",
    expiresAt: "永久",
    device: "—",
    createdAt: "2026-07-09",
  },
  {
    id: "3",
    prefix: "QP-3M7Q",
    plan: "专业版 · 1年",
    status: "revoked",
    expiresAt: "2026-12-21",
    device: "LAPTOP-LIN",
    createdAt: "2025-12-21",
  },
  {
    id: "4",
    prefix: "QP-W2B8",
    plan: "专业版 · 30天",
    status: "active",
    expiresAt: "2026-08-04",
    device: "OFFICE-PC",
    createdAt: "2026-07-05",
  },
];
function Modal({
  close,
  create,
}: {
  close: () => void;
  create: (count: number, days: number) => void;
}) {
  const [count, setCount] = useState(10),
    [days, setDays] = useState(365);
  return (
    <div className="overlay">
      <div className="modal">
        <button className="icon close" onClick={close}>
          <X />
        </button>
        <h2>生成卡密</h2>
        <p>完整卡密仅在生成后显示一次，请妥善保存。</p>
        <label>
          生成数量
          <input
            type="number"
            min="1"
            max="100"
            value={count}
            onChange={(e) => setCount(+e.target.value)}
          />
        </label>
        <label>
          有效期
          <select value={days} onChange={(e) => setDays(+e.target.value)}>
            <option value="30">30 天</option>
            <option value="365">1 年</option>
            <option value="0">永久</option>
          </select>
        </label>
        <div className="actions">
          <button onClick={close}>取消</button>
          <button className="primary" onClick={() => create(count, days)}>
            生成卡密
          </button>
        </div>
      </div>
    </div>
  );
}
function App() {
  const [page, setPage] = useState("dashboard"),
    [licenses, setLicenses] = useState(seed),
    [modal, setModal] = useState(false),
    [generated, setGenerated] = useState<string[]>([]),
    [query, setQuery] = useState(""),
    [token, setToken] = useState(() => localStorage.getItem("qingpanAdminToken") || "");
  const shown = useMemo(
    () =>
      licenses.filter((x) =>
        (x.prefix + x.device).toLowerCase().includes(query.toLowerCase()),
      ),
    [licenses, query],
  );
  useEffect(() => {
    if (!token) return;
    api.licenses(token).then((rows: any[]) => setLicenses(rows.map((x: any) => ({
      id: x.id, prefix: x.prefix, plan: x.plan === "pro" ? "专业版" : x.plan,
      status: x.status, expiresAt: x.expiresAt || "激活后计算",
      device: x.deviceCount ? `${x.deviceCount} 台设备` : "—", createdAt: x.createdAt,
    })))).catch(() => { localStorage.removeItem("qingpanAdminToken"); setToken(""); });
  }, [token]);
  if (!token) return <Login onSuccess={setToken} />;
  const create = async (count: number, days: number) => {
    const { keys } = await api.generate(token, count, days);
    setGenerated(keys);
    setLicenses((v) => [
      ...keys.map((k, i) => ({
        id: crypto.randomUUID(),
        prefix: k.slice(0, 7),
        plan: `专业版 · ${days ? days + "天" : "永久"}`,
        status: "unused" as const,
        expiresAt: days ? "激活后计算" : "永久",
        device: "—",
        createdAt: "今天",
      })),
      ...v,
    ]);
    setModal(false);
  };
  return (
    <div className="shell">
      <aside>
        <div className="brand">
          <span>
            <KeyRound />
          </span>
          <div>
            <b>清盘控制台</b>
            <small>License Center</small>
          </div>
        </div>
        <nav>
          <button
            className={page === "dashboard" ? "active" : ""}
            onClick={() => setPage("dashboard")}
          >
            <LayoutDashboard />
            数据概览
          </button>
          <button
            className={page === "licenses" ? "active" : ""}
            onClick={() => setPage("licenses")}
          >
            <KeyRound />
            卡密管理
          </button>
          <button>
            <Users />
            设备管理
          </button>
          <button>
            <History />
            审计日志
          </button>
        </nav>
        <nav className="bottom">
          <button>
            <Settings />
            系统设置
          </button>
        </nav>
      </aside>
      <main>
        <header>
          <div>
            <h1>{page === "dashboard" ? "数据概览" : "卡密管理"}</h1>
            <p>清盘产品授权与设备状态</p>
          </div>
          <span className="admin">
            <i>管</i>
            <span>
              <b>管理员</b>
              <small>admin@qingpan.local</small>
            </span>
          </span>
        </header>
        {page === "dashboard" ? (
          <>
            <div className="stats">
              {[
                ["已生成卡密", "1,284", "+84 本月", KeyRound],
                ["有效授权", "936", "72.9%", ShieldCheck],
                ["激活设备", "891", "+17 本周", Users],
                ["今日校验", "2,648", "99.8% 成功", Activity],
              ].map((x: any) => (
                <div>
                  <span>
                    <small>{x[0]}</small>
                    <b>{x[1]}</b>
                    <em>{x[2]}</em>
                  </span>
                  {React.createElement(x[3])}
                </div>
              ))}
            </div>
            <div className="dashboard-grid">
              <section>
                <div className="title">
                  <h2>最近授权</h2>
                  <button onClick={() => setPage("licenses")}>查看全部</button>
                </div>
                <LicenseTable rows={licenses.slice(0, 4)} />
              </section>
              <section className="health">
                <div className="title">
                  <h2>服务状态</h2>
                </div>
                <div className="health-row">
                  <i />
                  <span>
                    <b>授权 API</b>
                    <small>运行正常</small>
                  </span>
                  <strong>24 ms</strong>
                </div>
                <div className="health-row">
                  <i />
                  <span>
                    <b>数据库</b>
                    <small>运行正常</small>
                  </span>
                  <strong>8 ms</strong>
                </div>
                <div className="health-row">
                  <i />
                  <span>
                    <b>过去 24 小时</b>
                    <small>可用率</small>
                  </span>
                  <strong>99.99%</strong>
                </div>
              </section>
            </div>
          </>
        ) : (
          <>
            <div className="toolbar">
              <div className="search">
                <Search />
                <input
                  placeholder="搜索卡密前缀或设备"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
              </div>
              <button className="primary" onClick={() => setModal(true)}>
                <Plus />
                生成卡密
              </button>
            </div>
            {generated.length > 0 && (
              <div className="generated">
                <div>
                  <ShieldCheck />
                  <span>
                    <b>卡密生成成功</b>
                    <small>完整卡密仅显示一次，共 {generated.length} 个</small>
                  </span>
                </div>
                <pre>{generated.join("\n")}</pre>
                <button
                  onClick={() =>
                    navigator.clipboard.writeText(generated.join("\n"))
                  }
                >
                  <Copy />
                  复制全部
                </button>
              </div>
            )}
            <section>
              <LicenseTable rows={shown} />
            </section>
          </>
        )}
        {modal && <Modal close={() => setModal(false)} create={create} />}
      </main>
    </div>
  );
}
function LicenseTable({ rows }: { rows: License[] }) {
  return (
    <div className="table">
      <div className="tr th">
        <span>卡密</span>
        <span>授权方案</span>
        <span>状态</span>
        <span>绑定设备</span>
        <span>到期时间</span>
      </div>
      {rows.map((x) => (
        <div className="tr">
          <code>{x.prefix}-••••-••••</code>
          <span>{x.plan}</span>
          <span>
            <i className={`status ${x.status}`} />
            {
              (
                {
                  active: "已激活",
                  unused: "未使用",
                  revoked: "已撤销",
                  expired: "已过期",
                } as any
              )[x.status]
            }
          </span>
          <span>{x.device}</span>
          <span>{x.expiresAt}</span>
        </div>
      ))}
    </div>
  );
}
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
