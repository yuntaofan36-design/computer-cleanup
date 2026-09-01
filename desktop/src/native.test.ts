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

import { startups as previewStartups } from './mockData';
import {
  deleteLargeFiles,
  inferCleanupScope,
  loadAppIcon,
  loadApps,
  loadPartitionDisks,
  loadStartupEntries,
  loadStartupIcon,
  setStartupEntryEnabled,
} from './native';

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

  // Developer caches previously fell through to the generic system bucket, so npm
  // and Maven entries were presented to the user as Windows system items.
  it('classifies developer tool caches as devtools rather than system', () => {
    expect(inferCleanupScope('开发者缓存', 'npm · 包内容缓存')).toBe('devtools');
    expect(inferCleanupScope('开发者缓存', 'Maven · 本地仓库')).toBe('devtools');
    expect(inferCleanupScope('开发者缓存', 'Cargo · crate 解压源码')).toBe('devtools');
    expect(inferCleanupScope('开发者缓存', 'pip · 下载缓存')).toBe('devtools');
    expect(inferCleanupScope('开发者缓存', 'Gradle · 构建缓存')).toBe('devtools');
  });

  it('separates QQ caches from WeChat and from generic system junk', () => {
    expect(inferCleanupScope('QQ 缓存', 'QQ · 网络缓存')).toBe('social');
    expect(inferCleanupScope('QQ 缓存', 'QQ · 崩溃报告')).toBe('social');
    // WeChat must still win when both could match, preserving existing behaviour.
    expect(inferCleanupScope('微信运行缓存', '微信 · 网络缓存')).toBe('wechat');
  });

  it('keeps Windows diagnostic locations in the system scope', () => {
    expect(inferCleanupScope('系统垃圾', 'Windows · 应用崩溃转储')).toBe('system');
    expect(inferCleanupScope('系统垃圾', 'Windows · D3D 着色器缓存')).toBe('system');
    expect(inferCleanupScope('系统缓存', '临时文件')).toBe('system');
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

describe('startup entry wrappers', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('uses typed Tauri commands and confirms startup state changes', async () => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {},
    });
    const entries = [{
      id: 'hkcu:OneDrive',
      name: 'OneDrive',
      publisher: '',
      command: 'OneDrive.exe /background',
      enabled: true,
      impact: '未知' as const,
      scope: '当前用户',
    }];
    invokeMock.mockResolvedValueOnce(entries).mockResolvedValueOnce(undefined);

    await expect(loadStartupEntries()).resolves.toEqual(entries);
    await expect(setStartupEntryEnabled('hkcu:OneDrive', false)).resolves.toBeUndefined();

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'list_startup_entries');
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'set_startup_enabled', {
      id: 'hkcu:OneDrive',
      enabled: false,
      confirmed: true,
    });
  });

  it('loads a startup icon by opaque entry id', async () => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {},
    });
    invokeMock.mockResolvedValueOnce(iconDataUrl);

    await expect(loadStartupIcon('hkcu:OneDrive')).resolves.toBe(iconDataUrl);

    expect(invokeMock).toHaveBeenCalledWith('get_startup_icon', { id: 'hkcu:OneDrive' });
  });

  it('keeps preview changes in an isolated in-memory copy', async () => {
    Reflect.deleteProperty(window, '__TAURI_INTERNALS__');
    const fixture = previewStartups[0];
    const originalEnabled = fixture.enabled;

    try {
      await setStartupEntryEnabled(fixture.id, !originalEnabled);
      const firstRead = await loadStartupEntries();
      firstRead[0].enabled = originalEnabled;
      const secondRead = await loadStartupEntries();

      expect(secondRead[0].enabled).toBe(!originalEnabled);
      expect(previewStartups[0].enabled).toBe(originalEnabled);
      expect(invokeMock).not.toHaveBeenCalled();
    } finally {
      await setStartupEntryEnabled(fixture.id, originalEnabled);
    }
  });
});
