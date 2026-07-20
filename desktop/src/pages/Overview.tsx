import { ArrowRight, CheckCircle2, Files, Fingerprint, HardDrive, History, ScanSearch, ShieldCheck, Sparkles } from 'lucide-react';
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

export default function Overview({ disk, items, records, lastScanAt, scanning, onScan, onNavigate }: OverviewProps) {
  const total = disk?.totalBytes || 0;
  const free = disk?.freeBytes || 0;
  const used = Math.max(0, total - free);
  const usedPercent = Math.round(percent(used, total));
  const safeBytes = items.filter((item) => item.selectable && item.risk === 'low' && item.confidence === 'high').reduce((sum, item) => sum + item.sizeBytes, 0);
  const reviewBytes = items.filter((item) => item.risk !== 'low' && item.selectable).reduce((sum, item) => sum + item.sizeBytes, 0);
  return <section className="page overview-page">
    <div className="overview-hero">
      <div className="hero-copy"><p className="eyebrow"><ShieldCheck />本地安全内核已启用</p><h1>看清空间去向，<br /><em>只清理有证据的内容。</em></h1><p>扫描与删除相互隔离。未知文件默认保留，用户内容从不自动勾选。</p><div className="hero-actions"><button className="button primary large" onClick={onScan} disabled={scanning}><ScanSearch />{scanning ? '正在只读扫描' : '扫描可安全清理项'}</button><button className="button ghost" onClick={() => onNavigate('analysis')}>查看空间地图<ArrowRight /></button></div><small className="last-scan"><CheckCircle2 />最近扫描：{lastScanAt}</small></div>
      <div className="disk-orbit-card">
        <div className="orbit-head"><span><HardDrive />{disk?.name || '正在读取磁盘'}</span><small>{disk?.mount || '--'}</small></div>
        <div className="disk-orbit" style={{ '--used': `${usedPercent * 3.6}deg` } as React.CSSProperties}><div><strong>{usedPercent}%</strong><span>已使用</span></div></div>
        <div className="disk-numbers"><div><small>可用空间</small><strong>{formatBytes(free)}</strong></div><div><small>磁盘容量</small><strong>{formatBytes(total)}</strong></div></div>
        <div className="orbit-foot"><span><i />已使用 {formatBytes(used)}</span><span><i />仍可用 {formatBytes(free)}</span></div>
      </div>
    </div>

    <div className="trust-strip">
      <div><span className="trust-icon safe"><Sparkles /></span><p><small>高置信度 · 可重建</small><strong>{formatBytes(safeBytes)}</strong><span>可进入默认清理</span></p></div>
      <div><span className="trust-icon review"><Fingerprint /></span><p><small>需要你判断</small><strong>{formatBytes(reviewBytes)}</strong><span>不会自动选择</span></p></div>
      <div><span className="trust-icon protected"><ShieldCheck /></span><p><small>保护规则</small><strong>4 个目录</strong><span>用户数据拒绝清理</span></p></div>
    </div>

    <div className="section-heading"><div><span>工作台</span><h2>从真实问题出发</h2></div><p>分析页不会直接删除任何文件。</p></div>
    <div className="workbench-grid">
      <button className="workbench-card blue" onClick={() => onNavigate('cleanup')}><span><Sparkles /></span><div><small>清理中心</small><h3>系统、浏览器与应用缓存</h3><p>逐条展示识别依据和删除影响。</p></div><ArrowRight /></button>
      <button className="workbench-card mint" onClick={() => onNavigate('files')}><span><Files /></span><div><small>文件发现</small><h3>大文件与精确重复项</h3><p>完整哈希确认，用户内容只读分析。</p></div><ArrowRight /></button>
      <button className="workbench-card amber" onClick={() => onNavigate('analysis')}><span><HardDrive /></span><div><small>磁盘地图</small><h3>目录与类型如何占用空间</h3><p>用矩形树图定位真正的空间热点。</p></div><ArrowRight /></button>
    </div>

    <div className="activity-layout"><div><div className="section-heading compact"><div><span>审计记录</span><h2>最近操作</h2></div><button className="text-button" onClick={() => onNavigate('recovery')}>全部记录<ArrowRight /></button></div><div className="activity-list">{records.slice(0, 3).map((record) => <div className="activity-row" key={record.id}><span className={`activity-icon ${record.kind}`}><History /></span><div><strong>{record.title}</strong><small>{record.createdAt} · {record.detail}</small></div><b>{record.reclaimedBytes ? `+ ${formatBytes(record.reclaimedBytes)}` : '已恢复'}</b></div>)}</div></div><aside className="guard-card"><div><ShieldCheck /><span>防误删状态</span></div><h3>7 道检查均正常</h3><ul><li><CheckCircle2 />规则白名单已加载</li><li><CheckCircle2 />重解析点保护开启</li><li><CheckCircle2 />执行前文件身份复检</li><li><CheckCircle2 />隐私数据仅保存在本机</li></ul><button className="text-button" onClick={() => onNavigate('settings')}>查看保护设置<ArrowRight /></button></aside></div>
  </section>;
}
