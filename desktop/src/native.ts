import { invoke } from '@tauri-apps/api/core';
import {
  apps as previewApps,
  cleanupItems as previewCleanupItems,
  directories as previewDirectories,
  disks as previewDisks,
  duplicateGroups as previewDuplicateGroups,
  largeFiles as previewLargeFiles,
  records as previewOperationRecords,
  storageCategories as previewStorageCategories,
} from './mockData';
import type {
  AppEntry,
  CleanupItem,
  DiskInfo,
  DuplicateScanResult,
  ExecuteResult,
  LargeFileScanResult,
  OperationRecord,
  ScanStats,
  StorageAnalysisResult,
  UninstallLaunchResult,
} from './types';

type NativeCleanupItem = Pick<CleanupItem, 'id' | 'category' | 'name' | 'path' | 'description' | 'sizeBytes' | 'risk' | 'deleteMode'>;
type NativeAppEntry = Omit<AppEntry, 'cacheBytes' | 'lastUsed'>;

const APP_ICON_DATA_URL_PREFIX = 'data:image/png;base64,';
const MAX_APP_ICON_DATA_URL_LENGTH = 350_000;
const MAX_CONCURRENT_APP_ICON_REQUESTS = 4;
const MAX_CACHED_APP_ICON_REQUESTS = 256;
const appIconRequests = new Map<string, Promise<string | null>>();
const appIconQueue: Array<() => void> = [];
let activeAppIconRequests = 0;

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

function normalizeCleanupItem(item: NativeCleanupItem): CleanupItem {
  const category = item.category.toLowerCase();
  const classification = `${category} ${item.name.toLowerCase()}`;
  const scope = classification.includes('浏览器') || classification.includes('edge') || classification.includes('chrome') || classification.includes('firefox')
    ? 'browser'
    : category.includes('应用') ? 'apps' : 'system';
  return {
    ...item,
    scope,
    product: scope === 'browser' ? item.name.replace(/缓存.*$/, '').trim() : scope === 'apps' ? item.name : 'Windows',
    reason: item.description || '命中已签名的可重建缓存规则',
    fileCount: 0,
    confidence: 'high',
    impact: 'rebuild',
    recoverability: 'rebuildable',
    selectable: item.risk === 'low',
  };
}

function previewScanStats(scannedFiles: number): ScanStats {
  return {
    scannedFiles,
    skipped: 0,
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

function enqueueAppIconRequest(appId: string): Promise<string | null> {
  return new Promise((resolve, reject) => {
    appIconQueue.push(() => {
      invoke<string | null>('get_app_icon', { id: appId })
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

export async function scanCleanup(): Promise<CleanupItem[]> {
  if (!isNativeRuntime()) return previewCleanupItems;
  const items = await invoke<NativeCleanupItem[]>('scan_cleanup');
  return items.map(normalizeCleanupItem);
}

export async function executeCleanup(itemIds: string[]): Promise<ExecuteResult> {
  if (!isNativeRuntime()) {
    const reclaimedBytes = previewCleanupItems.filter((item) => itemIds.includes(item.id)).reduce((sum, item) => sum + item.sizeBytes, 0);
    return { reclaimedBytes, succeeded: itemIds.length, failed: [] };
  }
  return invoke<ExecuteResult>('execute_cleanup', { request: { itemIds, confirmed: true } });
}

export async function loadApps(): Promise<AppEntry[]> {
  if (!isNativeRuntime()) return previewApps;
  appIconRequests.clear();
  const entries = await invoke<NativeAppEntry[]>('list_apps');
  return entries.map((entry) => ({ ...entry, cacheBytes: 0, lastUsed: '未知' }));
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
  request = enqueueAppIconRequest(appId)
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
