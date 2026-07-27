import { CheckCircle2, CircleAlert, Clock3, History } from 'lucide-react';
import { formatBytes } from '../../../../format';
import type { OperationRecord } from '../../../../types';
import styles from './styles/index.module.css';

export interface AuditHistoryProps {
  records: OperationRecord[];
}

const kindLabels: Record<OperationRecord['kind'], string> = {
  cleanup: '清理',
  restore: '导出',
  uninstall: '卸载',
};

const statusLabels: Record<OperationRecord['status'], string> = {
  success: '已完成',
  partial: '部分完成',
  failed: '失败',
};

function StatusIcon({ status }: { status: OperationRecord['status'] }): JSX.Element {
  if (status === 'success') return <CheckCircle2 aria-hidden='true' />;
  if (status === 'failed') return <CircleAlert aria-hidden='true' />;
  return <Clock3 aria-hidden='true' />;
}

export function AuditHistory({ records }: AuditHistoryProps): JSX.Element {
  return (
    <section className={styles.panel} aria-labelledby='audit-history-title'>
      <header>
        <div>
          <h2 id='audit-history-title'>本地操作历史</h2>
          <p>历史记录用于审计，不代表当前隔离对象仍存在或可导出。</p>
        </div>
      </header>
      {records.length === 0 ? (
        <div className={styles.empty}>
          <History aria-hidden='true' />
          <span><strong>暂无操作历史</strong><small>完成真实清理、导出或卸载后会显示在这里。</small></span>
        </div>
      ) : (
        <div className={styles.list}>
          {records.map((record) => (
            <article className={styles.row} key={record.id}>
              <span className={`${styles.statusIcon} ${styles[record.status]}`}>
                <StatusIcon status={record.status} />
              </span>
              <div className={styles.description}>
                <strong>{record.title}</strong>
                <small>{record.createdAt} · {record.detail}</small>
                <span>
                  <b>{kindLabels[record.kind]}</b>
                  <b>{statusLabels[record.status]}</b>
                </span>
              </div>
              <dl>
                <div><dt>实际释放</dt><dd>{formatBytes(record.reclaimedBytes)}</dd></div>
                <div><dt>当时记录隔离量</dt><dd>{formatBytes(record.stagedBytes)}</dd></div>
              </dl>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

