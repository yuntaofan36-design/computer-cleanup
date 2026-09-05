import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  apps as previewApps,
  directories as previewDirectories,
  disks as previewDisks,
  duplicateGroups as previewDuplicateGroups,
  largeFiles as previewLargeFiles,
  partitionDisks as previewPartitionDisks,
  records as previewOperationRecords,
  startups as previewStartups,
  storageCategories as previewStorageCategories,
} from './mockData';
export {
  createCleanupPlan,
  executeCleanupPlan,
  inferCleanupScope,
  scanCleanup,
} from './features/cleanup-plan';
import type {
  AppEntry,
  DiskInfo,
  DuplicateScanResult,
  LargeFileDeleteProgress,
  LargeFileDeleteResult,
  LargeFileScanResult,
  OperationRecord,
  PartitionDisk,
  ScanStats,
  StartupEntry,
  StorageAnalysisResult,
  UninstallLaunchResult,
} from './types';

type NativeAppEntry = Omit<AppEntry, 'cacheBytes' | 'lastUsed'>;

const APP_ICON_DATA_URL_PREFIX = 'data:image/png;base64,';
const MAX_APP_ICON_DATA_URL_LENGTH = 350_000;
const MAX_CONCURRENT_APP_ICON_REQUESTS = 4;
const MAX_CACHED_APP_ICON_REQUESTS = 256;
const appIconRequests = new Map<string, Promise<string | null>>();
const startupIconRequests = new Map<string, Promise<string | null>>();
const appIconQueue: Array<() => void> = [];
let activeAppIconRequests = 0;
let previewStartupEntries = previewStartups.map((entry) => ({ ...entry }));

export interface StorageScanOptions {
  maxFiles?: number;
  maxResults?: number;
  excludedPaths?: string[];
}

export interface LargeFileScanOptions {
  minSizeBytes?: number;
  maxFiles?: number;
  maxResults?: number;
  excludedPaths?: string[];
}

export interface DuplicateFileScanOptions {
  minSizeBytes?: number;
  maxFiles?: number;
  maxGroups?: number;
  maxMembers?: number;
  sampleBytes?: number;
  excludedPaths?: string[];
}

interface NativeScanRequest<TOptions> {
  taskId: string;
  root: string;
  options: TOptions;
}

type NativeOperationKind = 'cleanup' | 'quarantine' | 'uninstall' | 'restoreExport';
type NativeOperationStatus = 'succeeded' | 'partiallySucceeded' | 'failed' | 'skipped' | 'cancelled';

interface NativeOperationDetail {
  itemId: string;
  path?: string;
  bytes: number;
  detail: string;
}

interface NativeOperationRecord {
  schemaVersion: number;
  operationId: string;
  kind: NativeOperationKind;
  status: NativeOperationStatus;
  ruleVersion: string;
  startedAtMs: number;
  completedAtMs: number;
  reclaimedBytes: number;
  stagedBytes: number;
  succeeded: NativeOperationDetail[];
  failed: NativeOperationDetail[];
  skipped: NativeOperationDetail[];
}

interface NativeRecentRecords {
  records: NativeOperationRecord[];
  skippedCorruptLines: number;
}

type NativeOperationRecordsResponse = NativeOperationRecord[] | NativeRecentRecords;

const operationKindMap: Record<NativeOperationKind, OperationRecord['kind']> = {
  cleanup: 'cleanup',
  quarantine: 'cleanup',
  uninstall: 'uninstall',
  restoreExport: 'restore',
};

const operationTitleMap: Record<NativeOperationKind, string> = {
  cleanup: '安全清理',
  quarantine: '隔离清理',
  uninstall: '应用卸载',
  restoreExport: '恢复导出',
};

const operationStatusMap: Record<NativeOperationStatus, OperationRecord['status']> = {
  succeeded: 'success',
  partiallySucceeded: 'partial',
  failed: 'failed',
  skipped: 'partial',
  cancelled: 'partial',
};

export const isNativeRuntime = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

function previewScanStats(scannedFiles: number): ScanStats {
  return {
    scannedFiles,
    skipped: 0,
    deduplicatedHardLinks: 0,
    cancelled: false,
    limitReached: false,
  };
}

function formatOperationTime(timestampMs: number): string {
  if (!Number.isFinite(timestampMs) || timestampMs <= 0) return '时间未知';
  return new Date(timestampMs).toLocaleString('zh-CN', { hour12: false });
}

function operationDetail(record: NativeOperationRecord): string {
  const parts: string[] = [];
  if (record.succeeded.length > 0) parts.push(`${record.succeeded.length} 项成功`);
  if (record.skipped.length > 0) parts.push(`${record.skipped.length} 项跳过`);
  if (record.failed.length > 0) parts.push(`${record.failed.length} 项失败`);

  const issue = record.failed[0] ?? record.skipped[0];
  if (issue?.detail) parts.push(issue.detail);
  return parts.length > 0 ? parts.join('，') : `规则 ${record.ruleVersion}`;
}

function normalizeOperationRecord(record: NativeOperationRecord): OperationRecord {
  return {
    id: record.operationId,
    kind: operationKindMap[record.kind],
    title: operationTitleMap[record.kind],
    createdAt: formatOperationTime(record.completedAtMs || record.startedAtMs),
    reclaimedBytes: record.reclaimedBytes,
    stagedBytes: record.stagedBytes,
    status: operationStatusMap[record.status],
    detail: operationDetail(record),
  };
}

function pumpAppIconQueue(): void {
  while (activeAppIconRequests < MAX_CONCURRENT_APP_ICON_REQUESTS) {
    const startRequest = appIconQueue.shift();
    if (!startRequest) return;
    activeAppIconRequests += 1;
    startRequest();
  }
}

function enqueueIconRequest(command: 'get_app_icon' | 'get_startup_icon', id: string): Promise<string | null> {
  return new Promise((resolve, reject) => {
    appIconQueue.push(() => {
      invoke<string | null>(command, { id })
        .then(resolve, reject)
        .finally(() => {
          activeAppIconRequests -= 1;
          pumpAppIconQueue();
        });
    });
    pumpAppIconQueue();
  });
}

function normalizeAppIconDataUrl(value: string | null): string | null {
  if (
    typeof value !== 'string'
    || !value.startsWith(APP_ICON_DATA_URL_PREFIX)
    || value.length > MAX_APP_ICON_DATA_URL_LENGTH
  ) {
    return null;
  }
  return value;
}

export async function loadDisks(): Promise<DiskInfo[]> {
  if (!isNativeRuntime()) return previewDisks;
  return invoke<DiskInfo[]>('list_disks');
}

export async function loadProtectedDirectories(): Promise<string[]> {
  if (!isNativeRuntime()) return [];
  return invoke<string[]>('list_protected_directories');
}

export async function loadPartitionDisks(): Promise<PartitionDisk[]> {
  if (!isNativeRuntime()) return previewPartitionDisks;
  return invoke<PartitionDisk[]>('list_partition_disks');
}

export async function openWindowsDiskManagement(): Promise<void> {
  if (!isNativeRuntime()) throw new Error('浏览器预览不会启动 Windows 磁盘管理');
  return invoke<void>('open_windows_disk_management');
}

export async function loadApps(): Promise<AppEntry[]> {
  if (!isNativeRuntime()) return previewApps;
  appIconRequests.clear();
  const entries = await invoke<NativeAppEntry[]>('list_apps');
  return entries.map((entry) => ({ ...entry, cacheBytes: 0, lastUsed: '未知' }));
}

export async function loadStartupEntries(): Promise<StartupEntry[]> {
  if (!isNativeRuntime()) return previewStartupEntries.map((entry) => ({ ...entry }));
  startupIconRequests.clear();
  return invoke<StartupEntry[]>('list_startup_entries');
}

export async function setStartupEntryEnabled(id: string, enabled: boolean): Promise<void> {
  if (!id || id !== id.trim()) throw new Error('启动项标识无效');
  if (isNativeRuntime()) {
    return invoke<void>('set_startup_enabled', { id, enabled, confirmed: true });
  }

  const index = previewStartupEntries.findIndex((entry) => entry.id === id);
  if (index < 0) throw new Error('启动项不存在或列表已刷新');
  previewStartupEntries = previewStartupEntries.map((entry, entryIndex) => (
    entryIndex === index ? { ...entry, enabled } : entry
  ));
}

export function loadAppIcon(appId: string): Promise<string | null> {
  if (!isNativeRuntime() || !appId) return Promise.resolve(null);

  const cachedRequest = appIconRequests.get(appId);
  if (cachedRequest) {
    appIconRequests.delete(appId);
    appIconRequests.set(appId, cachedRequest);
    return cachedRequest;
  }

  let request: Promise<string | null>;
  request = enqueueIconRequest('get_app_icon', appId)
    .then(normalizeAppIconDataUrl)
    .then((dataUrl) => {
      if (dataUrl === null && appIconRequests.get(appId) === request) {
        appIconRequests.delete(appId);
      }
      return dataUrl;
    });
  appIconRequests.set(appId, request);
  while (appIconRequests.size > MAX_CACHED_APP_ICON_REQUESTS) {
    const oldestAppId = appIconRequests.keys().next().value;
    if (typeof oldestAppId !== 'string') break;
    appIconRequests.delete(oldestAppId);
  }
  void request.catch(() => {
    if (appIconRequests.get(appId) === request) appIconRequests.delete(appId);
  });
  return request;
}

export function loadStartupIcon(startupId: string): Promise<string | null> {
  if (!isNativeRuntime() || !startupId) return Promise.resolve(null);

  const cachedRequest = startupIconRequests.get(startupId);
  if (cachedRequest) {
    startupIconRequests.delete(startupId);
    startupIconRequests.set(startupId, cachedRequest);
    return cachedRequest;
  }

  let request: Promise<string | null>;
  request = enqueueIconRequest('get_startup_icon', startupId)
    .then(normalizeAppIconDataUrl)
    .then((dataUrl) => {
      if (dataUrl === null && startupIconRequests.get(startupId) === request) {
        startupIconRequests.delete(startupId);
      }
      return dataUrl;
    });
  startupIconRequests.set(startupId, request);
  while (startupIconRequests.size > MAX_CACHED_APP_ICON_REQUESTS) {
    const oldestStartupId = startupIconRequests.keys().next().value;
    if (typeof oldestStartupId !== 'string') break;
    startupIconRequests.delete(oldestStartupId);
  }
  void request.catch(() => {
    if (startupIconRequests.get(startupId) === request) startupIconRequests.delete(startupId);
  });
  return request;
}

export async function scanStorageUsage(
  taskId: string,
  root: string,
  options?: StorageScanOptions,
): Promise<StorageAnalysisResult> {
  if (!isNativeRuntime()) {
    const scannedFiles = previewDirectories.reduce((total, directory) => total + directory.fileCount, 0);
    return {
      directories: previewDirectories,
      categories: previewStorageCategories,
      ...previewScanStats(scannedFiles),
    };
  }
  const request: NativeScanRequest<StorageScanOptions> = { taskId, root, options: options ?? {} };
  return invoke<StorageAnalysisResult>('analyze_storage', { request });
}

export async function scanLargeFiles(
  taskId: string,
  root: string,
  options?: LargeFileScanOptions,
): Promise<LargeFileScanResult> {
  if (!isNativeRuntime()) {
    return {
      files: previewLargeFiles,
      ...previewScanStats(previewLargeFiles.length),
    };
  }
  const request: NativeScanRequest<LargeFileScanOptions> = { taskId, root, options: options ?? {} };
  return invoke<LargeFileScanResult>('scan_large_files', { request });
}

export async function deleteLargeFiles(
  itemIds: string[],
  onProgress?: (progress: LargeFileDeleteProgress) => void,
): Promise<LargeFileDeleteResult> {
  if (!isNativeRuntime()) {
    let deletedBytes = 0;
    const succeededIds: string[] = [];
    const failed: LargeFileDeleteResult['failed'] = [];
    onProgress?.({
      phase: 'starting',
      completed: 0,
      total: itemIds.length,
      currentItemId: '',
      currentName: '',
      currentPath: '',
      deletedBytes,
      failed: 0,
    });
    for (const [index, id] of itemIds.entries()) {
      const item = previewLargeFiles.find((entry) => entry.id === id);
      onProgress?.({
        phase: 'running',
        completed: index,
        total: itemIds.length,
        currentItemId: id,
        currentName: item?.name || '未知文件',
        currentPath: item?.path || '',
        deletedBytes,
        failed: failed.length,
      });
      if (onProgress) await new Promise((resolve) => window.setTimeout(resolve, 180));
      if (!item || item.sensitivity === 'protected') {
        failed.push({ id, error: item ? '受保护文件已安全保留' : '条目不属于最近一次扫描' });
      } else {
        deletedBytes += item.sizeBytes;
        succeededIds.push(id);
      }
      onProgress?.({
        phase: 'item_complete',
        completed: index + 1,
        total: itemIds.length,
        currentItemId: id,
        currentName: item?.name || '未知文件',
        currentPath: item?.path || '',
        deletedBytes,
        failed: failed.length,
      });
    }
    onProgress?.({
      phase: 'complete',
      completed: itemIds.length,
      total: itemIds.length,
      currentItemId: '',
      currentName: '',
      currentPath: '',
      deletedBytes,
      failed: failed.length,
    });
    return { deletedBytes, succeededIds, failed };
  }

  const unlisten = onProgress
    ? await listen<LargeFileDeleteProgress>(
      'large-file-delete-progress',
      (event) => onProgress(event.payload),
    )
    : undefined;
  try {
    return await invoke<LargeFileDeleteResult>('delete_large_files', {
      request: { itemIds, confirmedPermanent: true },
    });
  } finally {
    unlisten?.();
  }
}

export async function scanDuplicateFiles(
  taskId: string,
  root: string,
  options?: DuplicateFileScanOptions,
): Promise<DuplicateScanResult> {
  if (!isNativeRuntime()) {
    const scannedFiles = previewDuplicateGroups.reduce((total, group) => total + group.members.length, 0);
    return {
      groups: previewDuplicateGroups,
      ...previewScanStats(scannedFiles),
    };
  }
  const request: NativeScanRequest<DuplicateFileScanOptions> = { taskId, root, options: options ?? {} };
  return invoke<DuplicateScanResult>('scan_duplicate_files', { request });
}

export async function cancelNativeTask(taskId: string): Promise<void> {
  if (!isNativeRuntime()) return;
  return invoke<void>('cancel_task', { taskId });
}

export async function loadOperationRecords(limit: number): Promise<OperationRecord[]> {
  if (!isNativeRuntime()) return previewOperationRecords.slice(0, Math.max(0, Math.trunc(limit)));
  const response = await invoke<NativeOperationRecordsResponse>('list_operation_records', { limit });
  const records = Array.isArray(response) ? response : response.records;
  return records.map(normalizeOperationRecord);
}

export async function revealInExplorer(path: string): Promise<void> {
  if (!isNativeRuntime()) return;
  return invoke<void>('reveal_in_explorer', { path });
}

export async function requestUninstall(id: string): Promise<UninstallLaunchResult> {
  if (!isNativeRuntime()) throw new Error('浏览器预览不会启动系统卸载器');
  return invoke<UninstallLaunchResult>('uninstall_app', { id, confirmed: true });
}
