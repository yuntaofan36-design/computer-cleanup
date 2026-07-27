import { invoke } from '@tauri-apps/api/core';
import type {
  QuarantineApi,
  QuarantineExportResult,
  QuarantineListResponse,
} from './types';

const DEFAULT_LIST_LIMIT = 100;
const MAX_LIST_LIMIT = 500;

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function normalizeLimit(limit: number): number {
  const normalized = Math.trunc(limit);
  if (!Number.isFinite(limit) || normalized < 1 || normalized > MAX_LIST_LIMIT) {
    throw new Error(`隔离记录数量必须在 1 到 ${MAX_LIST_LIMIT} 之间`);
  }
  return normalized;
}

export async function listQuarantine(
  limit = DEFAULT_LIST_LIMIT,
): Promise<QuarantineListResponse> {
  const normalizedLimit = normalizeLimit(limit);
  if (!isTauriRuntime()) return { records: [], corruptRecords: 0 };
  return invoke<QuarantineListResponse>('list_quarantine_preview', {
    limit: normalizedLimit,
  });
}

export async function exportQuarantineCopy(
  recordId: string,
): Promise<QuarantineExportResult> {
  const normalizedRecordId = recordId.trim();
  if (!normalizedRecordId) throw new Error('缺少隔离记录标识');
  if (!isTauriRuntime()) throw new Error('浏览器预览不执行隔离副本导出');
  return invoke<QuarantineExportResult>('export_quarantine_copy_preview', {
    recordId: normalizedRecordId,
  });
}

export const quarantineApi: QuarantineApi = {
  list: listQuarantine,
  exportCopy: exportQuarantineCopy,
};

