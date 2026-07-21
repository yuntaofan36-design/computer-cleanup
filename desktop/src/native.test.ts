// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { executeCleanup, inferCleanupScope, loadAppIcon, loadApps, loadPartitionDisks, scanCleanup } from './native';

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

  it('maps high-risk WeChat data to an explicitly selectable irreversible item', async () => {
    invokeMock.mockResolvedValueOnce([{
      id: 'wechat-user-test-chat-records',
      category: '微信聊天记录',
      name: '微信 · account-test · 聊天记录',
      path: 'C:\\Users\\Test\\Documents\\WeChat Files\\account-test\\Msg',
      description: '微信用户数据',
      sizeBytes: 4096,
      risk: 'high',
      deleteMode: 'permanent',
    }]);

    const [item] = await scanCleanup();

    expect(item.scope).toBe('wechat');
    expect(item.impact).toBe('user_data');
    expect(item.recoverability).toBe('irreversible');
    expect(item.selectable).toBe(true);
  });

  it('keeps a running browser visible but unavailable for cleanup', async () => {
    invokeMock.mockResolvedValueOnce([{
      id: 'browser-vivaldi-profile-cache',
      category: '浏览器缓存',
      name: 'Vivaldi · Default · Cache',
      path: 'C:\\Users\\Test\\AppData\\Local\\Vivaldi\\User Data\\Default\\Cache',
      description: '可由应用或 Windows 自动重新生成',
      blockedReason: '检测到 Vivaldi 正在运行；为避免误清理正在使用的数据，本次已安全跳过',
      sizeBytes: 8192,
      risk: 'low',
      deleteMode: 'permanent',
    }]);

    const [item] = await scanCleanup();

    expect(item.scope).toBe('browser');
    expect(item.product).toBe('Vivaldi');
    expect(item.selectable).toBe(false);
    expect(item.reason).toContain('Vivaldi 正在运行');
  });

  it('sends irreversible item confirmations separately from the cleanup plan', async () => {
    invokeMock.mockResolvedValueOnce({ reclaimedBytes: 0, succeeded: 0, failed: [] });

    await executeCleanup(['safe-cache', 'wechat-user-data'], ['wechat-user-data']);

    expect(invokeMock).toHaveBeenCalledWith('execute_cleanup', {
      request: {
        itemIds: ['safe-cache', 'wechat-user-data'],
        confirmed: true,
        confirmedIrreversibleItemIds: ['wechat-user-data'],
      },
    });
  });

  it('loads the validated physical disk layout through a typed Tauri command', async () => {
    invokeMock.mockResolvedValueOnce([]);

    await expect(loadPartitionDisks()).resolves.toEqual([]);

    expect(invokeMock).toHaveBeenCalledWith('list_partition_disks');
  });
});
