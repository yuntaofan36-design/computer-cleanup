// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SelectMenu from './SelectMenu';

afterEach(cleanup);

const options = [
  { value: 'c', label: '系统盘 (C:)' },
  { value: 'd', label: '数据盘 (D:)' },
  { value: 'e', label: '备份盘 (E:)' },
];

describe('SelectMenu', () => {
  it('renders a custom listbox and selects an option without a native select', () => {
    const onChange = vi.fn();
    const { container } = render(
      <SelectMenu
        ariaLabel="选择磁盘"
        label="当前磁盘"
        value="c"
        options={options}
        onChange={onChange}
      />,
    );

    const trigger = screen.getByRole('combobox', { name: '选择磁盘' });
    fireEvent.click(trigger);

    expect(screen.getAllByRole('option')).toHaveLength(3);
    expect(screen.getByRole('option', { name: '系统盘 (C:)' }).getAttribute('aria-selected')).toBe('true');
    fireEvent.click(screen.getByRole('option', { name: '数据盘 (D:)' }));

    expect(onChange).toHaveBeenCalledWith('d');
    expect(screen.queryByRole('listbox')).toBeNull();
    expect(container.querySelector('select')).toBeNull();
  });

  it('supports keyboard navigation and keeps disabled controls closed', () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <SelectMenu
        ariaLabel="排序方式"
        value="c"
        options={options}
        onChange={onChange}
      />,
    );
    const trigger = screen.getByRole('combobox', { name: '排序方式' });

    fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    fireEvent.keyDown(trigger, { key: 'Enter' });
    expect(onChange).toHaveBeenCalledWith('d');

    rerender(
      <SelectMenu
        ariaLabel="排序方式"
        value="c"
        options={options}
        disabled
        onChange={onChange}
      />,
    );
    fireEvent.click(screen.getByRole('combobox', { name: '排序方式' }));
    expect(screen.queryByRole('listbox')).toBeNull();
  });
});
