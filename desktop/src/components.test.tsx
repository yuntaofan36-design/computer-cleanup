// @vitest-environment jsdom

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { CleanupExecutionSummary, Dialog } from './components';
import { formatBytes } from './format';
import type { CleanupItem, CleanupProgress } from './types';

const cleanupItems: CleanupItem[] = [
  {
    id: 'browser-cache',
    scope: 'browser',
    category: '浏览器缓存',
    product: 'Edge',
    name: 'Edge 缓存',
    path: 'C:\\Temp\\Edge',
    description: '可重建缓存',
    reason: '长时间未使用',
    sizeBytes: 4096,
    fileCount: 20,
    risk: 'low',
    confidence: 'high',
    impact: 'rebuild',
    recoverability: 'rebuildable',
    deleteMode: 'permanent',
    selectable: true,
  },
  {
    id: 'app-log',
    scope: 'apps',
    category: '软件缓存',
    product: 'Figma',
    name: 'Figma 诊断日志',
    path: 'C:\\Temp\\Figma',
    description: '诊断日志',
    reason: '旧日志',
    sizeBytes: 2048,
    fileCount: 4,
    risk: 'medium',
    confidence: 'medium',
    impact: 'none',
    recoverability: 'recoverable',
    deleteMode: 'recycle_bin',
    selectable: true,
  },
];

const runningProgress: CleanupProgress = {
  phase: 'running',
  completedItems: 0,
  totalItems: 2,
  completedFiles: 6,
  totalFiles: 24,
  currentItemId: 'browser-cache',
  currentItemName: 'Edge 缓存',
  currentPath: 'C:\\Temp\\Edge\\cache.bin',
  reclaimedBytes: 1024,
  failedFiles: 0,
};

describe('destructive confirmation dialog', () => {
  it('keeps the destructive action disabled until irreversible data is acknowledged', () => {
    const { rerender } = render(
      <Dialog
        title="确认执行"
        confirmLabel="永久删除所选内容"
        confirmDisabled
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      >
        <p>微信用户数据</p>
      </Dialog>,
    );

    const confirm = screen.getByRole('button', { name: '永久删除所选内容' }) as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);

    rerender(
      <Dialog
        title="确认执行"
        confirmLabel="永久删除所选内容"
        confirmDisabled={false}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      >
        <p>微信用户数据</p>
      </Dialog>,
    );
    expect(confirm.disabled).toBe(false);
  });
});

describe('cleanup execution summary', () => {
  it('shows animated file-level progress and the current cleanup target', () => {
    const { container } = render(
      <CleanupExecutionSummary
        items={cleanupItems}
        progress={runningProgress}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByText('25%')).toBeTruthy();
    expect(screen.getByText('C:\\Temp\\Edge\\cache.bin')).toBeTruthy();
    expect(container.querySelector('.execution-item.running')?.textContent).toContain('清理中');
    expect(container.querySelector('.execution-item.pending')?.textContent).toContain('等待中');
    expect(container.querySelector('.execution-metrics')?.textContent).toContain('隔离占用待结果');
  });

  it('summarizes completed rules and marks retained files as partial', () => {
    const { container } = render(
      <CleanupExecutionSummary
        items={cleanupItems}
        progress={{
          ...runningProgress,
          phase: 'complete',
          completedItems: 2,
          completedFiles: 24,
          reclaimedBytes: 4096,
          failedFiles: 1,
        }}
        result={{
          reclaimedBytes: 4096,
          stagedBytes: 2048,
          succeeded: 1,
          failed: [{ id: 'app-log', error: '文件正在使用' }],
        }}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByText('100%')).toBeTruthy();
    expect(container.querySelector('.execution-item.done')?.textContent).toContain('已完成');
    expect(container.querySelector('.execution-item.partial')?.textContent).toContain('部分保留');
    expect(container.querySelector('.execution-metrics')?.textContent).toContain(`实际释放${formatBytes(4096)}`);
    expect(container.querySelector('.execution-metrics')?.textContent).toContain(`隔离占用${formatBytes(2048)}`);
    expect(screen.getByText('发生变化或被占用的文件已安全保留')).toBeTruthy();
  });
});
