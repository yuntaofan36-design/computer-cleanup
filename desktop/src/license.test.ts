// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { activateLicense, validateLicense } from './license';

describe('offline activation key', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it('activates 996436416 without requesting the server', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');

    await expect(activateLicense('996436416')).resolves.toEqual({ offline: true });
    await expect(validateLicense()).resolves.toBe(true);
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
