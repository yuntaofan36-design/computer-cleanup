// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import StartupManager from './StartupManager';
import type { StartupEntry } from '../types';

const entry: StartupEntry = {
  id: 'hkcu:OneDrive',
  name: 'Microsoft OneDrive',
  publisher: 'Microsoft',
  command: 'OneDrive.exe /background',
  enabled: true,
  impact: '中',
  scope: '当前用户',
};

describe('StartupManager', () => {
  it('delegates explicit state changes and refreshes', async () => {
    const onToggle = vi.fn().mockResolvedValue(undefined);
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    render(<StartupManager entries={[entry]} onToggle={onToggle} onRefresh={onRefresh} />);

    fireEvent.click(screen.getByRole('switch', { name: '禁用 Microsoft OneDrive' }));
    await waitFor(() => expect(onToggle).toHaveBeenCalledWith(entry.id, false));
    fireEvent.click(screen.getByRole('button', { name: '刷新' }));
    await waitFor(() => expect(onRefresh).toHaveBeenCalledOnce());
  });
});
