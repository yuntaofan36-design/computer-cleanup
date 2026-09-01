import { useMemo, useState } from 'react';
import { createPortal } from 'react-dom';
import {
  Database,
  ExternalLink,
  Package,
  RefreshCw,
  Search,
  ShieldCheck,
  X,
} from 'lucide-react';
import { formatBytes } from '../format';
import type { AppEntry } from '../types';
import { ApplicationIcon } from './ApplicationIcon';

export interface AppManagementProps {
  apps: AppEntry[];
  onRequestUninstall: (app: AppEntry) => void;
  onClearCache: (app: AppEntry) => void;
  busyAppId?: string | null;
}

type PendingAction = {
  kind: 'uninstall' | 'cache';
  app: AppEntry;
};

interface ConfirmationDialogProps {
  action: PendingAction;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

function ConfirmationDialog({
  action,
  busy,
  onCancel,
  onConfirm,
}: ConfirmationDialogProps): JSX.Element {
  const isUninstall = action.kind === 'uninstall';

  return createPortal(
    <div className="overlay" role="presentation" onMouseDown={onCancel}>
      <div
        className="dialog confirmation-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="app-action-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button
          type="button"
          className="icon-button dialog-close"
          onClick={onCancel}
          aria-label="关闭"
          disabled={busy}
        >
          <X size={18} />
        </button>
        <span className={`dialog-icon ${isUninstall ? 'warning' : 'safe'}`} aria-hidden="true">
          {isUninstall ? <ExternalLink /> : <RefreshCw />}
        </span>
        <h2 id="app-action-title">
          {isUninstall
            ? `调用 ${action.app.name} 的官方卸载器？`
            : `清理 ${action.app.name} 的可重建缓存？`}
        </h2>
        <p>
          {isUninstall
            ? '系统将打开此应用注册的官方卸载程序，由它完成卸载。Lumina Clean 不会直接删除安装目录，也不会碰触用户文档。'
            : `仅处理扫描结果中已确认可重建的 ${formatBytes(action.app.cacheBytes)} 缓存；不会删除应用配置、登录信息或用户文件。`}
        </p>
        <div className="dialog-actions">
          <button type="button" className="button secondary" onClick={onCancel} disabled={busy}>
            取消
          </button>
          <button
            type="button"
            className={`button ${isUninstall ? 'danger' : 'primary'}`}
            onClick={onConfirm}
            disabled={busy}
          >
            {busy ? '正在提交…' : isUninstall ? '打开官方卸载器' : '清理可重建缓存'}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}

export default function AppManagement({
  apps,
  onRequestUninstall,
  onClearCache,
  busyAppId = null,
}: AppManagementProps): JSX.Element {
  const [query, setQuery] = useState('');
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);

  const normalizedQuery = query.trim().toLocaleLowerCase();
  const visibleApps = useMemo(() => {
    if (!normalizedQuery) return apps;
    return apps.filter((app) =>
      [app.name, app.publisher, app.version]
        .join(' ')
        .toLocaleLowerCase()
        .includes(normalizedQuery),
    );
  }, [apps, normalizedQuery]);

  const totalAppBytes = useMemo(
    () => apps.reduce((total, app) => total + Math.max(0, app.sizeBytes), 0),
    [apps],
  );
  const totalCacheBytes = useMemo(
    () => apps.reduce((total, app) => total + Math.max(0, app.cacheBytes), 0),
    [apps],
  );
  const cacheCandidateCount = useMemo(
    () => apps.filter((app) => app.cacheBytes > 0).length,
    [apps],
  );

  const confirmAction = (): void => {
    if (!pendingAction) return;
    if (pendingAction.kind === 'uninstall') {
      onRequestUninstall(pendingAction.app);
    } else {
      onClearCache(pendingAction.app);
    }
    setPendingAction(null);
  };

  return (
    <section className="page-section management-page">
      <header className="page-head">
        <div className="page-title-block">
          <h1>应用管理</h1>
          <p>通过官方卸载入口移除应用，只清理由扫描器确认可重建的缓存。</p>
        </div>
        <label className="searchbox" htmlFor="app-search">
          <Search aria-hidden="true" />
          <input
            id="app-search"
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="搜索名称、发布者或版本"
          />
        </label>
      </header>

      <div className="summary-grid management-summary" aria-label="应用空间概览">
        <article className="summary-card">
          <span className="summary-icon" aria-hidden="true"><Package /></span>
          <div>
            <small>已安装应用</small>
            <strong>{apps.length} 个</strong>
            <span>合计 {formatBytes(totalAppBytes)}</span>
          </div>
        </article>
        <article className="summary-card safe">
          <span className="summary-icon" aria-hidden="true"><Database /></span>
          <div>
            <small>可重建缓存</small>
            <strong>{formatBytes(totalCacheBytes)}</strong>
            <span>{cacheCandidateCount} 个应用可按需清理</span>
          </div>
        </article>
        <article className="summary-card policy">
          <span className="summary-icon" aria-hidden="true"><ShieldCheck /></span>
          <div>
            <small>安全边界</small>
            <strong>不直接删目录</strong>
            <span>卸载始终交给应用自己的卸载器</span>
          </div>
        </article>
      </div>

      <div className="content-panel management-panel">
        <div className="panel-head">
          <div>
            <h2>已安装应用</h2>
            <p>{normalizedQuery ? `找到 ${visibleApps.length} 个匹配项` : '按应用逐项确认操作'}</p>
          </div>
        </div>

        {visibleApps.length === 0 ? (
          <div className="empty-state">
            <Search aria-hidden="true" />
            <h3>{apps.length === 0 ? '尚未获取应用清单' : '没有匹配的应用'}</h3>
            <p>{apps.length === 0 ? '完成应用扫描后，结果会显示在这里。' : '请尝试名称、发布者或版本中的其他关键词。'}</p>
          </div>
        ) : (
          <div className="table app-management-table">
            <div className="table-head" role="row">
              <span>应用</span>
              <span>上次使用</span>
              <span>应用大小</span>
              <span>可重建缓存</span>
              <span>安全操作</span>
            </div>
            {visibleApps.map((app) => {
              const isBusy = busyAppId === app.id;
              return (
                <div className="table-row" role="row" key={app.id}>
                  <span className="app-name">
                    <ApplicationIcon appId={app.id} name={app.name} />
                    <span>
                      <strong>{app.name}</strong>
                      <small>{app.publisher} · {app.version} · 安装于 {app.installedAt || '未知日期'}</small>
                    </span>
                  </span>
                  <span className="table-value">{app.lastUsed || '未记录'}</span>
                  <span className="table-value"><strong>{formatBytes(app.sizeBytes)}</strong></span>
                  <span className="table-value cache-value">
                    <strong>{formatBytes(app.cacheBytes)}</strong>
                    <small>{app.cacheBytes > 0 ? '可重新生成' : '无候选项'}</small>
                  </span>
                  <span className="row-actions">
                    {app.cacheBytes > 0 && (
                      <button
                        type="button"
                        className="button secondary small"
                        onClick={() => setPendingAction({ kind: 'cache', app })}
                        disabled={isBusy}
                      >
                        <RefreshCw size={15} />
                        清理可重建缓存
                      </button>
                    )}
                    <button
                      type="button"
                      className="button secondary small uninstall-button"
                      onClick={() => setPendingAction({ kind: 'uninstall', app })}
                      disabled={isBusy || app.uninstallable === false}
                      title={app.uninstallable === false ? '此应用没有注册可用的卸载器' : undefined}
                    >
                      <ExternalLink size={15} />
                      调用官方卸载器
                    </button>
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {pendingAction && (
        <ConfirmationDialog
          action={pendingAction}
          busy={busyAppId === pendingAction.app.id}
          onCancel={() => setPendingAction(null)}
          onConfirm={confirmAction}
        />
      )}
    </section>
  );
}
