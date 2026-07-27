// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock, listenMock, unlistenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  unlistenMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

import { deleteLargeFiles, inferCleanupScope, loadAppIcon, loadApps, loadPartitionDisks } from './native';

const iconDataUrl = 'data:image/png;base64,AA==';

function iconInvocationCount(): number {
  return invokeMock.mock.calls.filter(([command]) => command === 'get_app_icon').length;
}

describe('cleanup scope classification', () => {
  it('classifies all supported WeChat names before generic application caches', () => {
    expect(inferCleanupScope('微信运行缓存', '微信 · 网络缓存')).toBe('wechat');
    expect(inferCleanupScope('应用缓存', 'WeChat · Code Cache')).toBe('wechat');
    expect(inferCleanupScope('应用缓存', 'Weixin · GPUCache')).toBe('wechat');
    expect(inferCleanupScope('应用缓存', 'xwechat · Crashpad')).toBe('wechat');
    expect(inferCleanupScope('应用缓存', 'Figma · Cache')).toBe('apps');
  });

  it('classifies every backend browser cache rule without product-name keywords', () => {
    expect(inferCleanupScope('浏览器缓存', 'Vivaldi · Default · Cache')).toBe('browser');
    expect(inferCleanupScope('浏览器缓存', '360 极速浏览器 · Default · GPUCache')).toBe('browser');
    expect(inferCleanupScope('浏览器缓存', 'Opera GX · 默认配置 · Code Cache')).toBe('browser');
  });
});

describe('native application icon loading', () => {
  beforeEach(async () => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {},
    });
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (command === 'list_apps') return Promise.resolve([]);
      return Promise.resolve(null);
    });
    await loadApps();
    invokeMock.mockClear();
  });

  it('runs no more than four icon requests concurrently', async () => {
    const resolvers: Array<(value: string | null) => void> = [];
    let activeRequests = 0;
    let peakActiveRequests = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command !== 'get_app_icon') return Promise.resolve([]);
      activeRequests += 1;
      peakActiveRequests = Math.max(peakActiveRequests, activeRequests);
      return new Promise<string | null>((resolve) => {
        resolvers.push((value) => {
          activeRequests -= 1;
          resolve(value);
        });
      });
    });

    const requests = Array.from({ length: 6 }, (_, index) => loadAppIcon(`concurrent-${index}`));
    expect(iconInvocationCount()).toBe(4);

    resolvers.splice(0).forEach((resolve) => resolve(iconDataUrl));
    await vi.waitFor(() => expect(iconInvocationCount()).toBe(6));
    resolvers.splice(0).forEach((resolve) => resolve(iconDataUrl));
    await Promise.all(requests);

    expect(peakActiveRequests).toBe(4);
  });

  it('does not cache an empty or rejected icon response', async () => {
    invokeMock
      .mockResolvedValueOnce(null)
      .mockRejectedValueOnce(new Error('temporary icon failure'))
      .mockResolvedValueOnce(iconDataUrl);

    await expect(loadAppIcon('retryable')).resolves.toBeNull();
    await expect(loadAppIcon('retryable')).rejects.toThrow('temporary icon failure');
    await expect(loadAppIcon('retryable')).resolves.toBe(iconDataUrl);
    expect(iconInvocationCount()).toBe(3);
  });

  it('evicts the least recently used icon after 256 successful entries', async () => {
    invokeMock.mockResolvedValue(iconDataUrl);

    await Promise.all(
      Array.from({ length: 257 }, (_, index) => loadAppIcon(`cached-${index}`)),
    );
    expect(iconInvocationCount()).toBe(257);

    await loadAppIcon('cached-0');
    expect(iconInvocationCount()).toBe(258);
  });

  it('invalidates cached icons when the installed application list refreshes', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'list_apps') return Promise.resolve([]);
      return Promise.resolve(iconDataUrl);
    });

    await loadAppIcon('refreshed');
    await loadAppIcon('refreshed');
    expect(iconInvocationCount()).toBe(1);

    await loadApps();
    await loadAppIcon('refreshed');
    expect(iconInvocationCount()).toBe(2);
  });

  it('loads the validated physical disk layout through a typed Tauri command', async () => {
    invokeMock.mockResolvedValueOnce([]);

    await expect(loadPartitionDisks()).resolves.toEqual([]);

    expect(invokeMock).toHaveBeenCalledWith('list_partition_disks');
  });
});

describe('native large-file delete progress', () => {
  beforeEach(() => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {},
    });
    invokeMock.mockReset();
    listenMock.mockReset();
    unlistenMock.mockReset();
  });

  it('forwards large-file delete progress and sends only snapshot ids with permanent confirmation', async () => {
    const progress = {
      phase: 'running' as const,
      completed: 0,
      total: 1,
      currentItemId: 'large-video',
      currentName: 'recording.mp4',
      currentPath: 'D:\\Videos\\recording.mp4',
      deletedBytes: 0,
      failed: 0,
    };
    let emitProgress: ((event: { payload: typeof progress }) => void) | undefined;
    listenMock.mockImplementation((_eventName, handler) => {
      emitProgress = handler;
      return Promise.resolve(unlistenMock);
    });
    invokeMock.mockImplementation(() => {
      emitProgress?.({ payload: progress });
      return Promise.resolve({ deletedBytes: 4096, succeededIds: ['large-video'], failed: [] });
    });
    const onProgress = vi.fn();

    await deleteLargeFiles(['large-video'], onProgress);

    expect(listenMock).toHaveBeenCalledWith('large-file-delete-progress', expect.any(Function));
    expect(invokeMock).toHaveBeenCalledWith('delete_large_files', {
      request: { itemIds: ['large-video'], confirmedPermanent: true },
    });
    expect(onProgress).toHaveBeenCalledWith(progress);
    expect(unlistenMock).toHaveBeenCalledOnce();
  });
});
