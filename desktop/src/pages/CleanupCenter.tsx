import { useEffect, useMemo, useState } from 'react';
import { AppWindow, Check, ChevronRight, CircleSlash2, Globe2, Info, ListChecks, ScanSearch, ShieldCheck, ShoppingBasket, Sparkles } from 'lucide-react';
import { DimensionTags, EmptyState, PageHeader, SafetyNotice } from '../components';
import { formatBytes } from '../format';
import type { CleanupItem, CleanupScope } from '../types';

interface CleanupCenterProps {
  items: CleanupItem[];
  selected: Set<string>;
  scanning: boolean;
  progress: number;
  scanPath: string;
  onScan: () => void;
  onToggle: (id: string) => void;
  onOpenBasket: () => void;
}

const scopes: Array<{ id: CleanupScope; label: string; icon: typeof Sparkles; note: string }> = [
  { id: 'system', label: '系统垃圾', icon: Sparkles, note: '临时文件、缩略图与诊断数据' },
  { id: 'browser', label: '浏览器数据', icon: Globe2, note: '按浏览器和配置文件区分' },
  { id: 'apps', label: '应用缓存', icon: AppWindow, note: '只有可重建内容可清理' },
];

export default function CleanupCenter({ items, selected, scanning, progress, scanPath, onScan, onToggle, onOpenBasket }: CleanupCenterProps) {
  const [scope, setScope] = useState<CleanupScope>('system');
  const visibleItems = useMemo(() => items.filter((item) => item.scope === scope), [items, scope]);
  const [activeId, setActiveId] = useState('');
  useEffect(() => { if (!visibleItems.some((item) => item.id === activeId)) setActiveId(visibleItems[0]?.id || ''); }, [activeId, visibleItems]);
  const active = visibleItems.find((item) => item.id === activeId);
  const selectedItems = items.filter((item) => selected.has(item.id));
  const selectedBytes = selectedItems.reduce((sum, item) => sum + item.sizeBytes, 0);
  const scopeTotal = visibleItems.reduce((sum, item) => sum + item.sizeBytes, 0);
  const groups = Array.from(new Set(visibleItems.map((item) => item.category)));

  return <section className="page cleanup-page">
    <PageHeader eyebrow="只读发现 → 生成计划 → 执行前复检" title="清理中心" description="默认只选择高置信度、可重建、低影响的内容。" actions={<><button className="button secondary" onClick={onOpenBasket}><ShoppingBasket />清理篮 {selected.size > 0 && <b>{selected.size}</b>}</button><button className="button primary" onClick={onScan} disabled={scanning}><ScanSearch />{scanning ? '扫描中' : '重新扫描'}</button></>} />
    <div className="scope-switcher">{scopes.map((entry) => <button key={entry.id} className={scope === entry.id ? 'active' : ''} onClick={() => setScope(entry.id)}><entry.icon /><span><strong>{entry.label}</strong><small>{entry.note}</small></span><ChevronRight /></button>)}</div>

    {scanning ? <div className="scanner-panel"><div className="scanner-visual"><span style={{ '--progress': `${progress * 3.6}deg` } as React.CSSProperties}><ScanSearch /></span><i>{progress}%</i></div><div><p className="eyebrow">扫描阶段只读</p><h2>正在建立文件快照</h2><p>{scanPath || '正在枚举严格白名单目录…'}</p><div className="progress-track"><span style={{ width: `${progress}%` }} /></div><small><ShieldCheck />不会跟随符号链接，不会跨卷，也不会修改任何内容。</small></div></div> : !visibleItems.length ? <EmptyState icon={<ScanSearch />} title="还没有扫描结果" description="先进行一次只读扫描，结果不会自动加入清理篮。" action={<button className="button primary" onClick={onScan}><ScanSearch />开始扫描</button>} /> : <>
      <div className="result-ribbon"><div><small>当前范围发现</small><strong>{formatBytes(scopeTotal)}</strong></div><div><small>清理篮已选</small><strong>{formatBytes(selectedBytes)}</strong></div><span><ShieldCheck />新增或变化的文件在执行时自动跳过</span></div>
      {scope === 'browser' && <SafetyNotice tone="info"><strong>登录状态与身份数据受到额外保护</strong><p>Cookie、会话、密码、书签和自动填充不会进入一键清理；缓存清理后首次打开网页可能稍慢。</p></SafetyNotice>}
      <div className="cleanup-workspace"><div className="cleanup-results">{groups.map((group) => <div className="result-group" key={group}><header><div><span>{group}</span><small>{visibleItems.filter((item) => item.category === group).length} 项</small></div><b>{formatBytes(visibleItems.filter((item) => item.category === group).reduce((sum, item) => sum + item.sizeBytes, 0))}</b></header>{visibleItems.filter((item) => item.category === group).map((item) => <button type="button" key={item.id} className={`cleanup-row ${activeId === item.id ? 'active' : ''} ${!item.selectable ? 'protected' : ''}`} onClick={() => setActiveId(item.id)}><span className={`check-box ${selected.has(item.id) ? 'checked' : ''} ${!item.selectable ? 'disabled' : ''}`} role="checkbox" aria-checked={selected.has(item.id)} aria-disabled={!item.selectable} onClick={(event) => { event.stopPropagation(); if (item.selectable) onToggle(item.id); }}>{item.selectable ? selected.has(item.id) && <Check /> : <CircleSlash2 />}</span><span className={`item-symbol ${item.scope}`}>{item.scope === 'browser' ? <Globe2 /> : item.scope === 'apps' ? <AppWindow /> : <Sparkles />}</span><span className="row-copy"><strong>{item.name}{!item.selectable && <em>受保护</em>}</strong><small>{item.product} · {item.description}</small><DimensionTags item={item} compact /></span><b>{formatBytes(item.sizeBytes)}</b><ChevronRight /></button>)}</div>)}</div>
        <aside className="evidence-panel">{active ? <><div className={`evidence-icon ${active.scope}`}>{active.selectable ? <ListChecks /> : <ShieldCheck />}</div><p className="eyebrow">判定证据</p><h2>{active.name}</h2><p className="evidence-product">{active.product}</p><DimensionTags item={active} /><dl><div><dt>为什么识别为此类</dt><dd>{active.reason}</dd></div><div><dt>匹配路径</dt><dd className="path-text">{active.path}</dd></div><div><dt>扫描快照</dt><dd>{active.fileCount ? `${active.fileCount.toLocaleString()} 个文件` : '执行时逐文件复检'}</dd></div><div><dt>处理方式</dt><dd>{active.recoverability === 'rebuildable' ? '永久删除派生缓存，应用可重建' : active.recoverability === 'protected' ? '仅展示占用，规则内核拒绝清理' : '需要独立确认'}</dd></div></dl>{active.selectable ? <button className={`button ${selected.has(active.id) ? 'secondary' : 'primary'} wide`} onClick={() => onToggle(active.id)}>{selected.has(active.id) ? '从清理篮移除' : '加入清理篮'}</button> : <div className="protected-proof"><ShieldCheck /><span><strong>已受规则保护</strong><small>此类数据没有普通删除入口</small></span></div>}</> : <EmptyState title="选择一项查看证据" description="每个结果都会说明识别依据和删除影响。" />}</aside>
      </div>
      <div className="sticky-selection"><span><ShoppingBasket /><span>已选择 <strong>{selected.size} 项</strong><small>{formatBytes(selectedBytes)} · 执行前仍需复检</small></span></span><button className="button primary" disabled={!selected.size} onClick={onOpenBasket}>检查清理计划<ChevronRight /></button></div>
    </>}
  </section>;
}
