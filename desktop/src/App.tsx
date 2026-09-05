import { useEffect, useMemo, useState } from 'react';
import {
  Grid2X2,
  HardDrive,
  LayoutDashboard,
  Monitor,
  Moon,
  Package,
  RotateCcw,
  Settings,
  ShieldCheck,
  ShoppingBasket,
  Sparkles,
  Sun,
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
  loadProtectedDirectories,
  loadStartupEntries,
  openWindowsDiskManagement,
  requestUninstall,
  revealInExplorer,
  scanDuplicateFiles,
  scanLargeFiles,
  scanStorageUsage,
  setStartupEntryEnabled,
} from './native';
import SelectMenu from './SelectMenu';
import AppManagement from './pages/AppManagement';
import CleanupCenter from './pages/CleanupCenter';
import FileDiscovery, { type FileDiscoveryTab } from './pages/FileDiscovery';
import DiskPartition from './pages/DiskPartition';
import Overview from './pages/Overview';
import RecoveryCenter from './pages/RecoveryCenter';
import SettingsPage from './pages/SettingsPage';
import StorageAnalysis from './pages/StorageAnalysis';
import StartupManager from './pages/StartupManager';
import Toolbox from './pages/Toolbox';
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
  StartupEntry,
} from './types';

const primaryNav: Array<{ id: Page; label: string; icon: typeof LayoutDashboard }> = [
  { id: 'overview', label: '概览', icon: LayoutDashboard },
  { id: 'cleanup', label: '清理', icon: Sparkles },
  { id: 'tools', label: '工具箱', icon: Grid2X2 },
];

const secondaryNav: Array<{ id: Page; label: string; icon: typeof Package }> = [
  { id: 'recovery', label: '恢复', icon: RotateCcw },
  { id: 'settings', label: '设置', icon: Settings },
];

const toolPages = new Set<Page>(['tools', 'files', 'analysis', 'startup', 'partition', 'apps']);

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

function pathsOverlap(left: string, right: string): boolean {
  return pathIsInside(left, right) || pathIsInside(right, left);
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
    page, setPage, theme, setTheme, disks, setDisks, activeDiskId, setActiveDiskId,
    cleanupItems, setCleanupItems, selected, toggleItem, setSafeDefaults, clearSelection, removeSelected,
    scanning, setScanning, progress, setProgress, scanPath, setScanPath, lastScanAt, setLastScanAt,
    basketOpen, setBasketOpen,
  } = useAppStore();
  const nativeRuntime = isNativeRuntime();
  const [installedApps, setInstalledApps] = useState<AppEntry[]>([]);
  const [startupEntries, setStartupEntries] = useState<StartupEntry[]>([]);
  const [operationRecords, setOperationRecords] = useState<OperationRecord[]>(nativeRuntime ? [] : previewRecords);
  const [discoveryFiles, setDiscoveryFiles] = useState<LargeFileEntry[]>(nativeRuntime ? [] : previewLargeFiles);
  const [discoveryDuplicates, setDiscoveryDuplicates] = useState<DuplicateGroup[]>(nativeRuntime ? [] : previewDuplicateGroups);
  const [analysisDirectories, setAnalysisDirectories] = useState<DirectoryUsage[]>(nativeRuntime ? [] : previewDirectories);
  const [analysisCategories, setAnalysisCategories] = useState<StorageCategory[]>(nativeRuntime ? [] : previewStorageCategories);
  const [partitionDisks, setPartitionDisks] = useState<PartitionDisk[]>([]);
  const [partitionLoading, setPartitionLoading] = useState(false);
  const [partitionLoaded, setPartitionLoaded] = useState(false);
  const [partitionError, setPartitionError] = useState('');
  const [protectedDirectories, setProtectedDirectories] = useState<string[]>([]);
  const [fileScanStatus, setFileScanStatus] = useState<TaskStatus>(nativeRuntime ? 'idle' : 'complete');
  const [analysisScanStatus, setAnalysisScanStatus] = useState<TaskStatus>(nativeRuntime ? 'idle' : 'complete');
  const [fileScannedAt, setFileScannedAt] = useState(nativeRuntime ? '' : '演示数据');
  const [analysisScannedAt, setAnalysisScannedAt] = useState(nativeRuntime ? '' : '演示数据');
  const [fileDiscoveryTab, setFileDiscoveryTab] = useState<FileDiscoveryTab>('large-files');
  const [fileTaskId, setFileTaskId] = useState<string | null>(null);
  const [analysisTaskId, setAnalysisTaskId] = useState<string | null>(null);
  const [userExclusions, setUserExclusions] = useState<string[]>(readUserExclusions);
  const [busyAppId, setBusyAppId] = useState<string | null>(null);
  const [busyStartupId, setBusyStartupId] = useState<string | null>(null);
  const [startupError, setStartupError] = useState('');
  const [latestCleanupScanId, setLatestCleanupScanId] = useState<string | null>(null);
  const [cleanupScanTaskId, setCleanupScanTaskId] = useState<string | null>(null);
  const [cleanupScanCancelling, setCleanupScanCancelling] = useState(false);
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

  function openFileDiscovery(tab: FileDiscoveryTab) {
    setFileDiscoveryTab(tab);
    setPage('files');
  }

  function cycleTheme() {
    setTheme(theme === 'system' ? 'light' : theme === 'light' ? 'dark' : 'system');
  }

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem('luminaTheme', theme);
  }, [theme]);

  useEffect(() => {
    window.scrollTo({ top: 0, left: 0, behavior: 'auto' });
  }, [page]);

  useEffect(() => {
    loadDisks().then(setDisks).catch((error) => setToast(error instanceof Error ? error.message : '无法读取磁盘'));
    loadProtectedDirectories().then(setProtectedDirectories).catch((error) => setToast(error instanceof Error ? error.message : '无法读取受保护目录'));
    loadApps().then(setInstalledApps).catch((error) => setToast(error instanceof Error ? error.message : '无法读取应用清单'));
    loadStartupEntries().then(setStartupEntries).catch((error) => setStartupError(error instanceof Error ? error.message : '无法读取启动项'));
    loadOperationRecords(50).then(setOperationRecords).catch(() => setOperationRecords(nativeRuntime ? [] : previewRecords));
    if (!nativeRuntime) {
      void scanCleanup(crypto.randomUUID()).then((scan) => {
        const visibleItems = scan.items.filter((item) => !userExclusions.some((path) => pathsOverlap(item.path, path)));
        setLatestCleanupScanId(scan.scanId);
        setCleanupItems(visibleItems);
        setSafeDefaults(visibleItems);
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
    const taskId = crypto.randomUUID();
    setPage('cleanup');
    setCleanupScanTaskId(taskId);
    setCleanupScanCancelling(false);
    setScanning(true);
    setProgress(0);
    setScanPath('正在按签名规则执行只读扫描…');
    clearSelection();
    try {
      const scan = await scanCleanup(taskId);
      const visibleItems = scan.items.filter((item) => !userExclusions.some((path) => pathsOverlap(item.path, path)));
      setLatestCleanupScanId(scan.scanId);
      setCleanupItems(visibleItems);
      setSafeDefaults(visibleItems);
      setProgress(100);
      setScanPath(`已建立 ${visibleItems.length} 项安全快照`);
      setLastScanAt(scanTime());
    } catch (error) {
      const message = error instanceof Error
        ? error.message
        : typeof error === 'string'
          ? error
          : '扫描失败，请重试';
      if (message.includes('清理扫描已取消')) {
        setScanPath('扫描已终止，保留上次结果');
        setToast('扫描已终止，保留上次结果');
      } else {
        setToast(message);
      }
    } finally {
      setCleanupScanTaskId(null);
      setCleanupScanCancelling(false);
      setScanning(false);
    }
  }

  async function cancelCleanupScan() {
    if (!cleanupScanTaskId || cleanupScanCancelling) return;
    setCleanupScanCancelling(true);
    setScanPath('正在安全终止扫描…');
    try {
      await cancelNativeTask(cleanupScanTaskId);
    } catch (error) {
      setCleanupScanCancelling(false);
      setToast(error instanceof Error ? error.message : '无法终止扫描，请稍后重试');
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
          const refreshedScan = await scanCleanup(crypto.randomUUID());
          const visibleItems = refreshedScan.items.filter((item) => !userExclusions.some((path) => pathsOverlap(item.path, path)));
          setLatestCleanupScanId(refreshedScan.scanId);
          setCleanupItems(visibleItems);
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

  async function refreshStartupEntries() {
    setStartupError('');
    try {
      setStartupEntries(await loadStartupEntries());
    } catch (error) {
      const message = error instanceof Error ? error.message : '无法读取启动项';
      setStartupError(message);
      throw error;
    }
  }

  async function toggleStartupEntry(id: string, enabled: boolean) {
    if (busyStartupId) return;
    setBusyStartupId(id);
    setStartupError('');
    try {
      await setStartupEntryEnabled(id, enabled);
      setStartupEntries((entries) => entries.map((entry) => entry.id === id ? { ...entry, enabled } : entry));
      setToast(enabled ? '启动项已启用' : '启动项已禁用');
    } catch (error) {
      const message = error instanceof Error ? error.message : '无法更新启动项';
      setStartupError(message);
      throw error;
    } finally {
      setBusyStartupId(null);
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
    setCleanupItems(cleanupItems.filter((item) => !pathsOverlap(item.path, path)));
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
    setAnalysisDirectories((directories) => directories.filter((directory) => !pathIsInside(directory.path, path)));
    setToast('已加入本机排除规则，后续扫描会在入口处跳过');
  }

  function removeExclusion(path: string) {
    const next = userExclusions.filter((entry) => entry !== path);
    setUserExclusions(next);
    localStorage.setItem(exclusionsKey, JSON.stringify(next));
    setToast('已移除排除路径；重新扫描后会更新结果');
  }

  const currentPageLabel: Record<Page, string> = {
    overview: '空间概览',
    cleanup: '清理中心',
    tools: '工具箱',
    files: '文件发现',
    analysis: '磁盘分析',
    startup: '启动项管理',
    partition: '磁盘分区',
    apps: '应用管理',
    recovery: '恢复中心',
    settings: '设置',
  };
  const ThemeIcon = theme === 'dark' ? Moon : theme === 'light' ? Sun : Monitor;
  const themeLabel = theme === 'dark' ? '深色' : theme === 'light' ? '浅色' : '跟随系统';

  return <div className="app-shell">
    <aside className="sidebar">
      <button className="brand" type="button" onClick={() => setPage('overview')} aria-label="返回 Lumina Clean 首页">
        <span className="brand-mark"><Sparkles /></span>
        <span className="brand-name"><strong>Lumina</strong><small>CLEAN</small></span>
      </button>
      <nav className="primary-nav" aria-label="主要导航">{primaryNav.map((item) => {
        const active = item.id === 'tools' ? toolPages.has(page) : page === item.id;
        return <button key={item.id} className={active ? 'active' : ''} onClick={() => setPage(item.id)} title={item.label}><span><item.icon /></span><strong>{item.label}</strong></button>;
      })}</nav>
      <nav className="secondary-nav" aria-label="辅助导航">{secondaryNav.map((item) => <button key={item.id} className={page === item.id ? 'active' : ''} onClick={() => setPage(item.id)} title={item.label}><item.icon /><strong>{item.label}</strong></button>)}</nav>
      <div className="sidebar-foot"><ShieldCheck /><span><strong>本地模式</strong><small>规则 v2026.07</small></span></div>
    </aside>

    <div className="app-stage">
      <header className="topbar">
        <div className="page-identity"><small>Lumina Clean</small><strong>{currentPageLabel[page]}</strong></div>
        <SelectMenu className="drive-select" ariaLabel="选择磁盘" label="当前磁盘" leading={<HardDrive />} value={activeDiskId} options={disks.map((disk) => ({ value: disk.id, label: `${disk.name} (${disk.mount})` }))} disabled={anyReadScan || scanning || !disks.length} placeholder={`${activeDisk.name} (${activeDisk.mount || '--'})`} onChange={setActiveDiskId} />
        <div className="topbar-status"><span className="live-dot" />仅在本机运行</div>
        <button className="icon-button top-icon" type="button" onClick={cycleTheme} aria-label={`当前${themeLabel}，切换主题`} title={`主题：${themeLabel}`}><ThemeIcon /></button>
        <button className="basket-button" type="button" onClick={() => setBasketOpen(true)} aria-label={`清理篮，${selected.size} 项`}><ShoppingBasket /><span><small>清理篮</small><strong>{selected.size ? `${selected.size} 项 · ${formatBytes(selectedBytes)}` : '未选择'}</strong></span>{selected.size > 0 && <b>{selected.size}</b>}</button>
      </header>
      <main className="content">
        {page === 'overview' && <Overview disk={activeDisk} items={cleanupItems} records={operationRecords} lastScanAt={lastScanAt} scanning={scanning} onScan={runScan} onNavigate={setPage} />}
        {page === 'cleanup' && <CleanupCenter items={cleanupItems} selected={selected} scanning={scanning} scanCancelling={cleanupScanCancelling} progress={progress} scanPath={scanPath} disk={activeDisk} onScan={runScan} onCancelScan={() => void cancelCleanupScan()} onToggle={toggleItem} onOpenBasket={() => setBasketOpen(true)} onClean={() => void openExecutionReview()} />}
        {page === 'tools' && <Toolbox largeFileCount={discoveryFiles.length} duplicateGroupCount={discoveryDuplicates.length} appCount={installedApps.length} startupCount={startupEntries.length} analyzedDirectoryCount={analysisDirectories.length} onOpenFileDiscovery={openFileDiscovery} onNavigate={setPage} />}
        {page === 'files' && <FileDiscovery initialTab={fileDiscoveryTab} largeFiles={discoveryFiles} duplicateGroups={discoveryDuplicates} scanStatus={fileScanStatus} scannedAt={fileScannedAt || undefined} onScan={(tab) => void runFileScan(tab)} onCancel={() => void cancelFileScan()} onDeleteLargeFiles={runLargeFileDelete} onRevealInExplorer={(path) => void reveal(path)} onAddExclusion={addExclusion} />}
        {page === 'analysis' && <StorageAnalysis disk={activeDisk} directories={analysisDirectories} categories={analysisCategories} initialPath={activeDisk.mount} scanStatus={analysisScanStatus} scannedAt={analysisScannedAt || undefined} onScan={() => void runStorageScan()} onCancel={() => void cancelStorageScan()} onAnalyzeDirectory={(directory) => void runStorageScan(directory.path, true)} />}
        {page === 'startup' && <StartupManager entries={startupEntries} busyId={busyStartupId} error={startupError} onToggle={toggleStartupEntry} onRefresh={refreshStartupEntries} />}
        {page === 'partition' && <DiskPartition disks={partitionDisks} loading={partitionLoading} error={partitionError} onRefresh={() => void refreshPartitionLayout()} onOpenDiskManagement={() => void openPartitionManager()} />}
        {page === 'apps' && <AppManagement apps={installedApps} busyAppId={busyAppId} onRequestUninstall={(app) => void uninstall(app.id)} onClearCache={(app) => { setPage('cleanup'); setToast(`请在应用缓存中复核 ${app.name} 的可重建内容`); }} />}
        {page === 'recovery' && <RecoveryCenter auditRecords={operationRecords} />}
        {page === 'settings' && <SettingsPage protectedDirectories={protectedDirectories} builtInExclusionRules={builtInExclusionRules} userExclusions={userExclusions} onAddExclusion={addExclusion} onRemoveExclusion={removeExclusion} theme={theme} setTheme={setTheme} />}
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
        ? <CleanupExecutionSummary items={executionPlan} progress={cleanupProgress} result={cleanupResult} error={cleanupError} onDone={() => { closeExecution(); setPage('overview'); }} />
        : <><p>后端计划包含 <strong>{cleanupPlan?.totalItems ?? 0} 个规则类别</strong>，预计释放 <strong>{formatBytes(plannedBytes)}</strong>。执行时会按该计划保存的扫描快照逐文件复检。</p><div className="confirm-proof"><span><ShieldCheck /></span><div><strong>默认失败策略：跳过并保留</strong><small>文件已修改、路径变化、被占用或身份不符时不会强制删除。</small></div></div>{plannedIrreversibleCount > 0 && <><div className="confirm-warning"><ShieldCheck /><span><strong>{plannedIrreversibleCount} 项不可恢复内容</strong><small>包含微信聊天或媒体数据，执行后无法找回。</small></span></div><label className="irreversible-confirmation"><input type="checkbox" checked={irreversibleConfirmed} onChange={(event) => setIrreversibleConfirmed(event.target.checked)} /><span>我确认永久删除所选微信用户数据</span></label></>}</>}
    </Dialog>}
    {toast && <div className="toast" role="status"><ShieldCheck /><span>{toast}</span><button className="icon-button" onClick={() => setToast('')} aria-label="关闭提示"><X /></button></div>}
  </div>;
}
