import { cleanupItems as previewCleanupItems } from '../../mockData';
import type {
  CleanupItem,
  CleanupPlan,
  CleanupProgress,
  CleanupProgressHandler,
  CleanupScan,
  ExecuteResult,
} from './types';

const PREVIEW_RULE_VERSION = 'preview-2026.07';
const PREVIEW_TTL_MS = 15 * 60 * 1000;

const scans = new Map<string, CleanupScan>();
const plans = new Map<string, CleanupPlan>();
let idSequence = 0;

function nextId(prefix: string): string {
  idSequence += 1;
  const suffix = typeof globalThis.crypto?.randomUUID === 'function'
    ? globalThis.crypto.randomUUID()
    : `${Date.now()}-${idSequence}`;
  return `${prefix}-${suffix}`;
}

function cloneItem(item: CleanupItem): CleanupItem {
  return { ...item };
}

function clonePlan(plan: CleanupPlan): CleanupPlan {
  return {
    ...plan,
    items: plan.items.map(cloneItem),
    irreversibleItemIds: [...plan.irreversibleItemIds],
  };
}

function pruneExpired(now: number): void {
  for (const [scanId, scan] of scans) {
    if (scan.expiresAtMs <= now) scans.delete(scanId);
  }
  for (const [planId, plan] of plans) {
    if (plan.expiresAtMs <= now) plans.delete(planId);
  }
}

function confirmationsMatch(expectedIds: readonly string[], confirmedIds: readonly string[]): boolean {
  const expected = new Set(expectedIds);
  const confirmed = new Set(confirmedIds);
  return expected.size === expectedIds.length
    && confirmed.size === confirmedIds.length
    && expected.size === confirmed.size
    && [...expected].every((id) => confirmed.has(id));
}

function emitProgress(
  handler: CleanupProgressHandler | undefined,
  progress: CleanupProgress,
): void {
  handler?.(progress);
}

export async function previewScanCleanup(): Promise<CleanupScan> {
  const now = Date.now();
  pruneExpired(now);
  const scan: CleanupScan = {
    scanId: nextId('preview-scan'),
    ruleVersion: PREVIEW_RULE_VERSION,
    expiresAtMs: now + PREVIEW_TTL_MS,
    items: previewCleanupItems.map(cloneItem),
  };
  scans.set(scan.scanId, scan);
  return { ...scan, items: scan.items.map(cloneItem) };
}

export async function previewCreateCleanupPlan(
  scanId: string,
  itemIds: readonly string[],
): Promise<CleanupPlan> {
  const now = Date.now();
  pruneExpired(now);
  const scan = scans.get(scanId);
  if (!scan) throw new Error('扫描快照不存在或已过期，请重新扫描');
  if (itemIds.length < 1 || itemIds.length > 100 || new Set(itemIds).size !== itemIds.length) {
    throw new Error('清理计划必须包含 1 到 100 个不重复条目');
  }

  const indexedItems = new Map(scan.items.map((item) => [item.id, item]));
  const items = itemIds.map((id) => indexedItems.get(id));
  if (items.some((item) => !item)) {
    throw new Error('清理计划包含不属于该扫描快照的条目');
  }
  const selectedItems = items.map((item) => cloneItem(item as CleanupItem));
  const plan: CleanupPlan = {
    planId: nextId('preview-plan'),
    scanId,
    ruleVersion: scan.ruleVersion,
    createdAtMs: now,
    expiresAtMs: Math.min(scan.expiresAtMs, now + PREVIEW_TTL_MS),
    items: selectedItems,
    totalItems: selectedItems.length,
    totalFiles: selectedItems.reduce((sum, item) => sum + item.fileCount, 0),
    totalBytes: selectedItems.reduce((sum, item) => sum + item.sizeBytes, 0),
    irreversibleItemIds: selectedItems
      .filter((item) => item.risk === 'high')
      .map((item) => item.id),
  };
  plans.set(plan.planId, plan);
  return clonePlan(plan);
}

export async function previewExecuteCleanupPlan(
  planId: string,
  confirmedIrreversibleItemIds: readonly string[],
  onProgress?: CleanupProgressHandler,
): Promise<ExecuteResult> {
  const now = Date.now();
  pruneExpired(now);
  const plan = plans.get(planId);
  if (!plan) throw new Error('清理计划不存在、已过期或已经执行');
  if (!confirmationsMatch(plan.irreversibleItemIds, confirmedIrreversibleItemIds)) {
    throw new Error('不可恢复内容确认列表无效');
  }

  // A valid execution request consumes the plan before file operations begin.
  plans.delete(planId);
  let completedFiles = 0;
  let reclaimedBytes = 0;
  let stagedBytes = 0;
  emitProgress(onProgress, {
    phase: 'starting',
    completedItems: 0,
    totalItems: plan.totalItems,
    completedFiles,
    totalFiles: plan.totalFiles,
    currentItemId: '',
    currentItemName: '',
    currentPath: '',
    reclaimedBytes,
    failedFiles: 0,
  });

  plan.items.forEach((item, index) => {
    emitProgress(onProgress, {
      phase: 'running',
      completedItems: index,
      totalItems: plan.totalItems,
      completedFiles,
      totalFiles: plan.totalFiles,
      currentItemId: item.id,
      currentItemName: item.name,
      currentPath: item.path,
      reclaimedBytes,
      failedFiles: 0,
    });
    completedFiles += item.fileCount;
    if (item.deleteMode === 'quarantine') {
      stagedBytes += item.sizeBytes;
    } else {
      reclaimedBytes += item.sizeBytes;
    }
    emitProgress(onProgress, {
      phase: 'item_complete',
      completedItems: index + 1,
      totalItems: plan.totalItems,
      completedFiles,
      totalFiles: plan.totalFiles,
      currentItemId: item.id,
      currentItemName: item.name,
      currentPath: item.path,
      reclaimedBytes,
      failedFiles: 0,
    });
  });

  emitProgress(onProgress, {
    phase: 'complete',
    completedItems: plan.totalItems,
    totalItems: plan.totalItems,
    completedFiles: plan.totalFiles,
    totalFiles: plan.totalFiles,
    currentItemId: '',
    currentItemName: '',
    currentPath: '',
    reclaimedBytes,
    failedFiles: 0,
  });
  return { reclaimedBytes, stagedBytes, succeeded: plan.totalItems, failed: [] };
}

export function resetPreviewCleanupStateForTests(): void {
  scans.clear();
  plans.clear();
  idSequence = 0;
}
