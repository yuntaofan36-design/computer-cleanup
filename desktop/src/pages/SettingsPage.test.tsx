// @vitest-environment jsdom

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import SettingsPage from './SettingsPage';

describe('SettingsPage exclusions', () => {
  it('adds and removes user-managed exclusion paths', () => {
    const onAddExclusion = vi.fn();
    const onRemoveExclusion = vi.fn();
    render(<SettingsPage
      protectedDirectories={['C:\\Users\\User\\Documents']}
      builtInExclusionRules={['*.pst']}
      userExclusions={['D:\\Work']}
      onAddExclusion={onAddExclusion}
      onRemoveExclusion={onRemoveExclusion}
      theme="system"
      setTheme={vi.fn()}
    />);

    fireEvent.change(screen.getByPlaceholderText('例如 D:\\Work\\Important'), { target: { value: 'E:\\Archive' } });
    fireEvent.click(screen.getByRole('button', { name: '加入' }));
    expect(onAddExclusion).toHaveBeenCalledWith('E:\\Archive');

    fireEvent.click(screen.getByRole('button', { name: '移除排除路径 D:\\Work' }));
    expect(onRemoveExclusion).toHaveBeenCalledWith('D:\\Work');
  });
});
