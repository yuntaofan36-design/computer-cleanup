// @vitest-environment jsdom

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import Toolbox from './Toolbox';

describe('Toolbox', () => {
  it('routes the four primary tools to their real destinations', () => {
    const onOpenFileDiscovery = vi.fn();
    const onNavigate = vi.fn();
    render(<Toolbox
      largeFileCount={6}
      duplicateGroupCount={3}
      appCount={5}
      startupCount={3}
      analyzedDirectoryCount={5}
      onOpenFileDiscovery={onOpenFileDiscovery}
      onNavigate={onNavigate}
    />);

    fireEvent.click(screen.getByRole('button', { name: /重复文件/ }));
    expect(onOpenFileDiscovery).toHaveBeenCalledWith('duplicates');
    fireEvent.click(screen.getByRole('button', { name: /大文件/ }));
    expect(onOpenFileDiscovery).toHaveBeenCalledWith('large-files');
    fireEvent.click(screen.getByRole('button', { name: /磁盘分析/ }));
    expect(onNavigate).toHaveBeenCalledWith('analysis');
    fireEvent.click(screen.getByRole('button', { name: /启动项/ }));
    expect(onNavigate).toHaveBeenCalledWith('startup');
  });
});
