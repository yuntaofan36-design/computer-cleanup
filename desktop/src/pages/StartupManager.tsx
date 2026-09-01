import { useMemo, useState } from 'react';
import {
  AlertTriangle,
  Gauge,
  LoaderCircle,
  Power,
  PowerOff,
  RefreshCw,
  Rocket,
  Search,
} from 'lucide-react';
import { EmptyState } from '../components';
import type { StartupEntry } from '../types';

export interface StartupManagerProps {
  entries: StartupEntry[];
  busyId?: string | null;
  error?: string | null;
  onToggle: (id: string, enabled: boolean) => void | Promise<void>;
  onRefresh: () => void | Promise<void>;
}

const impactTone: Record<StartupEntry['impact'], string> = {
  低: 'status-success',
  中: 'status-partial',
  高: 'status-failed',
  未知: '',
};

function messageFrom(error: unknown, fallback: string): string {
  if (typeof error === 'string' && error.trim()) return error;
  return error instanceof Error && error.message ? error.message : fallback;
}

export default function StartupManager({
  entries,
  busyId = null,
  error = null,
  onToggle,
  onRefresh,
}: StartupManagerProps): JSX.Element {
  const [query, setQuery] = useState('');
  const [refreshing, setRefreshing] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const normalizedQuery = query.trim().toLocaleLowerCase();
  const visibleEntries = useMemo(() => {
    if (!normalizedQuery) return entries;
    return entries.filter((entry) => (
      [entry.name, entry.publisher, entry.command, entry.scope, entry.impact]
        .join(' ')
        .toLocaleLowerCase()
        .includes(normalizedQuery)
    ));
  }, [entries, normalizedQuery]);

  const enabledCount = useMemo(
    () => entries.filter((entry) => entry.enabled).length,
    [entries],
  );
  const highImpactCount = useMemo(
    () => entries.filter((entry) => entry.impact === '高').length,
    [entries],
  );
  const visibleError = error || actionError;

  const refresh = async (): Promise<void> => {
    if (refreshing || busyId) return;
    setActionError(null);
    setRefreshing(true);
    try {
      await onRefresh();
    } catch (refreshError) {
      setActionError(messageFrom(refreshError, '无法刷新启动项，请稍后重试。'));
    } finally {
      setRefreshing(false);
    }
  };

  const toggle = async (entry: StartupEntry): Promise<void> => {
    if (refreshing || busyId) return;
    setActionError(null);
    try {
      await onToggle(entry.id, !entry.enabled);
    } catch (toggleError) {
      setActionError(messageFrom(toggleError, `无法更新 ${entry.name} 的启动状态。`));
    }
  };

  return (
    <section className="page-section management-page startup-manager-page">
      <header className="page-head">
        <div className="page-title-block">
          <p className="eyebrow">系统工具</p>
          <h1>启动项管理</h1>
          <p>查看由 Windows 注册的登录启动程序及当前状态。</p>
        </div>
        <div className="page-actions">
          <label className="searchbox" htmlFor="startup-search">
            <Search aria-hidden="true" />
            <input
              id="startup-search"
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="搜索名称、发布者或命令"
            />
          </label>
          <button
            type="button"
            className="button secondary"
            onClick={() => void refresh()}
            disabled={refreshing || Boolean(busyId)}
          >
            <RefreshCw className={refreshing ? 'spin' : ''} aria-hidden="true" />
            {refreshing ? '刷新中' : '刷新'}
          </button>
        </div>
      </header>

      {visibleError && (
        <div className="partition-inline-error" role="alert">
          <AlertTriangle aria-hidden="true" />
          <span>{visibleError}</span>
        </div>
      )}

      <div className="summary-grid management-summary" aria-label="启动项概览">
        <article className="summary-card">
          <span className="summary-icon" aria-hidden="true"><Rocket /></span>
          <div><small>已登记</small><strong>{entries.length} 项</strong><span>当前用户登录启动程序</span></div>
        </article>
        <article className="summary-card safe">
          <span className="summary-icon" aria-hidden="true"><Power /></span>
          <div><small>已启用</small><strong>{enabledCount} 项</strong><span>将在下次登录时启动</span></div>
        </article>
        <article className="summary-card warning">
          <span className="summary-icon" aria-hidden="true"><Gauge /></span>
          <div><small>高影响</small><strong>{highImpactCount} 项</strong><span>建议结合实际使用频率判断</span></div>
        </article>
      </div>

      <div className="content-panel management-panel">
        <div className="panel-head">
          <div>
            <h2>登录启动程序</h2>
            <p>{normalizedQuery ? `找到 ${visibleEntries.length} 个匹配项` : `${entries.length} 个启动项`}</p>
          </div>
        </div>

        {visibleEntries.length === 0 ? (
          <EmptyState
            icon={entries.length === 0 ? <PowerOff /> : <Search />}
            title={entries.length === 0 ? '没有已登记的启动项' : '没有匹配的启动项'}
            description={entries.length === 0 ? 'Windows 当前用户启动列表为空。' : '请尝试名称、发布者、命令或作用范围中的其他关键词。'}
            action={entries.length === 0 ? (
              <button type="button" className="button secondary" onClick={() => void refresh()} disabled={refreshing}>
                <RefreshCw className={refreshing ? 'spin' : ''} aria-hidden="true" />
                重新读取
              </button>
            ) : undefined}
          />
        ) : (
          <div className="list operation-list" aria-busy={Boolean(busyId)}>
            {visibleEntries.map((entry) => {
              const isBusy = busyId === entry.id;
              const nextState = !entry.enabled;
              return (
                <div className="list-row" key={entry.id}>
                  <span className={`row-icon ${entry.enabled ? 'status-success' : ''}`} aria-hidden="true">
                    {isBusy ? <LoaderCircle className="spin" /> : entry.enabled ? <Power /> : <PowerOff />}
                  </span>
                  <span className="grow operation-description">
                    <strong>{entry.name}<span className="badge">{entry.scope}</span></strong>
                    <small title={entry.command}>
                      {entry.publisher || '发布者未知'} · {entry.command || '未提供启动命令'}
                    </small>
                  </span>
                  <span className={`badge ${impactTone[entry.impact]}`}>{entry.impact}影响</span>
                  <span className={`badge ${entry.enabled ? 'status-success' : ''}`}>
                    {entry.enabled ? '已启用' : '已禁用'}
                  </span>
                  <span className="setting-control">
                    <span className="setting-state">
                      {isBusy ? '正在更新' : entry.enabled ? '开启' : '关闭'}
                    </span>
                    <button
                      type="button"
                      role="switch"
                      aria-label={`${nextState ? '启用' : '禁用'} ${entry.name}`}
                      aria-checked={entry.enabled}
                      className={`switch ${entry.enabled ? 'on' : ''}`}
                      onClick={() => void toggle(entry)}
                      disabled={refreshing || Boolean(busyId)}
                    >
                      <span />
                    </button>
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </section>
  );
}
