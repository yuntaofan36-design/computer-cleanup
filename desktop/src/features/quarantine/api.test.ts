// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { exportQuarantineCopy, listQuarantine } from './api';

function useNativeRuntime(): void {
  Object.defineProperty(window, '__TAURI_INTERNALS__', {
    configurable: true,
    value: {},
  });
}

function usePreviewRuntime(): void {
  Reflect.deleteProperty(window, '__TAURI_INTERNALS__');
}

describe('quarantine native API', () => {
  beforeEach(() => {
    useNativeRuntime();
    invokeMock.mockReset();
  });

  it('loads the real quarantine inventory with the fixed command payload', async () => {
    invokeMock.mockResolvedValue({ records: [], corruptRecords: 2 });

    await expect(listQuarantine(80)).resolves.toEqual({ records: [], corruptRecords: 2 });

    expect(invokeMock).toHaveBeenCalledWith('list_quarantine_preview', { limit: 80 });
  });

  it('exports one record through the backend and returns only its terminal result', async () => {
    const result = {
      operationId: 'operation-1',
      recordId: 'record-1',
      exportedDirectory: 'C:\\Qingpan\\Exports\\operation-1',
      exportedFileName: 'cache.tmp',
      bytes: 2048,
      quarantineSourceRetained: true,
      auditPersisted: true,
    };
    invokeMock.mockResolvedValue(result);

    await expect(exportQuarantineCopy(' record-1 ')).resolves.toEqual(result);

    expect(invokeMock).toHaveBeenCalledWith('export_quarantine_copy_preview', {
      recordId: 'record-1',
    });
  });

  it('rejects invalid limits before invoking native code', async () => {
    await expect(listQuarantine(0)).rejects.toThrow('1 到 500');
    await expect(listQuarantine(501)).rejects.toThrow('1 到 500');
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe('quarantine browser preview', () => {
  beforeEach(() => {
    usePreviewRuntime();
    invokeMock.mockReset();
  });

  it('shows an empty inventory and refuses to simulate a successful export', async () => {
    await expect(listQuarantine()).resolves.toEqual({ records: [], corruptRecords: 0 });
    await expect(exportQuarantineCopy('record-1')).rejects.toThrow('浏览器预览不执行');
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
