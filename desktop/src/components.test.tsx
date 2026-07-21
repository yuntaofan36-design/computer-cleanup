// @vitest-environment jsdom

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Dialog } from './components';

describe('destructive confirmation dialog', () => {
  it('keeps the destructive action disabled until irreversible data is acknowledged', () => {
    const { rerender } = render(
      <Dialog
        title="确认执行"
        confirmLabel="永久删除所选内容"
        confirmDisabled
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      >
        <p>微信用户数据</p>
      </Dialog>,
    );

    const confirm = screen.getByRole('button', { name: '永久删除所选内容' }) as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);

    rerender(
      <Dialog
        title="确认执行"
        confirmLabel="永久删除所选内容"
        confirmDisabled={false}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      >
        <p>微信用户数据</p>
      </Dialog>,
    );
    expect(confirm.disabled).toBe(false);
  });
});
