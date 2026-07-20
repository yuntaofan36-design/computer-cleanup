import { useMemo, useState } from 'react';
import {
  ArchiveRestore,
  CheckCircle2,
  CircleAlert,
  Clock3,
  HardDrive,
  History,
  RotateCcw,
  ShieldCheck,
  X,
} from 'lucide-react';
import { formatBytes } from '../format';
import type { OperationRecord } from '../types';

export interface RecoveryCenterProps {
  records: OperationRecord[];
  onRestore: (recordId: string) => void;
  busyRecordId?: string | null;
}

const kindLabels: Record<OperationRecord['kind'], string> = {
  cleanup: '清理',
  restore: '恢复',
  uninstall: '卸载',
};

const statusLabels: Record<OperationRecord['status'], string> = {
  success: '已完成',
  partial: '部分完成',
  failed: '失败',
};

interface RestoreDialogProps {
  record: OperationRecord;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

function RestoreDialog({ record, busy, onCancel, onConfirm }: RestoreDialogProps): JSX.Element {
  return (
    <div className="overlay" role="presentation" onMouseDown={onCancel}>
      <div
        className="dialog confirmation-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="restore-dialog-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button
          type="button"
          className="icon-button dialog-close"
          onClick={onCancel}
          aria-label="关闭"
          disabled={busy}
        >
          <X size={18} />
        </button>
        <span className="dialog-icon safe" aria-hidden="true"><ArchiveRestore /></span>
        <h2 id="restore-dialog-title">恢复“{record.title}”中的隔离内容？</h2>
        <p>
          为防止覆盖用户后来创建或修改的文件，恢复内容会写入清盘新建的恢复目录，绝不会直接覆盖原路径；同名文件会使用唯一名称。
        </p>
        <div className="restore-fact">
          <span>预计从隔离区恢复</span>
          <strong>{formatBytes(record.stagedBytes)}</strong>
        </div>
        <div className="dialog-actions">
          <button type="button" className="button secondary" onClick={onCancel} disabled={busy}>
            取消
          </button>
          <button type="button" className="button primary" onClick={onConfirm} disabled={busy}>
            <ArchiveRestore size={17} />
            {busy ? '正在提交…' : '恢复到新目录'}
          </button>
        </div>
      </div>
    </div>
  );
}

function StatusIcon({ status }: { status: OperationRecord['status'] }): JSX.Element {
  if (status === 'success') return <CheckCircle2 aria-hidden="true" />;
  if (status === 'failed') return <CircleAlert aria-hidden="true" />;
  return <Clock3 aria-hidden="true" />;
}

export default function RecoveryCenter({
  records,
  onRestore,
  busyRecordId = null,
}: RecoveryCenterProps): JSX.Element {
  const [restoreTarget, setRestoreTarget] = useState<OperationRecord | null>(null);
  const [submittedRestoreId, setSubmittedRestoreId] = useState<string | null>(null);

  const reclaimedBytes = useMemo(
    () => records.reduce((total, record) => total + Math.max(0, record.reclaimedBytes), 0),
    [records],
  );
  const stagedBytes = useMemo(
    () => records.reduce((total, record) => total + Math.max(0, record.stagedBytes), 0),
    [records],
  );
  const restorableCount = useMemo(
    () => records.filter((record) => record.kind === 'cleanup' && record.status !== 'failed' && record.stagedBytes > 0).length,
    [records],
  );

  const confirmRestore = (): void => {
    if (!restoreTarget) return;
    onRestore(restoreTarget.id);
    setSubmittedRestoreId(restoreTarget.id);
    setRestoreTarget(null);
  };

  return (
    <section className="page-section recovery-page">
      <header className="page-head">
        <div className="page-title-block">
          <h1>恢复中心</h1>
          <p>核对清理、隔离、恢复与卸载记录；隔离数据仍然占用磁盘空间。</p>
        </div>
      </header>

      <div className="summary-grid recovery-summary" aria-label="清理空间核算">
        <article className="summary-card safe">
          <span className="summary-icon" aria-hidden="true"><HardDrive /></span>
          <div>
            <small>实际释放</small>
            <strong>{formatBytes(reclaimedBytes)}</strong>
            <span>已经归还给文件系统的可用空间</span>
          </div>
        </article>
        <article className="summary-card warning">
          <span className="summary-icon" aria-hidden="true"><History /></span>
          <div>
            <small>隔离区占用</small>
            <strong>{formatBytes(stagedBytes)}</strong>
            <span>仍在本机保留，不计入实际释放</span>
          </div>
        </article>
        <article className="summary-card">
          <span className="summary-icon" aria-hidden="true"><ArchiveRestore /></span>
          <div>
            <small>可恢复记录</small>
            <strong>{restorableCount} 条</strong>
            <span>恢复时统一写入新的安全目录</span>
          </div>
        </article>
      </div>

      <div className="notice recovery-policy">
        <ShieldCheck aria-hidden="true" />
        <span><strong>恢复不会覆盖原路径。</strong> 文件将写入独立恢复目录；核对无误后，再由你决定是否移动回原位置。</span>
      </div>

      {submittedRestoreId && (
        <div className="notice success-bg" role="status">
          <ArchiveRestore aria-hidden="true" />
          恢复请求已提交。完成后请在新建的恢复目录中核对文件，原位置不会被覆盖。
        </div>
      )}

      <div className="content-panel recovery-panel">
        <div className="panel-head">
          <div>
            <h2>操作记录</h2>
            <p>每次影响磁盘内容的操作都保留本地审计记录。</p>
          </div>
        </div>

        {records.length === 0 ? (
          <div className="empty-state">
            <History aria-hidden="true" />
            <h3>暂无操作记录</h3>
            <p>完成清理、恢复或卸载操作后，记录会显示在这里。</p>
          </div>
        ) : (
          <div className="list operation-list">
            {records.map((record) => {
              const canRestore = record.kind === 'cleanup' && record.status !== 'failed' && record.stagedBytes > 0;
              const isBusy = busyRecordId === record.id;
              return (
                <article className="list-row operation-row" key={record.id}>
                  <span className={`row-icon status-${record.status}`}>
                    <StatusIcon status={record.status} />
                  </span>
                  <div className="grow operation-description">
                    <strong>
                      {record.title}
                      <span className="badge neutral">{kindLabels[record.kind]}</span>
                      <span className={`badge status-${record.status}`}>{statusLabels[record.status]}</span>
                    </strong>
                    <small>{record.createdAt} · {record.detail}</small>
                  </div>
                  <dl className="operation-space">
                    <div>
                      <dt>实际释放</dt>
                      <dd>{formatBytes(record.reclaimedBytes)}</dd>
                    </div>
                    <div>
                      <dt>隔离占用</dt>
                      <dd>{formatBytes(record.stagedBytes)}</dd>
                    </div>
                  </dl>
                  <div className="row-actions">
                    {canRestore ? (
                      <button
                        type="button"
                        className="button secondary small"
                        onClick={() => setRestoreTarget(record)}
                        disabled={isBusy}
                      >
                        <RotateCcw size={15} />
                        {isBusy ? '恢复中…' : '恢复到新目录'}
                      </button>
                    ) : (
                      <span className="muted-action">不可恢复</span>
                    )}
                  </div>
                </article>
              );
            })}
          </div>
        )}
      </div>

      {restoreTarget && (
        <RestoreDialog
          record={restoreTarget}
          busy={busyRecordId === restoreTarget.id}
          onCancel={() => setRestoreTarget(null)}
          onConfirm={confirmRestore}
        />
      )}
    </section>
  );
}
