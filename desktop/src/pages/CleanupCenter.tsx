import { useEffect, useMemo, useState } from 'react';
import {
  AppWindow, Check, ChevronDown, CircleSlash2, FileStack, Globe2, HardDrive,
  Info, MessageCircle, RefreshCw, ScanSearch, ShieldCheck, Sparkles, Trash2, TriangleAlert,
} from 'lucide-react';
import { DimensionTags, EmptyState, SafetyNotice } from '../components';
import { formatBytes } from '../format';
import type { CleanupItem, CleanupScope, DiskInfo } from '../types';

type ScopeFilter = 'all' | CleanupScope;

interface CleanupCenterProps {
  items: CleanupItem[];
  selected: Set<string>;
  scanning: boolean;
  progress: number;
  scanPath: string;
  disk?: DiskInfo;
  onScan: () => void;
  onToggle: (id: string) => void;
  onOpenBasket: () => void;
  onClean?: () => void;
}

const scopes: Array<{ id: CleanupScope; label: string; shortLabel: string; icon: typeof Sparkles }> = [
  { id: 'system', label: '系统盘清理', shortLabel: '系统', icon: Sparkles },
  { id: 'apps', label: '软件缓存', shortLabel: '软件', icon: AppWindow },
  { id: 'browser', label: '浏览器数据', shortLabel: '浏览器', icon: Globe2 },
  { id: 'wechat', label: '微信专清', shortLabel: '微信', icon: MessageCircle },
];

const sumBytes = (items: CleanupItem[]) => items.reduce((sum, item) => sum + item.sizeBytes, 0);
const isRecommended = (item: CleanupItem) => item.selectable
  && item.risk === 'low'
  && item.confidence === 'high'
  && (item.recoverability === 'rebuildable' || item.deleteMode === 'quarantine');

function executionDescription(item: CleanupItem): string {
  if (item.deleteMode === 'quarantine') {
    return '移入本机隔离仓库；可在“隔离与记录”中导出副本，隔离占用不计入实际释放';
  }
  if (item.recoverability === 'rebuildable') {
    return '只删除扫描快照中未变化的文件，应用可重新生成';
  }
  if (item.recoverability === 'irreversible') {
    return '永久删除，必须在最终弹窗再次确认';
  }
  return '仅展示占用，普通清理入口已禁用';
}

function itemIcon(item: CleanupItem) {
  if (item.scope === 'browser') return <Globe2 />;
  if (item.scope === 'apps') return <AppWindow />;
  if (item.scope === 'wechat') return <MessageCircle />;
  return item.category.includes('系统') ? <Sparkles /> : <FileStack />;
}

function CleanupTile({ item, active, checked, onInspect, onToggle }: {
  item: CleanupItem;
  active: boolean;
  checked: boolean;
  onInspect: () => void;
  onToggle: () => void;
}) {
  return <article className={`cleanup-tile ${active ? 'active' : ''} ${!item.selectable ? 'protected' : ''}`}>
    <button
      type="button"
      className={`tile-check ${checked ? 'checked' : ''}`}
      role="checkbox"
      aria-label={`${checked ? '取消选择' : '选择'} ${item.name}`}
      aria-checked={checked}
      aria-disabled={!item.selectable}
      disabled={!item.selectable}
      onClick={onToggle}
    >
      {item.selectable ? checked && <Check /> : <CircleSlash2 />}
    </button>
    <button type="button" className="tile-main" onClick={onInspect} aria-label={`查看 ${item.name} 的清理依据`}>
      <span className={`item-symbol ${item.scope}`}>{itemIcon(item)}</span>
      <strong title={item.name}>{item.name}</strong>
      <small title={item.product}>{item.product}</small>
      <b>{formatBytes(item.sizeBytes)}</b>
      <span className="tile-meta">
        {item.blockedReason
          ? '使用中'
          : item.deleteMode === 'quarantine'
          ? '隔离处理'
          : item.recoverability === 'irreversible'
          ? '需确认'
          : `${item.fileCount.toLocaleString()} 个文件`}
      </span>
    </button>
  </article>;
}

export default function CleanupCenter({
  items, selected, scanning, progress, scanPath, disk,
  onScan, onToggle, onOpenBasket, onClean,
}: CleanupCenterProps) {
  const [scope, setScope] = useState<ScopeFilter>('all');
  const [activeId, setActiveId] = useState('');
  const [reviewOpen, setReviewOpen] = useState(true);
  const visibleItems = useMemo(
    () => scope === 'all' ? items : items.filter((item) => item.scope === scope),
    [items, scope],
  );
  const recommended = visibleItems.filter(isRecommended);
  const needsReview = visibleItems.filter((item) => !isRecommended(item));
  const active = visibleItems.find((item) => item.id === activeId);
  const selectedItems = items.filter((item) => selected.has(item.id));
  const selectedBytes = sumBytes(selectedItems);
  const discoveredBytes = sumBytes(items);
  const selectableVisible = visibleItems.filter((item) => item.selectable);
  const selectedVisibleCount = selectableVisible.filter((item) => selected.has(item.id)).length;
  const usedBytes = disk ? Math.max(0, disk.totalBytes - disk.freeBytes) : 0;

  useEffect(() => {
    if (!visibleItems.some((item) => item.id === activeId)) setActiveId(visibleItems[0]?.id || '');
  }, [activeId, visibleItems]);

  function toggleItems(targets: CleanupItem[]) {
    const selectable = targets.filter((item) => item.selectable);
    const allSelected = selectable.length > 0 && selectable.every((item) => selected.has(item.id));
    selectable.forEach((item) => {
      if (allSelected ? selected.has(item.id) : !selected.has(item.id)) onToggle(item.id);
    });
  }

  function requestCleanup() {
    if (selected.size) (onClean || onOpenBasket)();
  }

  return <section className="page cleanup-page">
    <div className="cleanup-hero">
      <div className="cleanup-drive-mark"><HardDrive /></div>
      <div className="cleanup-summary">
        <p>清理中心 <span><ShieldCheck />执行前逐文件复检</span></p>
        <h1>发现 <em>{formatBytes(discoveredBytes)}</em> 可释放，已勾选 <em>{formatBytes(selectedBytes)}</em></h1>
        <div className="cleanup-capacity" aria-label="各清理范围发现容量">
          {scopes.map((entry) => {
            const bytes = sumBytes(items.filter((item) => item.scope === entry.id));
            return <span key={entry.id} className={entry.id} style={{ flexGrow: Math.max(1, bytes) }} title={`${entry.label} ${formatBytes(bytes)}`} />;
          })}
        </div>
        <div className="capacity-caption">
          <span>{disk ? `${disk.name} ${disk.mount} · 已用 ${formatBytes(usedBytes)}` : `${items.length} 个清理规则`}</span>
          <span>{disk ? `${formatBytes(disk.freeBytes)} 可用 / 共 ${formatBytes(disk.totalBytes)}` : '文件信息仅在本机处理'}</span>
        </div>
      </div>
      <div className="cleanup-hero-actions">
        <button className="icon-button cleanup-refresh" onClick={onScan} disabled={scanning} aria-label="重新扫描" title="重新扫描"><RefreshCw /></button>
        <button className="button cleanup-now" onClick={requestCleanup} disabled={scanning || !selected.size}>
          <Trash2 /><span><strong>{selected.size ? '一键安全清理' : '请先选择项目'}</strong><small>{selected.size ? `${selected.size} 项 · ${formatBytes(selectedBytes)}` : '仅处理明确勾选的内容'}</small></span>
        </button>
      </div>
    </div>

    <div className="cleanup-scope-bar" role="tablist" aria-label="清理范围">
      <button role="tab" aria-selected={scope === 'all'} className={scope === 'all' ? 'active' : ''} onClick={() => setScope('all')}>
        <span><HardDrive /></span><div><strong>全部项目</strong><small>{items.length} 项</small></div><b>{formatBytes(discoveredBytes)}</b>
      </button>
      {scopes.map((entry) => {
        const scopeItems = items.filter((item) => item.scope === entry.id);
        return <button key={entry.id} role="tab" aria-selected={scope === entry.id} className={scope === entry.id ? `active ${entry.id}` : entry.id} onClick={() => setScope(entry.id)}>
          <span><entry.icon /></span><div><strong>{entry.label}</strong><small>{scopeItems.length} 项</small></div><b>{formatBytes(sumBytes(scopeItems))}</b>
        </button>;
      })}
    </div>

    {scanning ? <div className="scanner-panel">
      <div className="scanner-visual indeterminate" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuetext={progress === 100 ? '扫描完成' : '正在扫描，暂不提供百分比'}><span><ScanSearch /></span><i>扫描中</i></div>
      <div><p className="eyebrow">扫描阶段只读</p><h2>正在建立文件快照</h2><p>{scanPath || '正在枚举严格白名单目录…'}</p><div className="progress-track indeterminate"><span /></div><small><ShieldCheck />不会跟随符号链接，不会跨卷，也不会修改任何内容。</small></div>
    </div> : !visibleItems.length ? <EmptyState
      icon={scope === 'wechat' ? <MessageCircle /> : <ScanSearch />}
      title={items.length ? '此范围没有可清理内容' : '还没有扫描结果'}
      description={items.length ? '没有发现符合安全规则且实际包含文件的目录。' : '先进行一次只读扫描，系统会默认勾选低风险、可重建的内容。'}
      action={<button className="button primary" onClick={onScan}><ScanSearch />开始扫描</button>}
    /> : <>
      {scope === 'browser' && <SafetyNotice tone="info"><strong>登录状态与身份数据受到额外保护</strong><p>Cookie、会话、密码、书签和自动填充不会进入一键清理；正在运行的浏览器只展示占用，关闭后重新扫描才能选择。</p></SafetyNotice>}
      {scope === 'wechat' && <SafetyNotice tone="warning"><strong>聊天与媒体数据由你决定是否清理</strong><p>聊天记录、图片、视频、文件、语音、收藏和表情默认不勾选；主动选择后仍需最终确认，删除后无法恢复。</p></SafetyNotice>}

      {recommended.length > 0 && <div className="cleanup-section recommended-section">
        <header className="cleanup-section-head">
          <div><span className="section-status safe"><ShieldCheck /></span><span><strong>建议清理</strong><small>低风险、高置信且可重新生成</small></span></div>
          <div><b>{formatBytes(sumBytes(recommended))}</b><button className="section-select" onClick={() => toggleItems(recommended)}>{recommended.every((item) => selected.has(item.id)) ? '取消全选' : '全选本组'}</button></div>
        </header>
        <div className="cleanup-tile-grid">{recommended.map((item) => <CleanupTile key={item.id} item={item} active={activeId === item.id} checked={selected.has(item.id)} onInspect={() => setActiveId(item.id)} onToggle={() => onToggle(item.id)} />)}</div>
      </div>}

      {needsReview.length > 0 && <div className="cleanup-section review-section">
        <button className="cleanup-review-toggle" onClick={() => setReviewOpen((open) => !open)} aria-expanded={reviewOpen}>
          <span><span className="section-status review"><TriangleAlert /></span><span><strong>需确认清理项</strong><small>用户内容、占用项与受保护数据不会自动选择</small></span></span>
          <span><b>{formatBytes(sumBytes(needsReview))}</b><ChevronDown /></span>
        </button>
        {reviewOpen && <div className="cleanup-tile-grid review-grid">{needsReview.map((item) => <CleanupTile key={item.id} item={item} active={activeId === item.id} checked={selected.has(item.id)} onInspect={() => setActiveId(item.id)} onToggle={() => onToggle(item.id)} />)}</div>}
      </div>}

      {active && <aside className="cleanup-inspector" aria-live="polite">
        <div className={`evidence-icon ${active.scope}`}>{active.selectable ? <Info /> : <ShieldCheck />}</div>
        <div className="inspector-title"><p className="eyebrow">清理依据</p><h2>{active.name}</h2><span>{active.product} · {formatBytes(active.sizeBytes)} · {active.fileCount.toLocaleString()} 个文件</span></div>
        <DimensionTags item={active} />
        <dl><div><dt>识别依据</dt><dd>{active.reason}</dd></div><div><dt>匹配路径</dt><dd className="path-text">{active.path}</dd></div><div><dt>执行方式</dt><dd>{active.blockedReason || executionDescription(active)}</dd></div></dl>
        {active.selectable ? <button className={`button ${selected.has(active.id) ? 'secondary' : 'primary'}`} onClick={() => onToggle(active.id)}>{selected.has(active.id) ? '从清理计划移除' : '加入清理计划'}</button> : <div className="protected-proof"><ShieldCheck /><span><strong>{active.blockedReason ? active.scope === 'browser' ? '浏览器正在使用' : '应用正在使用' : '已受规则保护'}</strong><small>{active.blockedReason || '此类数据没有普通删除入口'}</small></span></div>}
      </aside>}

      <div className="sticky-selection">
        <span><ShieldCheck /><span>已选择 <strong>{selected.size} 项</strong><small>{formatBytes(selectedBytes)} · {selectedVisibleCount}/{selectableVisible.length} 个当前范围项目</small></span></span>
        <div><button className="button secondary" onClick={onOpenBasket}>查看清理计划</button><button className="button primary" disabled={!selected.size} onClick={requestCleanup}><Trash2 />复检并清理</button></div>
      </div>
    </>}
  </section>;
}
