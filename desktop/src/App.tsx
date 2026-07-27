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
import { BasketDrawer, CleanupExecutionSummary, Dialog } from './components';
import {
  createCleanupPlan,
  executeCleanupPlan,
  scanCleanup,
} from './features/cleanup-plan';
import type {
  CleanupItem,
  CleanupPlan,
  CleanupProgress,
  ExecuteResult,
} from './features/cleanup-plan';
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
  deleteLargeFiles,
  isNativeRuntime,
  loadApps,
  loadDisks,
  loadOperationRecords,
  loadPartitionDisks,
  openWindowsDiskManagement,
  requestUninstall,
  revealInExplorer,
  scanDuplicateFiles,
  scanLargeFiles,
  scanStorageUsage,
} from './native';
import AppManagement from './pages/AppManagement';
import CleanupCenter from './pages/CleanupCenter';
import FileDiscovery, { type FileDiscoveryTab } from './pages/FileDiscovery';
import DiskPartition from './pages/DiskPartition';
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
  LargeFileDeleteProgress,
  LargeFileDeleteResult,
  LargeFileEntry,
  OperationRecord,
  Page,
  PartitionDisk,
  StorageCategory,
} from './types';

const primaryNav: Array<{ id: Page; label: string; description: string; icon: typeof LayoutDashboard }> = [
  { id: 'overview', label: '空间概览', description: '磁盘与建议', icon: LayoutDashboard },
  { id: 'cleanup', label: '清理中心', description: '有证据地清理', icon: Sparkles },
  { id: 'files', label: '文件发现', description: '大文件与重复项', icon: Files },
  { id: 'analysis', label: '磁盘地图', description: '空间可视化', icon: ChartNoAxesCombined },
  { id: 'partition', label: '磁盘分区', description: '布局与系统管理', icon: HardDrive },
];

const secondaryNav: Array<{ id: Page; label: string; icon: typeof Package }> = [
  { id: 'apps', label: '应用管理', icon: Package },
  { id: 'recovery', label: '恢复与记录', icon: RotateCcw },
  { id: 'settings', label: '安全设置', icon: Settings },
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

function initialCleanupProgress(items: CleanupItem[]): CleanupProgress {
  return {
    phase: 'starting',
    completedItems: 0,
    totalItems: items.length,
    completedFiles: 0,
    totalFiles: items.reduce((sum, item) => sum + item.fileCount, 0),
    currentItemId: '',
    currentItemName: '',
    currentPath: '',
    reclaimedBytes: 0,
    failedFiles: 0,
  };
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
  const [partitionDisks, setPartitionDisks] = useState<PartitionDisk[]>([]);
  const [partitionLoading, setPartitionLoading] = useState(false);
  const [partitionLoaded, setPartitionLoaded] = useState(false);
  const [partitionError, setPartitionError] = useState('');
  const [fileScanStatus, setFileScanStatus] = useState<TaskStatus>(nativeRuntime ? 'idle' : 'complete');
  const [analysisScanStatus, setAnalysisScanStatus] = useState<TaskStatus>(nativeRuntime ? 'idle' : 'complete');
  const [fileScannedAt, setFileScannedAt] = useState(nativeRuntime ? '' : '演示数据');
  const [analysisScannedAt, setAnalysisScannedAt] = useState(nativeRuntime ? '' : '演示数据');
  const [fileTaskId, setFileTaskId] = useState<string | null>(null);
  const [analysisTaskId, setAnalysisTaskId] = useState<string | null>(null);
  const [userExclusions, setUserExclusions] = useState<string[]>(readUserExclusions);
  const [busyAppId, setBusyAppId] = useState<string | null>(null);
  const [latestCleanupScanId, setLatestCleanupScanId] = useState<string | null>(null);
  const [planningCleanup, setPlanningCleanup] = useState(false);
  const [executeOpen, setExecuteOpen] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [cleanupPlan, setCleanupPlan] = useState<CleanupPlan | null>(null);
  const [cleanupProgress, setCleanupProgress] = useState<CleanupProgress | null>(null);
  const [cleanupResult, setCleanupResult] = useState<ExecuteResult | null>(null);
  const [cleanupError, setCleanupError] = useState('');
  const [cleanupFollowUp, setCleanupFollowUp] = useState('');
  const [irreversibleConfirmed, setIrreversibleConfirmed] = useState(false);
  const [toast, setToast] = useState('');
  const activeDisk = disks.find((disk) => disk.id === activeDiskId) || disks[0] || (nativeRuntime ? unavailableDisk : previewDisks[0]);
  const selectedItems = useMemo(() => cleanupItems.filter((item) => selected.has(item.id)), [cleanupItems, selected]);
  const selectedBytes = selectedItems.reduce((sum, item) => sum + item.sizeBytes, 0);
  const executionPlan = cleanupPlan?.items ?? [];
  const plannedBytes = cleanupPlan?.items
    .filter((item) => item.deleteMode !== 'quarantine')
    .reduce((sum, item) => sum + item.sizeBytes, 0) ?? 0;
  const plannedIrreversibleCount = cleanupPlan?.irreversibleItemIds.length ?? 0;
  const showingExecutionSummary = executing || Boolean(cleanupResult) || Boolean(cleanupError);
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
      void scanCleanup().then((scan) => {
        setLatestCleanupScanId(scan.scanId);
        setCleanupItems(scan.items);
        setSafeDefaults(scan.items);
      });
      setLastScanAt('演示数据 · 尚未执行真实扫描');
    }
  }, [nativeRuntime, setCleanupItems, setDisks, setLastScanAt, setSafeDefaults]);

  useEffect(() => {
    if (!toast) return;
    const timeout = window.setTimeout(() => setToast(''), 4200);
    return () => window.clearTimeout(timeout);
  }, [toast]);

  useEffect(() => {
    if (page === 'partition' && !partitionLoaded && !partitionLoading) {
      void refreshPartitionLayout();
    }
  }, [page, partitionLoaded, partitionLoading]);

  async function refreshPartitionLayout() {
    if (partitionLoading) return;
    setPartitionLoading(true);
    setPartitionError('');
    try {
      setPartitionDisks(await loadPartitionDisks());
    } catch (error) {
      setPartitionError(error instanceof Error ? error.message : '无法读取磁盘分区布局');
    } finally {
      setPartitionLoaded(true);
      setPartitionLoading(false);
    }
  }

  async function openPartitionManager() {
    try {
      await openWindowsDiskManagement();
      setToast('已打开 Windows 磁盘管理');
    } catch (error) {
      setToast(error instanceof Error ? error.message : '无法打开 Windows 磁盘管理');
    }
  }

  async function runScan() {
    if (scanning) return;
    setPage('cleanup');
    setScanning(true);
    setProgress(0);
    setScanPath('正在按签名规则执行只读扫描…');
    clearSelection();
    try {
      const scan = await scanCleanup();
      setLatestCleanupScanId(scan.scanId);
      setCleanupItems(scan.items);
      setSafeDefaults(scan.items);
      setProgress(100);
      setScanPath(`已建立 ${scan.items.length} 项安全快照`);
      setLastScanAt(scanTime());
    } catch (error) {
      setToast(error instanceof Error ? error.message : '扫描失败，请重试');
    } finally {
      setScanning(false);
    }
  }

  async function openExecutionReview() {
    if (!selectedItems.length || !latestCleanupScanId || executing || planningCleanup) return;
    const selectedIds = selectedItems.map((item) => item.id);
    setPlanningCleanup(true);
    setToast('正在校验扫描快照并生成清理计划…');
    try {
      const plan = await createCleanupPlan(latestCleanupScanId, selectedIds);
      setCleanupPlan(plan);
      setCleanupProgress(initialCleanupProgress(plan.items));
      setCleanupResult(null);
      setCleanupError('');
      setCleanupFollowUp('');
      setIrreversibleConfirmed(false);
      setToast('');
      setExecuteOpen(true);
    } catch (error) {
      setToast(error instanceof Error ? error.message : '无法生成清理计划，请重新扫描');
    } finally {
      setPlanningCleanup(false);
    }
  }

  function closeExecution() {
    if (executing) return;
    const followUp = cleanupFollowUp;
    setExecuteOpen(false);
    setIrreversibleConfirmed(false);
    setCleanupPlan(null);
    setCleanupProgress(null);
    setCleanupResult(null);
    setCleanupError('');
    setCleanupFollowUp('');
    if (followUp) setToast(followUp);
  }

  async function runCleanup() {
    if (!cleanupPlan || executing || (plannedIrreversibleCount > 0 && !irreversibleConfirmed)) return;
    const plan = cleanupPlan;
    const ids = plan.items.map((item) => item.id);
    const irreversibleIds = plan.irreversibleItemIds;
    setCleanupProgress(initialCleanupProgress(plan.items));
    setCleanupResult(null);
    setCleanupError('');
    setCleanupFollowUp('');
    setExecuting(true);
    try {
      const result = await executeCleanupPlan(plan.planId, irreversibleIds, setCleanupProgress);
      setCleanupResult(result);
      setCleanupProgress((current) => ({
        ...(current || initialCleanupProgress(plan.items)),
        phase: 'complete',
        completedItems: plan.totalItems,
        completedFiles: current?.totalFiles ?? plan.totalFiles,
        currentItemId: '',
        currentItemName: '',
        currentPath: '',
        reclaimedBytes: result.reclaimedBytes,
        failedFiles: result.failed.length,
      }));
      removeSelected(ids);
      setBasketOpen(false);

      if (nativeRuntime) {
        try {
          const refreshedScan = await scanCleanup();
          setLatestCleanupScanId(refreshedScan.scanId);
          setCleanupItems(refreshedScan.items);
        } catch {
          setCleanupFollowUp('清理已完成，但待清理内容刷新失败，可重新扫描后查看');
        }
      } else {
        setCleanupItems(cleanupItems.filter((item) => !ids.includes(item.id)));
      }
      try {
        setOperationRecords(await loadOperationRecords(50));
      } catch {
        // Cleanup has already succeeded; history can refresh on the next app load.
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : '清理未执行';
      setCleanupError(message);
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
          minSizeBytes: 100 * 1024 ** 2,
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

  async function runLargeFileDelete(
    ids: string[],
    onProgress: (progress: LargeFileDeleteProgress) => void,
  ): Promise<LargeFileDeleteResult> {
    const result = await deleteLargeFiles(ids, onProgress);
    if (result.succeededIds.length) {
      const deleted = new Set(result.succeededIds);
      setDiscoveryFiles((files) => files.filter((file) => !deleted.has(file.id)));
    }
    try {
      setOperationRecords(await loadOperationRecords(50));
    } catch {
      // The delete result is authoritative; history can refresh on the next load.
    }
    return result;
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
        {page === 'cleanup' && <CleanupCenter items={cleanupItems} selected={selected} scanning={scanning} progress={progress} scanPath={scanPath} disk={activeDisk} onScan={runScan} onToggle={toggleItem} onOpenBasket={() => setBasketOpen(true)} onClean={() => void openExecutionReview()} />}
        {page === 'files' && <FileDiscovery largeFiles={discoveryFiles} duplicateGroups={discoveryDuplicates} scanStatus={fileScanStatus} scannedAt={fileScannedAt || undefined} onScan={(tab) => void runFileScan(tab)} onCancel={() => void cancelFileScan()} onDeleteLargeFiles={runLargeFileDelete} onRevealInExplorer={(path) => void reveal(path)} onAddExclusion={addExclusion} />}
        {page === 'analysis' && <StorageAnalysis disk={activeDisk} directories={analysisDirectories} categories={analysisCategories} initialPath={activeDisk.mount} scanStatus={analysisScanStatus} scannedAt={analysisScannedAt || undefined} onScan={() => void runStorageScan()} onCancel={() => void cancelStorageScan()} onAnalyzeDirectory={(directory) => void runStorageScan(directory.path, true)} />}
        {page === 'partition' && <DiskPartition disks={partitionDisks} loading={partitionLoading} error={partitionError} onRefresh={() => void refreshPartitionLayout()} onOpenDiskManagement={() => void openPartitionManager()} />}
        {page === 'apps' && <AppManagement apps={installedApps} busyAppId={busyAppId} onRequestUninstall={(app) => void uninstall(app.id)} onClearCache={(app) => { setPage('cleanup'); setToast(`请在应用缓存中复核 ${app.name} 的可重建内容`); }} />}
        {page === 'recovery' && <RecoveryCenter auditRecords={operationRecords} />}
        {page === 'settings' && <SettingsPage protectedDirectories={protectedPaths.map((item) => item.path)} exclusionRules={[...builtInExclusionRules, ...userExclusions]} autoCleanupEnabled={false} theme={theme} setTheme={useAppStore.getState().setTheme} />}
      </main>
    </div>

    {basketOpen && <BasketDrawer items={selectedItems} busy={executing || planningCleanup} onClose={() => setBasketOpen(false)} onRemove={toggleItem} onExecute={() => void openExecutionReview()} />}
    {executeOpen && <Dialog
      title={executing ? '正在执行清理计划' : cleanupResult ? '清理完成' : cleanupError ? '清理未完成' : '确认执行这份清理计划？'}
      danger
      busy={executing}
      confirmDisabled={plannedIrreversibleCount > 0 && !irreversibleConfirmed}
      confirmLabel={plannedIrreversibleCount > 0 ? '永久删除所选内容' : '复检并执行'}
      hideActions={showingExecutionSummary}
      closeDisabled={executing}
      wide={showingExecutionSummary}
      onClose={closeExecution}
      onConfirm={runCleanup}
    >
      {showingExecutionSummary && cleanupProgress
        ? <CleanupExecutionSummary items={executionPlan} progress={cleanupProgress} result={cleanupResult} error={cleanupError} onDone={closeExecution} />
        : <><p>后端计划包含 <strong>{cleanupPlan?.totalItems ?? 0} 个规则类别</strong>，预计释放 <strong>{formatBytes(plannedBytes)}</strong>。执行时会按该计划保存的扫描快照逐文件复检。</p><div className="confirm-proof"><span><ShieldCheck /></span><div><strong>默认失败策略：跳过并保留</strong><small>文件已修改、路径变化、被占用或身份不符时不会强制删除。</small></div></div>{plannedIrreversibleCount > 0 && <><div className="confirm-warning"><ShieldCheck /><span><strong>{plannedIrreversibleCount} 项不可恢复内容</strong><small>包含微信聊天或媒体数据，执行后无法找回。</small></span></div><label className="irreversible-confirmation"><input type="checkbox" checked={irreversibleConfirmed} onChange={(event) => setIrreversibleConfirmed(event.target.checked)} /><span>我确认永久删除所选微信用户数据</span></label></>}</>}
    </Dialog>}
    {toast && <div className="toast" role="status"><ShieldCheck /><span>{toast}</span><button className="icon-button" onClick={() => setToast('')} aria-label="关闭提示"><X /></button></div>}
  </div>;
}
