// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { loadAppIcon, loadApps } from './native';

const iconDataUrl = 'data:image/png;base64,AA==';

function iconInvocationCount(): number {
  return invokeMock.mock.calls.filter(([command]) => command === 'get_app_icon').length;
}

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
});
