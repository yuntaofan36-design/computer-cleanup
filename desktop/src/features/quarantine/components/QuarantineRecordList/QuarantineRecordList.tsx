import {
  AlertTriangle,
  ArchiveRestore,
  CheckCircle2,
  CircleHelp,
  FileArchive,
  LoaderCircle,
} from 'lucide-react';
import { formatBytes } from '../../../../format';
import type { QuarantineRecord, QuarantineRecordState } from '../../types';
import styles from './styles/index.module.css';

export interface QuarantineRecordListProps {
  records: QuarantineRecord[];
  busyRecordId: string | null;
  onExport: (record: QuarantineRecord) => void;
}

interface StatePresentation {
  label: string;
  detail: string;
  tone: 'safe' | 'warning' | 'danger' | 'neutral';
  icon: typeof CheckCircle2;
}

const statePresentation: Record<QuarantineRecordState, StatePresentation> = {
  committed: {
    label: '已隔离',
    detail: '隔离对象已提交，可导出副本。',
    tone: 'safe',
    icon: CheckCircle2,
  },
  sourceRetained: {
    label: '两份均保留',
    detail: '两份内容均保留；未完成隔离。',
    tone: 'warning',
    icon: AlertTriangle,
  },
  damaged: {
    label: '记录异常',
    detail: '隔离对象未通过完整性检查，普通导出不可用。',
    tone: 'danger',
    icon: AlertTriangle,
  },
  recoveryRequired: {
    label: '需专用救援',
    detail: '普通导出不可用；仅可由专用救援/取证流程处理。',
    tone: 'warning',
    icon: CircleHelp,
  },
};

function canExportOrdinaryCopy(record: QuarantineRecord): boolean {
  return record.exportable
    && (record.state === 'committed' || record.state === 'sourceRetained');
}

function formatCreatedAt(timestampMs: number): string {
  if (!Number.isFinite(timestampMs) || timestampMs <= 0) return '时间未知';
  return new Date(timestampMs).toLocaleString('zh-CN', { hour12: false });
}

function presentationFor(record: QuarantineRecord): StatePresentation {
  if (record.sourceRetained && record.state === 'committed') {
    return statePresentation.sourceRetained;
  }
  return statePresentation[record.state];
}

export function QuarantineRecordList({
  records,
  busyRecordId,
  onExport,
}: QuarantineRecordListProps): JSX.Element {
  return (
    <div className={styles.list}>
      {records.map((record) => {
        const presentation = presentationFor(record);
        const StatusIcon = presentation.icon;
        const busy = busyRecordId === record.recordId;
        const canExport = canExportOrdinaryCopy(record);
        return (
          <article className={styles.row} key={record.recordId}>
            <span className={styles.fileIcon} aria-hidden='true'><FileArchive /></span>
            <div className={styles.identity}>
              <strong title={record.fileName}>{record.fileName || '未命名隔离对象'}</strong>
              <small>{formatCreatedAt(record.createdAtMs)} · 规则 {record.ruleId}</small>
              <span className={`${styles.state} ${styles[presentation.tone]}`}>
                <StatusIcon aria-hidden='true' />
                <span><b>{presentation.label}</b>{presentation.detail}</span>
              </span>
            </div>
            <div className={styles.meta}>
              <small>隔离占用</small>
              <strong>{formatBytes(Math.max(0, record.sizeBytes))}</strong>
              <span title={record.planId}>计划 {record.planId}</span>
            </div>
            <div className={styles.action}>
              {canExport ? (
                <button
                  type='button'
                  className='button secondary small'
                  disabled={busyRecordId !== null}
                  onClick={() => onExport(record)}
                  aria-label={`导出 ${record.fileName || '未命名隔离对象'} 的隔离副本`}
                >
                  {busy ? <LoaderCircle className={styles.spin} /> : <ArchiveRestore />}
                  {busy ? '正在导出…' : '导出隔离副本'}
                </button>
              ) : (
                <span className={styles.unavailable}>
                  {record.state === 'recoveryRequired' ? '需专用救援' : '不可导出'}
                </span>
              )}
            </div>
          </article>
        );
      })}
    </div>
  );
}
