import { useEffect, useMemo, useState } from 'react';
import {
  Bell,
  Boxes,
  ChartNoAxesCombined,
  ChevronDown,
  Files,
  HardDrive,
  LayoutDashboard,
  Package,
  RotateCcw,
  Settings,
  ShieldCheck,
  ShoppingBasket,
  Sparkles,
  X,
} from 'lucide-react';
import { BasketDrawer, Dialog } from './components';
import {
  directories as previewDirectories,
  disks as previewDisks,
  duplicateGroups as previewDuplicateGroups,
  largeFiles as previewLargeFiles,
  protectedPaths,
  records as previewRecords,
  storageCategories as previewStorageCategories,
} from './mockData';
import {
  cancelNativeTask,
  executeCleanup,
  isNativeRuntime,
  loadApps,
  loadDisks,
  loadOperationRecords,
  requestUninstall,
  revealInExplorer,
  scanCleanup,
  scanDuplicateFiles,
  scanLargeFiles,
  scanStorageUsage,
} from './native';
import AppManagement from './pages/AppManagement';
import CleanupCenter from './pages/CleanupCenter';
import FileDiscovery, { type FileDiscoveryTab } from './pages/FileDiscovery';
import Overview from './pages/Overview';
import RecoveryCenter from './pages/RecoveryCenter';
import SettingsPage from './pages/SettingsPage';
import StorageAnalysis from './pages/StorageAnalysis';
import { useAppStore } from './store';
import { formatBytes } from './format';
import type {
  AppEntry,
  DirectoryUsage,
  DiskInfo,
  DuplicateGroup,
  LargeFileEntry,
  OperationRecord,
  Page,
  StorageCategory,
} from './types';

const primaryNav: Array<{ id: Page; label: string; description: string; icon: typeof LayoutDashboard }> = [
  { id: 'overview', label: '空间概览', description: '磁盘与建议', icon: LayoutDashboard },
  { id: 'cleanup', label: '清理中心', description: '有证据地清理', icon: Sparkles },
  { id: 'files', label: '文件发现', description: '大文件与重复项', icon: Files },
  { id: 'analysis', label: '磁盘地图', description: '空间可视化', icon: ChartNoAxesCombined },
];

const secondaryNav: Array<{ id: Page; label: string; icon: typeof Package }> = [
  { id: 'apps', label: '应用管理', icon: Package },
  { id: 'recovery', label: '恢复与记录', icon: RotateCcw },
  { id: 'settings', label: '安全设置', icon: Settings },
];

const scanPaths = [
  '正在读取 Windows 临时目录…',
  '正在识别浏览器配置文件…',
  '正在核对可重建应用缓存…',
  '正在排除链接、同步目录与锁定文件…',
  '正在生成不可变清理计划…',
];

const builtInExclusionRules = [
  '*.pst / *.ost 邮件存档',
  '*.vhd / *.vhdx 虚拟磁盘',
  'Git 与工作区目录',
  'OneDrive 未同步文件',
];
const exclusionsKey = 'qingpanUserExclusions';
const unavailableDisk: DiskInfo = { id: 'unavailable', name: '磁盘不可用', mount: '', totalBytes: 0, freeBytes: 0 };
type TaskStatus = 'idle' | 'scanning' | 'complete';

function readUserExclusions(): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(exclusionsKey) || '[]');
    return Array.isArray(parsed) ? parsed.filter((value): value is string => typeof value === 'string') : [];
  } catch {
    return [];
  }
}

function scanTime(): string {
  return new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' }).format(new Date());
}

function pathIsInside(path: string, excludedPath: string): boolean {
  const normalize = (value: string) => value.replace(/\//g, '\\').replace(/\\+$/, '').toLocaleLowerCase();
  const candidate = normalize(path);
  const excluded = normalize(excludedPath);
  return candidate === excluded || candidate.startsWith(`${excluded}\\`);
}

function exclusionsWithinRoot(exclusions: string[], root: string): string[] {
  return exclusions.filter((path) => pathIsInside(path, root) && !pathIsInside(root, path));
}

export default function App() {
  const {
    page, setPage, theme, disks, setDisks, activeDiskId, setActiveDiskId,
    cleanupItems, setCleanupItems, selected, toggleItem, setSafeDefaults, clearSelection, removeSelected,
    scanning, setScanning, progress, setProgress, scanPath, setScanPath, lastScanAt, setLastScanAt,
    basketOpen, setBasketOpen,
  } = useAppStore();
  const nativeRuntime = isNativeRuntime();
  const [installedApps, setInstalledApps] = useState<AppEntry[]>([]);
  const [operationRecords, setOperationRecords] = useState<OperationRecord[]>(nativeRuntime ? [] : previewRecords);
  const [discoveryFiles, setDiscoveryFiles] = useState<LargeFileEntry[]>(nativeRuntime ? [] : previewLargeFiles);
  const [discoveryDuplicates, setDiscoveryDuplicates] = useState<DuplicateGroup[]>(nativeRuntime ? [] : previewDuplicateGroups);
  const [analysisDirectories, setAnalysisDirectories] = useState<DirectoryUsage[]>(nativeRuntime ? [] : previewDirectories);
  const [analysisCategories, setAnalysisCategories] = useState<StorageCategory[]>(nativeRuntime ? [] : previewStorageCategories);
  const [fileScanStatus, setFileScanStatus] = useState<TaskStatus>(nativeRuntime ? 'idle' : 'complete');
  const [analysisScanStatus, setAnalysisScanStatus] = useState<TaskStatus>(nativeRuntime ? 'idle' : 'complete');
  const [fileScannedAt, setFileScannedAt] = useState(nativeRuntime ? '' : '演示数据');
  const [analysisScannedAt, setAnalysisScannedAt] = useState(nativeRuntime ? '' : '演示数据');
  const [fileTaskId, setFileTaskId] = useState<string | null>(null);
  const [analysisTaskId, setAnalysisTaskId] = useState<string | null>(null);
  const [userExclusions, setUserExclusions] = useState<string[]>(readUserExclusions);
  const [busyAppId, setBusyAppId] = useState<string | null>(null);
  const [executeOpen, setExecuteOpen] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [toast, setToast] = useState('');
  const activeDisk = disks.find((disk) => disk.id === activeDiskId) || disks[0] || (nativeRuntime ? unavailableDisk : previewDisks[0]);
  const selectedItems = useMemo(() => cleanupItems.filter((item) => selected.has(item.id)), [cleanupItems, selected]);
  const selectedBytes = selectedItems.reduce((sum, item) => sum + item.sizeBytes, 0);
  const irreversibleCount = selectedItems.filter((item) => item.recoverability === 'irreversible').length;
  const anyReadScan = fileScanStatus === 'scanning' || analysisScanStatus === 'scanning';

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem('qingpanTheme', theme);
  }, [theme]);

  useEffect(() => {
    loadDisks().then(setDisks).catch((error) => setToast(error instanceof Error ? error.message : '无法读取磁盘'));
    loadApps().then(setInstalledApps).catch((error) => setToast(error instanceof Error ? error.message : '无法读取应用清单'));
    loadOperationRecords(50).then(setOperationRecords).catch(() => setOperationRecords(nativeRuntime ? [] : previewRecords));
    if (!nativeRuntime) {
      void scanCleanup().then(setCleanupItems);
      setLastScanAt('演示数据 · 尚未执行真实扫描');
    }
  }, [nativeRuntime, setCleanupItems, setDisks, setLastScanAt]);

  useEffect(() => {
    if (!toast) return;
    const timeout = window.setTimeout(() => setToast(''), 4200);
    return () => window.clearTimeout(timeout);
  }, [toast]);

  async function runScan() {
    if (scanning) return;
    setPage('cleanup');
    setScanning(true);
    setProgress(3);
    setScanPath(scanPaths[0]);
    clearSelection();
    let index = 0;
    const timer = window.setInterval(() => {
      setProgress(Math.min(88, useAppStore.getState().progress + 4));
      index = (index + 1) % scanPaths.length;
      setScanPath(scanPaths[index]);
    }, 180);
    try {
      if (!nativeRuntime) await new Promise((resolve) => window.setTimeout(resolve, 950));
      const items = await scanCleanup();
      window.clearInterval(timer);
      setCleanupItems(items);
      setSafeDefaults(items);
      setProgress(100);
      setScanPath(`已建立 ${items.length} 项安全快照`);
      setLastScanAt(scanTime());
      window.setTimeout(() => setScanning(false), 420);
    } catch (error) {
      window.clearInterval(timer);
      setScanning(false);
      setToast(error instanceof Error ? error.message : '扫描失败，请重试');
    }
  }

  async function runCleanup() {
    if (!selectedItems.length || executing) return;
    setExecuting(true);
    try {
      const ids = selectedItems.map((item) => item.id);
      const result = await executeCleanup(ids);
      if (nativeRuntime) {
        const refreshed = await scanCleanup();
        setCleanupItems(refreshed);
      } else {
        setCleanupItems(cleanupItems.filter((item) => !ids.includes(item.id)));
      }
      removeSelected(ids);
      setExecuteOpen(false);
      setBasketOpen(false);
      setOperationRecords(await loadOperationRecords(50));
      setToast(`已实际释放 ${formatBytes(result.reclaimedBytes)}${result.failed.length ? `，${result.failed.length} 个文件因变化或占用被保留` : ''}`);
    } catch (error) {
      setToast(error instanceof Error ? error.message : '清理未执行');
    } finally {
      setExecuting(false);
    }
  }

  async function runFileScan(tab: FileDiscoveryTab) {
    if (!activeDisk.mount || fileScanStatus === 'scanning') return;
    const taskId = crypto.randomUUID();
    setFileTaskId(taskId);
    setFileScanStatus('scanning');
    try {
      if (tab === 'large-files') {
        const result = await scanLargeFiles(taskId, activeDisk.mount, {
          minSizeBytes: 256 * 1024 ** 2,
          maxFiles: 750_000,
          maxResults: 2_000,
          excludedPaths: exclusionsWithinRoot(userExclusions, activeDisk.mount),
        });
        if (!result.cancelled) setDiscoveryFiles(result.files);
        setToast(result.cancelled ? '大文件分析已取消，保留上次结果' : `已分析 ${result.scannedFiles.toLocaleString()} 个文件，跳过 ${result.skipped.toLocaleString()} 项${result.limitReached ? '，结果达到安全上限' : ''}`);
      } else {
        const result = await scanDuplicateFiles(taskId, activeDisk.mount, {
          minSizeBytes: 1024 ** 2,
          maxFiles: 500_000,
          maxGroups: 2_000,
          maxMembers: 10_000,
          sampleBytes: 64 * 1024,
          excludedPaths: exclusionsWithinRoot(userExclusions, activeDisk.mount),
        });
        if (!result.cancelled) setDiscoveryDuplicates(result.groups);
        setToast(result.cancelled ? '重复文件分析已取消，保留上次结果' : `已完成内容校验，发现 ${result.groups.length.toLocaleString()} 组重复项，跳过 ${result.skipped.toLocaleString()} 项`);
      }
      setFileScannedAt(scanTime());
      setFileScanStatus('complete');
    } catch (error) {
      setFileScanStatus('idle');
      setToast(error instanceof Error ? error.message : '文件分析失败');
    } finally {
      setFileTaskId(null);
    }
  }

  async function cancelFileScan() {
    if (!fileTaskId) return;
    await cancelNativeTask(fileTaskId);
    setToast('正在安全停止文件分析…');
  }

  async function runStorageScan(root = activeDisk.mount, merge = false) {
    if (!root || analysisScanStatus === 'scanning') return;
    const taskId = crypto.randomUUID();
    setAnalysisTaskId(taskId);
    setAnalysisScanStatus('scanning');
    try {
      const result = await scanStorageUsage(taskId, root, {
        maxFiles: 750_000,
        maxResults: 10_000,
        excludedPaths: exclusionsWithinRoot(userExclusions, root),
      });
      if (!result.cancelled) {
        setAnalysisDirectories((current) => {
          if (!merge) return result.directories;
          const combined = new Map(current.map((directory) => [directory.id, directory]));
          result.directories.forEach((directory) => combined.set(directory.id, directory));
          return [...combined.values()];
        });
        setAnalysisCategories(result.categories);
      }
      setAnalysisScannedAt(scanTime());
      setAnalysisScanStatus('complete');
      setToast(result.cancelled ? '目录分析已取消，保留上次结果' : `已索引 ${result.scannedFiles.toLocaleString()} 个文件${result.limitReached ? '，扫描达到安全上限' : ''}`);
    } catch (error) {
      setAnalysisScanStatus('idle');
      setToast(error instanceof Error ? error.message : '目录分析失败');
    } finally {
      setAnalysisTaskId(null);
    }
  }

  async function cancelStorageScan() {
    if (!analysisTaskId) return;
    await cancelNativeTask(analysisTaskId);
    setToast('正在安全停止目录分析…');
  }

  async function uninstall(id: string) {
    if (busyAppId) return;
    setBusyAppId(id);
    try {
      const result = await requestUninstall(id);
      setToast(`已启动应用官方卸载器（PID ${result.pid}）`);
      setOperationRecords(await loadOperationRecords(50));
    } catch (error) {
      setToast(error instanceof Error ? error.message : '无法启动卸载程序');
    } finally {
      setBusyAppId(null);
    }
  }

  async function reveal(path: string) {
    try {
      await revealInExplorer(path);
      setToast(nativeRuntime ? '已在文件资源管理器中定位' : `演示定位：${path}`);
    } catch (error) {
      setToast(error instanceof Error ? error.message : '无法定位文件');
    }
  }

  function addExclusion(path: string) {
    if (userExclusions.some((entry) => pathIsInside(path, entry))) {
      setToast('此路径已在排除规则中');
      return;
    }
    const next = [...userExclusions, path];
    setUserExclusions(next);
    localStorage.setItem(exclusionsKey, JSON.stringify(next));
    setDiscoveryFiles((files) => files.filter((file) => !pathIsInside(file.path, path)));
    setDiscoveryDuplicates((groups) => groups
      .map((group) => {
        let members = group.members.filter((member) => !pathIsInside(member.path, path));
        if (members.length > 0 && !members.some((member) => member.suggestedKeep)) {
          members = members.map((member, index) => ({ ...member, suggestedKeep: index === 0 }));
        }
        return { ...group, members, reclaimableBytes: group.sizeBytes * Math.max(0, members.length - 1) };
      })
      .filter((group) => group.members.length > 1));
    setToast('已加入本机排除规则，后续扫描会在入口处跳过');
  }

  return <div className="app-shell">
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark"><ShieldCheck /></span><div><strong>清盘</strong><small>QINGPAN · SAFE SPACE</small></div></div>
      <div className="nav-label">空间工作台</div>
      <nav className="primary-nav">{primaryNav.map((item) => <button key={item.id} className={page === item.id ? 'active' : ''} onClick={() => setPage(item.id)}><span><item.icon /></span><div><strong>{item.label}</strong><small>{item.description}</small></div></button>)}</nav>
      <div className="nav-label secondary-label">管理</div>
      <nav className="secondary-nav">{secondaryNav.map((item) => <button key={item.id} className={page === item.id ? 'active' : ''} onClick={() => setPage(item.id)}><item.icon /><span>{item.label}</span></button>)}</nav>
      <div className="local-card"><span><ShieldCheck /></span><div><strong>文件信息仅在本机</strong><small>未上传路径与内容哈希</small></div></div>
      <div className="sidebar-foot"><span className="avatar">QP</span><div><strong>专业版</strong><small>安全规则 v2026.07</small></div><Boxes /></div>
    </aside>

    <div className="app-stage">
      <header className="topbar">
        <button className="drive-select" aria-label="选择磁盘"><span><HardDrive /></span><div><small>当前分析磁盘</small><strong>{activeDisk.name} ({activeDisk.mount || '--'})</strong></div><select value={activeDiskId} disabled={anyReadScan || scanning} onChange={(event) => setActiveDiskId(event.target.value)}>{disks.map((disk) => <option key={disk.id} value={disk.id}>{disk.name} ({disk.mount})</option>)}</select><ChevronDown /></button>
        <div className="topbar-status"><span className="live-dot" />保护引擎运行中</div>
        <button className="icon-button top-icon" aria-label="通知"><Bell /></button>
        <button className="basket-button" onClick={() => setBasketOpen(true)}><ShoppingBasket /><span><small>清理篮</small><strong>{selected.size ? `${selected.size} 项 · ${formatBytes(selectedBytes)}` : '尚未选择'}</strong></span>{selected.size > 0 && <b>{selected.size}</b>}</button>
      </header>
      <main className="content">
        {page === 'overview' && <Overview disk={activeDisk} items={cleanupItems} records={operationRecords} lastScanAt={lastScanAt} scanning={scanning} onScan={runScan} onNavigate={setPage} />}
        {page === 'cleanup' && <CleanupCenter items={cleanupItems} selected={selected} scanning={scanning} progress={progress} scanPath={scanPath} onScan={runScan} onToggle={toggleItem} onOpenBasket={() => setBasketOpen(true)} />}
        {page === 'files' && <FileDiscovery largeFiles={discoveryFiles} duplicateGroups={discoveryDuplicates} scanStatus={fileScanStatus} scannedAt={fileScannedAt || undefined} onScan={(tab) => void runFileScan(tab)} onCancel={() => void cancelFileScan()} onRevealInExplorer={(path) => void reveal(path)} onAddExclusion={addExclusion} />}
        {page === 'analysis' && <StorageAnalysis disk={activeDisk} directories={analysisDirectories} categories={analysisCategories} initialPath={activeDisk.mount} scanStatus={analysisScanStatus} scannedAt={analysisScannedAt || undefined} onScan={() => void runStorageScan()} onCancel={() => void cancelStorageScan()} onAnalyzeDirectory={(directory) => void runStorageScan(directory.path, true)} />}
        {page === 'apps' && <AppManagement apps={installedApps} busyAppId={busyAppId} onRequestUninstall={(app) => void uninstall(app.id)} onClearCache={(app) => { setPage('cleanup'); setToast(`请在应用缓存中复核 ${app.name} 的可重建内容`); }} />}
        {page === 'recovery' && <RecoveryCenter records={operationRecords} onRestore={(id) => setToast(`记录 ${id} 没有可直接覆盖恢复的内容`)} />}
        {page === 'settings' && <SettingsPage protectedDirectories={protectedPaths.map((item) => item.path)} exclusionRules={[...builtInExclusionRules, ...userExclusions]} autoCleanupEnabled={false} theme={theme} setTheme={useAppStore.getState().setTheme} />}
      </main>
    </div>

    {basketOpen && <BasketDrawer items={selectedItems} busy={executing} onClose={() => setBasketOpen(false)} onRemove={toggleItem} onExecute={() => setExecuteOpen(true)} />}
    {executeOpen && <Dialog title="确认执行这份清理计划？" danger busy={executing} confirmLabel="复检并执行" onClose={() => setExecuteOpen(false)} onConfirm={runCleanup}><p>将处理 <strong>{selectedItems.length} 个规则类别</strong>，预计释放 <strong>{formatBytes(selectedBytes)}</strong>。只会删除本次扫描快照中未变化的文件。</p><div className="confirm-proof"><span><ShieldCheck /></span><div><strong>默认失败策略：跳过并保留</strong><small>文件已修改、路径变化、被占用或身份不符时不会强制删除。</small></div></div>{irreversibleCount > 0 && <div className="confirm-warning"><ShieldCheck /><span><strong>{irreversibleCount} 项不可恢复内容</strong><small>这些项目不是默认推荐项，请再次核对。</small></span></div>}</Dialog>}
    {toast && <div className="toast" role="status"><ShieldCheck /><span>{toast}</span><button className="icon-button" onClick={() => setToast('')} aria-label="关闭提示"><X /></button></div>}
  </div>;
}
