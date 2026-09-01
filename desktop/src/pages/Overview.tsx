import {
  ArrowRight,
  CheckCircle2,
  Files,
  HardDrive,
  History,
  ScanSearch,
  ShieldCheck,
  Sparkles,
} from 'lucide-react';
import { formatBytes, percent } from '../format';
import type { CleanupItem, DiskInfo, OperationRecord, Page } from '../types';

interface OverviewProps {
  disk?: DiskInfo;
  items: CleanupItem[];
  records: OperationRecord[];
  lastScanAt: string;
  scanning: boolean;
  onScan: () => void;
  onNavigate: (page: Page) => void;
}

export default function Overview({
  disk,
  items,
  records,
  lastScanAt,
  scanning,
  onScan,
  onNavigate,
}: OverviewProps): JSX.Element {
  const total = disk?.totalBytes || 0;
  const free = disk?.freeBytes || 0;
  const used = Math.max(0, total - free);
  const usedPercent = Math.round(percent(used, total));
  const safeItems = items.filter((item) => item.selectable && item.risk === 'low' && item.confidence === 'high');
  const safeBytes = safeItems.reduce((sum, item) => sum + item.sizeBytes, 0);
  const reviewItems = items.filter((item) => item.selectable && (item.risk !== 'low' || item.confidence !== 'high'));
  const reviewBytes = reviewItems.reduce((sum, item) => sum + item.sizeBytes, 0);

  return <section className="page overview-page">
    <header className="overview-head">
      <div>
        <p className="eyebrow"><span className="status-dot" />本机保护已开启</p>
        <h1>空间概览</h1>
        <p>{disk ? `${disk.name} (${disk.mount})` : '正在读取本机磁盘'}</p>
      </div>
      <span className="privacy-status"><ShieldCheck />文件数据仅在本机处理</span>
    </header>

    <div className="dashboard-primary">
      <div className="scan-action-zone">
        <span className="scan-symbol"><Sparkles /></span>
        <p>可安全清理</p>
        <strong className="cleanable-number">{formatBytes(safeBytes)}</strong>
        <span className="cleanable-caption">{safeItems.length ? `${safeItems.length} 个低风险项目` : '扫描后显示安全建议'}</span>
        <button className="button primary scan-now" type="button" onClick={onScan} disabled={scanning}>
          <ScanSearch />{scanning ? '正在扫描' : '立即扫描'}
        </button>
        <small className="last-scan"><History />最近扫描：{lastScanAt}</small>
      </div>

      <aside className="disk-usage-panel">
        <div className="disk-panel-head"><span><HardDrive />磁盘使用</span><small>{disk?.mount || '--'}</small></div>
        <div className="disk-ring-wrap">
          <svg className="disk-ring" viewBox="0 0 120 120" role="img" aria-label={`磁盘已使用 ${usedPercent}%`}>
            <circle className="disk-ring-track" cx="60" cy="60" r="48" pathLength="100" />
            <circle className="disk-ring-value" cx="60" cy="60" r="48" pathLength="100" strokeDasharray={`${usedPercent} ${100 - usedPercent}`} />
          </svg>
          <span><strong>{usedPercent}%</strong><small>已使用</small></span>
        </div>
        <div className="disk-metrics">
          <span><small>可用空间</small><strong>{formatBytes(free)}</strong></span>
          <span><small>总容量</small><strong>{formatBytes(total)}</strong></span>
        </div>
      </aside>
    </div>

    <div className="overview-metrics" aria-label="扫描概览">
      <div><span className="metric-icon safe"><CheckCircle2 /></span><span><small>安全建议</small><strong>{formatBytes(safeBytes)}</strong></span></div>
      <div><span className="metric-icon review"><ShieldCheck /></span><span><small>需要复核</small><strong>{formatBytes(reviewBytes)}</strong></span></div>
      <div><span className="metric-icon files"><Files /></span><span><small>已识别项目</small><strong>{items.length} 项</strong></span></div>
    </div>

    <div className="overview-lower">
      <section className="recent-activity">
        <header><div><p className="eyebrow">最近活动</p><h2>清理记录</h2></div><button className="text-button" type="button" onClick={() => onNavigate('recovery')}>查看全部<ArrowRight /></button></header>
        <div className="activity-list">
          {records.length ? records.slice(0, 3).map((record) => <div className="activity-row" key={record.id}>
            <span className={`activity-icon ${record.kind}`}><History /></span>
            <div><strong>{record.title}</strong><small>{record.createdAt} · {record.detail}</small></div>
            <b>{record.reclaimedBytes ? `+ ${formatBytes(record.reclaimedBytes)}` : '已恢复'}</b>
          </div>) : <div className="activity-empty"><History /><span><strong>暂无清理记录</strong><small>完成首次清理后会显示在这里</small></span></div>}
        </div>
      </section>

      <section className="quick-actions">
        <header><p className="eyebrow">快捷入口</p><h2>常用工具</h2></header>
        <button type="button" onClick={() => onNavigate('cleanup')}><Sparkles /><span><strong>清理中心</strong><small>{formatBytes(safeBytes + reviewBytes)} 已发现</small></span><ArrowRight /></button>
        <button type="button" onClick={() => onNavigate('tools')}><Files /><span><strong>高级工具</strong><small>大文件、重复项与磁盘分析</small></span><ArrowRight /></button>
      </section>
    </div>
  </section>;
}
