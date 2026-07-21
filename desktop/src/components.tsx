import type { ReactNode } from 'react';
import { AlertTriangle, Check, ChevronRight, Database, Info, LockKeyhole, RotateCcw, ShieldCheck, ShoppingBasket, Sparkles, Trash2, X } from 'lucide-react';
import { formatBytes } from './format';
import type { CleanupItem, Confidence, Impact, Recoverability } from './types';

export function PageHeader({ eyebrow, title, description, actions }: { eyebrow?: string; title: string; description: string; actions?: ReactNode }) {
  return <header className="page-header"><div>{eyebrow && <p className="eyebrow">{eyebrow}</p>}<h1>{title}</h1><p>{description}</p></div>{actions && <div className="page-actions">{actions}</div>}</header>;
}

const confidenceLabel: Record<Confidence, string> = { high: '高置信', medium: '需复核', low: '低置信' };
const impactLabel: Record<Impact, string> = { none: '无感', rebuild: '需重建', signout: '会退出登录', user_data: '用户内容' };
const recoverabilityLabel: Record<Recoverability, string> = { rebuildable: '可重建', recoverable: '可恢复', irreversible: '不可恢复', protected: '受保护' };

export function DimensionTags({ item, compact = false }: { item: Pick<CleanupItem, 'confidence' | 'impact' | 'recoverability'>; compact?: boolean }) {
  return <div className={`dimension-tags ${compact ? 'compact' : ''}`}>
    <span className={`dimension confidence-${item.confidence}`}><Check />{confidenceLabel[item.confidence]}</span>
    <span className={`dimension impact-${item.impact}`}><Sparkles />{impactLabel[item.impact]}</span>
    <span className={`dimension recovery-${item.recoverability}`}>{item.recoverability === 'protected' ? <LockKeyhole /> : <RotateCcw />}{recoverabilityLabel[item.recoverability]}</span>
  </div>;
}

export function SafetyNotice({ tone = 'safe', children }: { tone?: 'safe' | 'warning' | 'info'; children: ReactNode }) {
  const Icon = tone === 'safe' ? ShieldCheck : tone === 'warning' ? AlertTriangle : Info;
  return <div className={`safety-notice ${tone}`}><Icon /><div>{children}</div></div>;
}

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (checked: boolean) => void; label: string }) {
  return <button type="button" role="switch" aria-checked={checked} aria-label={label} className={`toggle ${checked ? 'on' : ''}`} onClick={() => onChange(!checked)}><span /></button>;
}

export function Dialog({ title, children, confirmLabel = '确认', danger = false, busy = false, confirmDisabled = false, onClose, onConfirm }: {
  title: string; children: ReactNode; confirmLabel?: string; danger?: boolean; busy?: boolean; confirmDisabled?: boolean; onClose: () => void; onConfirm: () => void;
}) {
  return <div className="overlay" role="presentation" onMouseDown={onClose}><div className="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title" onMouseDown={(event) => event.stopPropagation()}>
    <button className="icon-button dialog-close" onClick={onClose} aria-label="关闭"><X /></button>
    <h2 id="dialog-title">{title}</h2><div className="dialog-body">{children}</div>
    <div className="dialog-actions"><button className="button secondary" onClick={onClose}>取消</button><button className={`button ${danger ? 'danger' : 'primary'}`} disabled={busy || confirmDisabled} onClick={onConfirm}>{busy ? '正在复检…' : confirmLabel}</button></div>
  </div></div>;
}

export function EmptyState({ icon, title, description, action }: { icon?: ReactNode; title: string; description: string; action?: ReactNode }) {
  return <div className="empty-state"><span>{icon || <Database />}</span><h3>{title}</h3><p>{description}</p>{action}</div>;
}

export function BasketDrawer({ items, busy, onClose, onExecute, onRemove }: {
  items: CleanupItem[]; busy: boolean; onClose: () => void; onExecute: () => void; onRemove: (id: string) => void;
}) {
  const total = items.reduce((sum, item) => sum + item.sizeBytes, 0);
  const reviewCount = items.filter((item) => item.risk !== 'low' || item.recoverability === 'irreversible').length;
  return <div className="drawer-layer" role="presentation" onMouseDown={onClose}><aside className="basket-drawer" aria-label="清理篮" onMouseDown={(event) => event.stopPropagation()}>
    <header><div><span className="drawer-kicker"><ShoppingBasket />清理篮</span><h2>{items.length ? `${items.length} 项待复检` : '还没有选择项目'}</h2></div><button className="icon-button" onClick={onClose} aria-label="关闭清理篮"><X /></button></header>
    {!items.length ? <EmptyState icon={<ShoppingBasket />} title="清理篮是空的" description="扫描后，只有高置信度的可清理项才能加入这里。" /> : <>
      <div className="basket-summary"><div><small>预计可释放</small><strong>{formatBytes(total)}</strong></div><span><ShieldCheck />执行前逐文件复检</span></div>
      {reviewCount > 0 && <SafetyNotice tone="warning"><strong>{reviewCount} 项需要额外确认</strong><p>包含诊断记录或不可恢复内容，请在最终确认中逐项复核。</p></SafetyNotice>}
      <div className="basket-list">{items.map((item) => <div className="basket-item" key={item.id}><span className={`scope-mark ${item.scope}`} /> <div><strong>{item.name}</strong><small>{item.product} · {formatBytes(item.sizeBytes)}</small></div><button className="icon-button" onClick={() => onRemove(item.id)} aria-label={`移除 ${item.name}`}><X /></button></div>)}</div>
      <div className="basket-proof"><h3>执行时会再次确认</h3><ul><li>路径仍在规则白名单内</li><li>文件大小与修改时间未变化</li><li>不是符号链接、联接点或云占位文件</li><li>被应用锁定的文件会跳过</li></ul></div>
      <footer><div><small>预计释放</small><strong>{formatBytes(total)}</strong></div><button className="button primary wide" disabled={busy} onClick={onExecute}><Trash2 />{busy ? '正在安全复检' : '查看并执行计划'}<ChevronRight /></button></footer>
    </>}
  </aside></div>;
}
