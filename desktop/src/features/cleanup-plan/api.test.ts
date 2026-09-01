// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock, listenMock, unlistenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  unlistenMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }));

import {
  createCleanupPlan,
  executeCleanupPlan,
  scanCleanup,
} from './api';
import { resetPreviewCleanupStateForTests } from './previewAdapter';

const nativeItem = {
  id: 'browser-cache',
  category: '浏览器缓存',
  name: 'Vivaldi · Default · Cache',
  path: 'C:\\Users\\Test\\AppData\\Local\\Vivaldi\\Cache',
  description: '可由浏览器重新生成',
  sizeBytes: 4096,
  fileCount: 3,
  risk: 'low' as const,
  deleteMode: 'permanent' as const,
};

function useNativeRuntime(): void {
  Object.defineProperty(window, '__TAURI_INTERNALS__', {
    configurable: true,
    value: {},
  });
}

function usePreviewRuntime(): void {
  Reflect.deleteProperty(window, '__TAURI_INTERNALS__');
}

describe('cleanup plan native API', () => {
  beforeEach(() => {
    useNativeRuntime();
    invokeMock.mockReset();
    listenMock.mockReset();
    unlistenMock.mockReset();
    listenMock.mockResolvedValue(unlistenMock);
  });

  it('uses scan_cleanup_v2 and returns the identified scan snapshot', async () => {
    invokeMock.mockResolvedValue({
      scanId: 'scan-1',
      ruleVersion: 'rules-1',
      expiresAtMs: 1000,
      items: [nativeItem],
    });

    const scan = await scanCleanup('11111111-1111-4111-8111-111111111111');

    expect(invokeMock).toHaveBeenCalledWith('scan_cleanup_v2', {
      request: { taskId: '11111111-1111-4111-8111-111111111111' },
    });
    expect(scan.scanId).toBe('scan-1');
    expect(scan.items[0]).toMatchObject({ id: 'browser-cache', scope: 'browser', selectable: true });
  });

  it('maps quarantine mode to recoverable without making it irreversible', async () => {
    invokeMock.mockResolvedValue({
      scanId: 'scan-quarantine',
      ruleVersion: 'rules-1',
      expiresAtMs: 1000,
      items: [{ ...nativeItem, id: 'temp', deleteMode: 'quarantine' }],
    });

    const scan = await scanCleanup('22222222-2222-4222-8222-222222222222');

    expect(scan.items[0]).toMatchObject({
      id: 'temp',
      deleteMode: 'quarantine',
      recoverability: 'recoverable',
      selectable: true,
    });
    expect(scan.items[0]?.recoverability).not.toBe('irreversible');
  });

  it('creates a plan from an exact scan id and item id list', async () => {
    invokeMock.mockResolvedValue({
      planId: 'plan-1',
      scanId: 'scan-1',
      ruleVersion: 'rules-1',
      createdAtMs: 100,
      expiresAtMs: 1000,
      items: [nativeItem],
      totalItems: 1,
      totalFiles: 3,
      totalBytes: 4096,
      irreversibleItemIds: [],
    });

    await createCleanupPlan('scan-1', ['browser-cache']);

    expect(invokeMock).toHaveBeenCalledWith('create_cleanup_plan', {
      request: { scanId: 'scan-1', itemIds: ['browser-cache'] },
    });
  });

  it('sends only the plan id and exact irreversible confirmations, then unlistens', async () => {
    const progress = {
      phase: 'running' as const,
      completedItems: 0,
      totalItems: 1,
      completedFiles: 1,
      totalFiles: 3,
      currentItemId: 'browser-cache',
      currentItemName: 'Vivaldi · Default · Cache',
      currentPath: nativeItem.path,
      reclaimedBytes: 1024,
      failedFiles: 0,
    };
    let emitProgress: ((event: { payload: typeof progress }) => void) | undefined;
    listenMock.mockImplementation((_eventName, handler) => {
      emitProgress = handler;
      return Promise.resolve(unlistenMock);
    });
    invokeMock.mockImplementation(() => {
      emitProgress?.({ payload: progress });
      return Promise.resolve({ reclaimedBytes: 4096, stagedBytes: 0, succeeded: 1, failed: [] });
    });
    const onProgress = vi.fn();

    await executeCleanupPlan('plan-1', ['wechat-user-data'], onProgress);

    expect(invokeMock).toHaveBeenCalledWith('execute_cleanup_plan', {
      request: {
        planId: 'plan-1',
        confirmed: true,
        confirmedIrreversibleItemIds: ['wechat-user-data'],
      },
    });
    expect(listenMock).toHaveBeenCalledWith('cleanup-progress', expect.any(Function));
    expect(onProgress).toHaveBeenCalledWith(progress);
    expect(unlistenMock).toHaveBeenCalledOnce();
  });

  it('unlistens when execution fails', async () => {
    invokeMock.mockRejectedValue(new Error('execution failed'));

    await expect(executeCleanupPlan('plan-1', [], vi.fn())).rejects.toThrow('execution failed');

    expect(unlistenMock).toHaveBeenCalledOnce();
  });
});

describe('cleanup plan preview adapter', () => {
  beforeEach(() => {
    usePreviewRuntime();
    resetPreviewCleanupStateForTests();
    invokeMock.mockReset();
    listenMock.mockReset();
    unlistenMock.mockReset();
  });

  it('keeps an existing plan valid across a later scan and consumes it once', async () => {
    const firstScan = await scanCleanup('33333333-3333-4333-8333-333333333333');
    const selected = firstScan.items.find((item) => item.id === 'temp');
    expect(selected).toBeDefined();
    const plan = await createCleanupPlan(firstScan.scanId, [selected!.id]);

    const secondScan = await scanCleanup('44444444-4444-4444-8444-444444444444');
    expect(secondScan.scanId).not.toBe(firstScan.scanId);

    const result = await executeCleanupPlan(plan.planId, plan.irreversibleItemIds);
    expect(result).toMatchObject({
      reclaimedBytes: 0,
      stagedBytes: selected!.sizeBytes,
      succeeded: 1,
    });
    await expect(
      executeCleanupPlan(plan.planId, plan.irreversibleItemIds),
    ).rejects.toThrow('已经执行');
  });

  it('does not consume a plan when irreversible confirmations are invalid', async () => {
    const scan = await scanCleanup('55555555-5555-4555-8555-555555555555');
    const selected = scan.items.find((item) => item.selectable);
    expect(selected).toBeDefined();
    const plan = await createCleanupPlan(scan.scanId, [selected!.id]);

    await expect(executeCleanupPlan(plan.planId, ['unexpected-id'])).rejects.toThrow('确认列表无效');
    await expect(
      executeCleanupPlan(plan.planId, plan.irreversibleItemIds),
    ).resolves.toMatchObject({ succeeded: 1 });
  });
});
