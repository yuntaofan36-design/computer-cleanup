import { useCallback, useEffect, useRef, useState } from 'react';
import { quarantineApi } from '../api';
import type {
  QuarantineApi,
  QuarantineExportResult,
  QuarantineListStatus,
  QuarantineRecord,
} from '../types';

export interface UseQuarantineOptions {
  api?: QuarantineApi;
  limit?: number;
  autoLoad?: boolean;
}

export interface UseQuarantineResult {
  records: QuarantineRecord[];
  corruptRecords: number;
  status: QuarantineListStatus;
  listError: string;
  exportError: string;
  busyRecordId: string | null;
  lastExport: QuarantineExportResult | null;
  refresh: () => Promise<void>;
  exportCopy: (recordId: string) => Promise<QuarantineExportResult>;
  clearExportFeedback: () => void;
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error && error.message ? error.message : fallback;
}

export function useQuarantine({
  api = quarantineApi,
  limit = 100,
  autoLoad = true,
}: UseQuarantineOptions = {}): UseQuarantineResult {
  const [records, setRecords] = useState<QuarantineRecord[]>([]);
  const [corruptRecords, setCorruptRecords] = useState(0);
  const [status, setStatus] = useState<QuarantineListStatus>('idle');
  const [listError, setListError] = useState('');
  const [exportError, setExportError] = useState('');
  const [busyRecordId, setBusyRecordId] = useState<string | null>(null);
  const [lastExport, setLastExport] = useState<QuarantineExportResult | null>(null);
  const mountedRef = useRef(true);
  const loadedRef = useRef(false);
  const loadSequenceRef = useRef(0);
  const exportInFlightRef = useRef<string | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const refresh = useCallback(async (): Promise<void> => {
    const sequence = loadSequenceRef.current + 1;
    loadSequenceRef.current = sequence;
    setListError('');
    setStatus(loadedRef.current ? 'refreshing' : 'loading');
    try {
      const response = await api.list(limit);
      if (!mountedRef.current || sequence !== loadSequenceRef.current) return;
      setRecords(response.records);
      setCorruptRecords(Math.max(0, response.corruptRecords));
      loadedRef.current = true;
      setStatus('ready');
    } catch (error) {
      if (!mountedRef.current || sequence !== loadSequenceRef.current) return;
      setListError(errorMessage(error, '无法读取隔离库存'));
      setStatus('error');
    }
  }, [api, limit]);

  useEffect(() => {
    if (autoLoad) void refresh();
  }, [autoLoad, refresh]);

  const exportCopy = useCallback(async (recordId: string): Promise<QuarantineExportResult> => {
    if (exportInFlightRef.current) throw new Error('已有隔离副本正在导出');
    exportInFlightRef.current = recordId;
    setBusyRecordId(recordId);
    setExportError('');
    setLastExport(null);
    try {
      const result = await api.exportCopy(recordId);
      if (mountedRef.current) setLastExport(result);
      await refresh();
      return result;
    } catch (error) {
      if (mountedRef.current) {
        setExportError(errorMessage(error, '隔离副本导出失败'));
      }
      throw error;
    } finally {
      exportInFlightRef.current = null;
      if (mountedRef.current) setBusyRecordId(null);
    }
  }, [api, refresh]);

  const clearExportFeedback = useCallback((): void => {
    setExportError('');
    setLastExport(null);
  }, []);

  return {
    records,
    corruptRecords,
    status,
    listError,
    exportError,
    busyRecordId,
    lastExport,
    refresh,
    exportCopy,
    clearExportFeedback,
  };
}

