// @vitest-environment jsdom

import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { OperationRecord } from '../../../../types';
import type {
  QuarantineApi,
  QuarantineExportResult,
  QuarantineRecord,
} from '../../types';
import { QuarantinePage } from './QuarantinePage';

afterEach(cleanup);

const committedRecord: QuarantineRecord = {
  recordId: 'record-1',
  fileName: 'cache.tmp',
  ruleId: 'temp',
  planId: 'plan-1',
  createdAtMs: 1_753_430_400_000,
  sizeBytes: 2048,
  state: 'committed',
  exportable: true,
  sourceRetained: false,
};

const exportResult: QuarantineExportResult = {
  operationId: 'operation-1',
  recordId: committedRecord.recordId,
  exportedDirectory: 'C:\\Qingpan\\Exports\\operation-1',
  exportedFileName: committedRecord.fileName,
  bytes: committedRecord.sizeBytes,
  quarantineSourceRetained: true,
  auditPersisted: true,
};

const historicalCleanup: OperationRecord = {
  id: 'history-1',
  kind: 'cleanup',
  title: '历史隔离清理',
  createdAt: '2026/7/25 10:00:00',
  reclaimedBytes: 0,
  stagedBytes: 8192,
  status: 'success',
  detail: '旧审计记录',
};

function createApi(
  records: QuarantineRecord[],
  exportCopy: QuarantineApi['exportCopy'] = async () => exportResult,
): QuarantineApi {
  return {
    list: vi.fn(async () => ({ records, corruptRecords: 0 })),
    exportCopy: vi.fn(exportCopy),
  };
}

describe('QuarantinePage inventory semantics', () => {
  it('keeps a load failure distinct from an empty inventory and supports retry', async () => {
    const list = vi.fn()
      .mockRejectedValueOnce(new Error('库存日志不可读'))
      .mockResolvedValueOnce({ records: [], corruptRecords: 0 });
    const api: QuarantineApi = {
      list,
      exportCopy: vi.fn(async () => exportResult),
    };
    render(<QuarantinePage api={api} />);

    expect(await screen.findByText('库存状态未知')).toBeTruthy();
    expect(screen.queryByText('当前没有隔离对象')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: '重试' }));

    expect(await screen.findByText('当前没有隔离对象')).toBeTruthy();
    expect(list).toHaveBeenCalledTimes(2);
  });

  it('does not infer exportability from historical staged bytes', async () => {
    const api = createApi([]);
    render(<QuarantinePage api={api} auditRecords={[historicalCleanup]} />);

    expect(await screen.findByText('当前没有隔离对象')).toBeTruthy();
    expect(screen.getByText('当时记录隔离量')).toBeTruthy();
    expect(screen.queryByText('导出隔离副本')).toBeNull();
    expect(api.exportCopy).not.toHaveBeenCalled();
  });

  it('renders source-retained, recovery-required and damaged states without false claims', async () => {
    const api = createApi([
      {
        ...committedRecord,
        recordId: 'source-retained',
        state: 'sourceRetained',
        sourceRetained: true,
      },
      {
        ...committedRecord,
        recordId: 'recovery-required',
        fileName: 'uncertain.tmp',
        state: 'recoveryRequired',
        exportable: true,
      },
      {
        ...committedRecord,
        recordId: 'damaged',
        fileName: 'damaged.tmp',
        state: 'damaged',
        exportable: false,
      },
    ]);
    render(<QuarantinePage api={api} />);

    expect(await screen.findByText('两份内容均保留；未完成隔离。')).toBeTruthy();
    expect(screen.getByText('普通导出不可用；仅可由专用救援/取证流程处理。')).toBeTruthy();
    expect(screen.getByText('隔离对象未通过完整性检查，普通导出不可用。')).toBeTruthy();
    expect(screen.getByText('不可导出')).toBeTruthy();
    expect(screen.getAllByText('需专用救援')).toHaveLength(2);
    expect(screen.queryByRole('button', {
      name: '导出 uncertain.tmp 的隔离副本',
    })).toBeNull();
    expect(screen.getAllByRole('button', { name: /导出 .* 的隔离副本/ })).toHaveLength(1);
  });
});

describe('QuarantinePage export flow', () => {
  it('waits for the backend terminal result before showing success', async () => {
    let resolveExport: (result: QuarantineExportResult) => void = () => undefined;
    const pendingExport = new Promise<QuarantineExportResult>((resolve) => {
      resolveExport = resolve;
    });
    const api = createApi([committedRecord], () => pendingExport);
    render(<QuarantinePage api={api} />);

    fireEvent.click(await screen.findByRole('button', {
      name: '导出 cache.tmp 的隔离副本',
    }));
    const confirm = screen.getByRole('button', { name: '导出隔离副本' });
    fireEvent.click(confirm);

    expect(screen.queryByText('隔离副本已导出')).toBeNull();
    expect(screen.getByRole('button', { name: '正在导出…' })).toBeTruthy();

    await act(async () => {
      resolveExport(exportResult);
      await pendingExport;
    });

    expect(await screen.findByText('隔离副本已导出')).toBeTruthy();
    expect(screen.getByText('隔离源副本仍保留，本次导出不会释放隔离占用。')).toBeTruthy();
    expect(api.exportCopy).toHaveBeenCalledWith(committedRecord.recordId);
    await waitFor(() => expect(api.list).toHaveBeenCalledTimes(2));
  });

  it('keeps the dialog open and exposes the backend error when export fails', async () => {
    const api = createApi(
      [committedRecord],
      async () => { throw new Error('隔离对象校验失败'); },
    );
    render(<QuarantinePage api={api} />);

    fireEvent.click(await screen.findByRole('button', {
      name: '导出 cache.tmp 的隔离副本',
    }));
    fireEvent.click(screen.getByRole('button', { name: '导出隔离副本' }));

    expect((await screen.findByRole('alert')).textContent).toContain('隔离对象校验失败');
    expect(screen.getByRole('dialog')).toBeTruthy();
    expect(screen.queryByText('隔离副本已导出')).toBeNull();
    expect(screen.getByRole('button', { name: '导出隔离副本' })).toBeTruthy();
  });
});
