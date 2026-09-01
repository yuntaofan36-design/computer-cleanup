import {
  AppWindow,
  ArrowUpRight,
  ChartNoAxesCombined,
  Files,
  FolderSearch2,
  HardDrive,
  RotateCcw,
  Rocket,
} from 'lucide-react';
import type { FileDiscoveryTab } from './FileDiscovery';
import type { Page } from '../types';

interface ToolboxProps {
  largeFileCount: number;
  duplicateGroupCount: number;
  appCount: number;
  startupCount: number;
  analyzedDirectoryCount: number;
  onOpenFileDiscovery: (tab: FileDiscoveryTab) => void;
  onNavigate: (page: Page) => void;
}

const countLabel = (count: number, unit: string): string => count > 0 ? `${count.toLocaleString()} ${unit}` : '等待分析';

export default function Toolbox({
  largeFileCount,
  duplicateGroupCount,
  appCount,
  startupCount,
  analyzedDirectoryCount,
  onOpenFileDiscovery,
  onNavigate,
}: ToolboxProps): JSX.Element {
  return <section className="page toolbox-page">
    <header className="page-head toolbox-head">
      <div className="page-title-block">
        <p className="eyebrow">高级工具</p>
        <h1>工具箱</h1>
        <p>按任务查看空间，不会在分析阶段修改文件。</p>
      </div>
    </header>

    <div className="tool-grid">
      <button className="tool-card" type="button" onClick={() => onOpenFileDiscovery('duplicates')}>
        <span className="tool-icon"><Files /></span>
        <span className="tool-copy"><strong>重复文件</strong><small>{countLabel(duplicateGroupCount, '组已核验')}</small></span>
        <ArrowUpRight className="tool-arrow" />
      </button>
      <button className="tool-card" type="button" onClick={() => onOpenFileDiscovery('large-files')}>
        <span className="tool-icon"><FolderSearch2 /></span>
        <span className="tool-copy"><strong>大文件</strong><small>{countLabel(largeFileCount, '项已发现')}</small></span>
        <ArrowUpRight className="tool-arrow" />
      </button>
      <button className="tool-card" type="button" onClick={() => onNavigate('analysis')}>
        <span className="tool-icon"><ChartNoAxesCombined /></span>
        <span className="tool-copy"><strong>磁盘分析</strong><small>{countLabel(analyzedDirectoryCount, '个目录')}</small></span>
        <ArrowUpRight className="tool-arrow" />
      </button>
      <button className="tool-card" type="button" onClick={() => onNavigate('startup')}>
        <span className="tool-icon"><Rocket /></span>
        <span className="tool-copy"><strong>启动项</strong><small>{countLabel(startupCount, '项已登记')}</small></span>
        <ArrowUpRight className="tool-arrow" />
      </button>
    </div>

    <div className="toolbox-secondary">
      <button type="button" onClick={() => onNavigate('apps')}><AppWindow /><span><strong>应用管理</strong><small>{countLabel(appCount, '个应用')}</small></span><ArrowUpRight /></button>
      <button type="button" onClick={() => onNavigate('partition')}><HardDrive /><span><strong>磁盘分区</strong><small>查看卷与未分配空间</small></span><ArrowUpRight /></button>
      <button type="button" onClick={() => onNavigate('recovery')}><RotateCcw /><span><strong>恢复中心</strong><small>隔离区与操作记录</small></span><ArrowUpRight /></button>
    </div>
  </section>;
}
