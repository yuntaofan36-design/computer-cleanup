import { AlertTriangle, ArchiveRestore, ShieldCheck, X } from 'lucide-react';
import { formatBytes } from '../../../../format';
import type { QuarantineRecord } from '../../types';
import styles from './styles/index.module.css';

export interface ExportCopyDialogProps {
  record: QuarantineRecord;
  busy: boolean;
  error: string;
  onCancel: () => void;
  onConfirm: () => void;
}

export function ExportCopyDialog({
  record,
  busy,
  error,
  onCancel,
  onConfirm,
}: ExportCopyDialogProps): JSX.Element {
  const sourceRetained = record.sourceRetained || record.state === 'sourceRetained';
  const recoveryRequired = record.state === 'recoveryRequired';

  return (
    <div
      className={styles.overlay}
      role='presentation'
      onMouseDown={() => {
        if (!busy) onCancel();
      }}
    >
      <div
        className={styles.dialog}
        role='dialog'
        aria-modal='true'
        aria-labelledby='export-copy-title'
        aria-busy={busy}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button
          type='button'
          className={`icon-button ${styles.close}`}
          onClick={onCancel}
          aria-label='关闭'
          disabled={busy}
        >
          <X />
        </button>
        <span className={styles.icon} aria-hidden='true'><ArchiveRestore /></span>
        <h2 id='export-copy-title'>导出“{record.fileName || '未命名隔离对象'}”的隔离副本？</h2>
        <p className={styles.description}>
          后端会把隔离内容复制到独立导出目录，不覆盖原路径。导出完成后，隔离源副本仍保留，本次操作不会释放隔离占用。
        </p>
        <dl className={styles.facts}>
          <div><dt>隔离对象</dt><dd>{record.fileName || '未命名隔离对象'}</dd></div>
          <div><dt>副本大小</dt><dd>{formatBytes(Math.max(0, record.sizeBytes))}</dd></div>
          <div><dt>来源规则</dt><dd>{record.ruleId}</dd></div>
        </dl>

        {sourceRetained && (
          <div className={styles.warning}>
            <AlertTriangle aria-hidden='true' />
            <span><strong>两份内容均保留；未完成隔离。</strong>导出后仍需人工核对原位置与隔离库存。</span>
          </div>
        )}
        {recoveryRequired && (
          <div className={styles.warning}>
            <AlertTriangle aria-hidden='true' />
            <span><strong>源状态需人工核对。</strong>这里只导出已验证的隔离对象，不推断原位置当前状态。</span>
          </div>
        )}
        {!sourceRetained && !recoveryRequired && (
          <div className={styles.proof}>
            <ShieldCheck aria-hidden='true' />
            <span>导出是复制操作；不会将内容直接写回原路径。</span>
          </div>
        )}
        {error && <div className={styles.error} role='alert'><AlertTriangle />{error}</div>}

        <div className={styles.actions}>
          <button type='button' className='button secondary' onClick={onCancel} disabled={busy}>
            取消
          </button>
          <button type='button' className='button primary' onClick={onConfirm} disabled={busy}>
            <ArchiveRestore />
            {busy ? '正在导出…' : '导出隔离副本'}
          </button>
        </div>
      </div>
    </div>
  );
}

