import { useMemo, useState } from 'react';
import {
  AlertTriangle,
  ArchiveRestore,
  History,
  LoaderCircle,
  RefreshCw,
  ShieldCheck,
} from 'lucide-react';
import { formatBytes } from '../../../../format';
import type { OperationRecord } from '../../../../types';
import { quarantineApi } from '../../api';
import { useQuarantine } from '../../hooks';
import type { QuarantineApi, QuarantineRecord } from '../../types';
import { AuditHistory } from '../AuditHistory';
import { ExportCopyDialog } from '../ExportCopyDialog';
import { QuarantineRecordList } from '../QuarantineRecordList';
import { QuarantineSummary } from '../QuarantineSummary';
import styles from './styles/index.module.css';

export interface QuarantinePageProps {
  auditRecords?: OperationRecord[];
  api?: QuarantineApi;
  listLimit?: number;
}

function exportedPath(directory: string, fileName: string): string {
  const separator = directory.endsWith('\\') || directory.endsWith('/') ? '' : '\\';
  return `${directory}${separator}${fileName}`;
}

function canExportOrdinaryCopy(record: QuarantineRecord): boolean {
  return record.exportable
    && (record.state === 'committed' || record.state === 'sourceRetained');
}

export function QuarantinePage({
  auditRecords = [],
  api = quarantineApi,
  listLimit = 100,
}: QuarantinePageProps): JSX.Element {
  const [exportTarget, setExportTarget] = useState<QuarantineRecord | null>(null);
  const {
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
  } = useQuarantine({ api, limit: listLimit });

  const totalBytes = useMemo(
    () => records.reduce((sum, record) => sum + Math.max(0, record.sizeBytes), 0),
    [records],
  );
  const exportableCount = useMemo(
    () => records.filter(canExportOrdinaryCopy).length,
    [records],
  );
  const reviewCount = useMemo(
    () => records.filter((record) => (
      record.state === 'sourceRetained'
      || record.state === 'recoveryRequired'
      || record.state === 'damaged'
      || record.sourceRetained
    )).length + corruptRecords,
    [corruptRecords, records],
  );
  const loading = status === 'idle' || status === 'loading';
  const refreshing = status === 'refreshing';

  function openExport(record: QuarantineRecord): void {
    if (!canExportOrdinaryCopy(record)) return;
    clearExportFeedback();
    setExportTarget(record);
  }

  function closeExport(): void {
    if (busyRecordId) return;
    setExportTarget(null);
    clearExportFeedback();
  }

  async function confirmExport(): Promise<void> {
    if (!exportTarget || busyRecordId) return;
    try {
      await exportCopy(exportTarget.recordId);
      setExportTarget(null);
    } catch {
      // The dialog remains open and renders the backend error from the hook.
    }
  }

  return (
    <section className={styles.page}>
      <header className={styles.pageHead}>
        <div>
          <h1>隔离与记录</h1>
          <p>当前库存来自本机隔离仓库；操作历史不参与可导出性判断。</p>
        </div>
        <button
          type='button'
          className='button secondary'
          onClick={() => void refresh()}
          disabled={loading || refreshing || busyRecordId !== null}
        >
          <RefreshCw className={refreshing ? styles.spin : undefined} />
          {refreshing ? '正在刷新…' : '刷新库存'}
        </button>
      </header>

      <QuarantineSummary
        totalBytes={totalBytes}
        exportableCount={exportableCount}
        reviewCount={reviewCount}
      />

      <div className={styles.scopeNotice}>
        <ShieldCheck aria-hidden='true' />
        <span><strong>当前仅对 temp 规则的低风险临时文件实验性启用隔离。</strong>高风险用户数据仍由规则引擎阻断，这里不代表完整恢复协议已经开放。</span>
      </div>

      {lastExport && (
        <div
          className={lastExport.quarantineSourceRetained && lastExport.auditPersisted
            ? styles.successNotice
            : styles.warningNotice}
          role='status'
        >
          {lastExport.quarantineSourceRetained && lastExport.auditPersisted
            ? <ArchiveRestore aria-hidden='true' />
            : <AlertTriangle aria-hidden='true' />}
          <span>
            <strong>隔离副本已导出</strong>
            <span>{exportedPath(lastExport.exportedDirectory, lastExport.exportedFileName)} · {formatBytes(lastExport.bytes)}</span>
            {lastExport.quarantineSourceRetained
              ? <small>隔离源副本仍保留，本次导出不会释放隔离占用。</small>
              : <small>后端未确认隔离源保留，请停止后续操作并人工核对库存。</small>}
            {!lastExport.auditPersisted && <small>导出已完成，但本地审计记录未成功落盘。</small>}
          </span>
        </div>
      )}

      {corruptRecords > 0 && (
        <div className={styles.warningNotice} role='status'>
          <AlertTriangle aria-hidden='true' />
          <span><strong>已跳过 {corruptRecords} 条损坏的隔离索引记录</strong><small>它们不计入当前占用或可导出数量，需要人工检查隔离仓库。</small></span>
        </div>
      )}

      <section className={styles.inventory} aria-labelledby='quarantine-inventory-title'>
        <header>
          <div>
            <h2 id='quarantine-inventory-title'>当前隔离库存</h2>
            <p>导出只复制到新目录，不覆盖原位置，也不会删除隔离源副本。</p>
          </div>
          <span>{records.length} 条记录</span>
        </header>

        {listError && (
          <div className={styles.inlineError} role='alert'>
            <AlertTriangle />
            <span><strong>无法读取隔离库存</strong><small>{listError}</small></span>
            <button type='button' className='button secondary small' onClick={() => void refresh()}>重试</button>
          </div>
        )}
        {loading ? (
          <div className={styles.loading}><LoaderCircle /><span>正在读取本机隔离库存…</span></div>
        ) : status === 'error' && records.length === 0 ? (
          <div className={styles.empty}>
            <AlertTriangle aria-hidden='true' />
            <h3>库存状态未知</h3>
            <p>读取失败不代表隔离仓库为空。请处理上方错误后重试。</p>
          </div>
        ) : records.length === 0 ? (
          <div className={styles.empty}>
            <History aria-hidden='true' />
            <h3>当前没有隔离对象</h3>
            <p>完成启用了隔离模式的真实清理后，库存记录才会出现在这里。浏览器预览不会生成演示记录。</p>
          </div>
        ) : (
          <QuarantineRecordList
            records={records}
            busyRecordId={busyRecordId}
            onExport={openExport}
          />
        )}
      </section>

      <AuditHistory records={auditRecords} />

      {exportTarget && (
        <ExportCopyDialog
          record={exportTarget}
          busy={busyRecordId === exportTarget.recordId}
          error={exportError}
          onCancel={closeExport}
          onConfirm={() => void confirmExport()}
        />
      )}
    </section>
  );
}
