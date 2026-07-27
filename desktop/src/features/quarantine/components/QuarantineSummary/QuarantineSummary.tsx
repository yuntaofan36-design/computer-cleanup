import { ArchiveRestore, FileCheck2, HardDrive } from 'lucide-react';
import { formatBytes } from '../../../../format';
import styles from './styles/index.module.css';

export interface QuarantineSummaryProps {
  totalBytes: number;
  exportableCount: number;
  reviewCount: number;
}

export function QuarantineSummary({
  totalBytes,
  exportableCount,
  reviewCount,
}: QuarantineSummaryProps): JSX.Element {
  return (
    <div className={styles.summary} aria-label='当前隔离库存概览'>
      <article>
        <span className={styles.primaryIcon}><HardDrive /></span>
        <div><small>当前隔离占用</small><strong>{formatBytes(totalBytes)}</strong><span>只统计当前可读取的库存记录</span></div>
      </article>
      <article>
        <span className={styles.safeIcon}><ArchiveRestore /></span>
        <div><small>可导出副本</small><strong>{exportableCount} 条</strong><span>由后端逐条给出导出能力</span></div>
      </article>
      <article>
        <span className={reviewCount ? styles.warningIcon : styles.primaryIcon}><FileCheck2 /></span>
        <div><small>需人工核对</small><strong>{reviewCount} 条</strong><span>异常记录不会被包装成恢复成功</span></div>
      </article>
    </div>
  );
}

