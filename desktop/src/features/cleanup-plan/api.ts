import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  previewCreateCleanupPlan,
  previewExecuteCleanupPlan,
  previewScanCleanup,
} from './previewAdapter';
import type {
  CleanupItem,
  CleanupPlan,
  CleanupProgress,
  CleanupProgressHandler,
  CleanupScan,
  CleanupScope,
  ExecuteResult,
  NativeCleanupItem,
  NativeCleanupPlan,
  NativeCleanupScan,
} from './types';

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function inferCleanupScope(categoryValue: string, nameValue: string): CleanupScope {
  const category = categoryValue.toLowerCase();
  const classification = `${category} ${nameValue.toLowerCase()}`;
  if (
    classification.includes('微信')
    || classification.includes('wechat')
    || classification.includes('weixin')
    || classification.includes('xwechat')
  ) return 'wechat';
  if (
    classification.includes('浏览器')
    || classification.includes('edge')
    || classification.includes('chrome')
    || classification.includes('firefox')
  ) return 'browser';
  return category.includes('应用') ? 'apps' : 'system';
}

function normalizeCleanupItem(item: NativeCleanupItem): CleanupItem {
  const scope = inferCleanupScope(item.category, item.name);
  const isWechatUserData = scope === 'wechat' && item.risk === 'high';
  const isQuarantined = item.deleteMode === 'quarantine';
  const blocked = Boolean(item.blockedReason);
  return {
    ...item,
    fileCount: item.fileCount ?? 0,
    scope,
    product: scope === 'browser'
      ? item.name.split('·')[0]?.trim() || item.name
      : scope === 'apps' ? item.name : scope === 'wechat' ? '微信' : 'Windows',
    reason: item.blockedReason || (isWechatUserData
      ? '命中微信文档根下的明确用户数据目录；默认不勾选，只有主动选择并确认后才会处理'
      : scope === 'wechat'
      ? '仅命中微信 AppData 下明确的缓存、日志或崩溃报告叶子目录，不进入聊天数据目录'
      : item.description || '命中已签名的可重建缓存规则'),
    confidence: 'high',
    impact: isWechatUserData ? 'user_data' : 'rebuild',
    recoverability: isWechatUserData
      ? 'irreversible'
      : isQuarantined ? 'recoverable' : 'rebuildable',
    selectable: !blocked && (item.risk === 'low' || isWechatUserData),
  };
}

function normalizeScan(scan: NativeCleanupScan): CleanupScan {
  return { ...scan, items: scan.items.map(normalizeCleanupItem) };
}

function normalizePlan(plan: NativeCleanupPlan): CleanupPlan {
  return {
    ...plan,
    items: plan.items.map(normalizeCleanupItem),
    irreversibleItemIds: [...plan.irreversibleItemIds],
  };
}

export async function scanCleanup(): Promise<CleanupScan> {
  if (!isTauriRuntime()) return previewScanCleanup();
  return normalizeScan(await invoke<NativeCleanupScan>('scan_cleanup_v2'));
}

export async function createCleanupPlan(
  scanId: string,
  itemIds: readonly string[],
): Promise<CleanupPlan> {
  if (!isTauriRuntime()) return previewCreateCleanupPlan(scanId, itemIds);
  const plan = await invoke<NativeCleanupPlan>('create_cleanup_plan', {
    request: { scanId, itemIds: [...itemIds] },
  });
  return normalizePlan(plan);
}

export async function executeCleanupPlan(
  planId: string,
  confirmedIrreversibleItemIds: readonly string[] = [],
  onProgress?: CleanupProgressHandler,
): Promise<ExecuteResult> {
  if (!isTauriRuntime()) {
    return previewExecuteCleanupPlan(planId, confirmedIrreversibleItemIds, onProgress);
  }
  const unlisten = onProgress
    ? await listen<CleanupProgress>('cleanup-progress', (event) => onProgress(event.payload))
    : undefined;
  try {
    return await invoke<ExecuteResult>('execute_cleanup_plan', {
      request: {
        planId,
        confirmed: true,
        confirmedIrreversibleItemIds: [...confirmedIrreversibleItemIds],
      },
    });
  } finally {
    unlisten?.();
  }
}
