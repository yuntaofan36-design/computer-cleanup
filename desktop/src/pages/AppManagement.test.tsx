// @vitest-environment jsdom

import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { AppEntry } from '../types';
import AppManagement from './AppManagement';

const installedApp: AppEntry = {
  id: 'app_test',
  name: 'Example App',
  publisher: 'Example Publisher',
  version: '1.0.0',
  sizeBytes: 1024,
  cacheBytes: 0,
  installedAt: '2026-07-20',
  lastUsed: '今天',
  uninstallable: true,
};

describe('AppManagement confirmation dialog', () => {
  it('mounts the uninstall dialog at document.body so it stays viewport-centered', () => {
    render(
      <AppManagement
        apps={[installedApp]}
        onRequestUninstall={vi.fn()}
        onClearCache={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '调用官方卸载器' }));

    const dialog = screen.getByRole('dialog', {
      name: '调用 Example App 的官方卸载器？',
    });
    const overlay = dialog.parentElement;
    expect(overlay).toHaveClass('overlay');
    expect(overlay?.parentElement).toBe(document.body);
  });
});
