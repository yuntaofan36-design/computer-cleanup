// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { largeFiles } from '../mockData';
import type { LargeFileDeleteProgress } from '../types';
import FileDiscovery from './FileDiscovery';

afterEach(cleanup);

describe('large-file cleanup experience', () => {
  it('matches the result-first layout and keeps protected files unavailable', () => {
    const { container } = render(
      <FileDiscovery largeFiles={largeFiles} duplicateGroups={[]} scanStatus='complete' />,
    );

    expect(screen.getByRole('heading', { name: /发现 6 个大文件，共/ })).toBeTruthy();
    expect((screen.getByRole('button', { name: /一键清理/ }) as HTMLButtonElement).disabled).toBe(true);
    const categories = container.querySelector('.large-file-category-tabs');
    expect(categories).toBeTruthy();
    expect(within(categories as HTMLElement).getByRole('tab', { name: /视频/ })).toBeTruthy();
    expect((screen.getByRole('checkbox', { name: /ext4.vhdx 受保护，不可选择/ }) as HTMLInputElement).disabled).toBe(true);
  });

  it('filters by category without losing the large-file summary', () => {
    const { container } = render(
      <FileDiscovery largeFiles={largeFiles} duplicateGroups={[]} scanStatus='complete' />,
    );
    const categories = container.querySelector('.large-file-category-tabs') as HTMLElement;

    fireEvent.click(within(categories).getByRole('tab', { name: /视频/ }));

    expect(screen.getByText('launch-film-final.mov')).toBeTruthy();
    expect(screen.getByText('screen-recording-0412.mp4')).toBeTruthy();
    expect(screen.queryByText('Windows11_24H2.iso')).toBeNull();
    expect(screen.getByRole('heading', { name: /发现 6 个大文件，共/ })).toBeTruthy();
  });

  it('offers the requested size thresholds and combines them with category filters', () => {
    const { container } = render(
      <FileDiscovery largeFiles={largeFiles} duplicateGroups={[]} scanStatus='complete' />,
    );
    const sizeFilter = screen.getByRole('combobox', { name: '大文件最小大小' });
    fireEvent.click(sizeFilter);

    expect(screen.getAllByRole('option').map((option) => option.textContent)).toEqual([
      '≥100MB',
      '≥500MB',
      '≥1GB',
      '≥5GB',
      '≥10GB',
    ]);

    fireEvent.click(screen.getByRole('option', { name: '≥10GB' }));

    expect(screen.getByText('ext4.vhdx')).toBeTruthy();
    expect(screen.getByText('launch-film-final.mov')).toBeTruthy();
    expect(screen.queryByText('Windows11_24H2.iso')).toBeNull();
    expect(screen.queryByText('screen-recording-0412.mp4')).toBeNull();

    const categories = container.querySelector('.large-file-category-tabs') as HTMLElement;
    fireEvent.click(within(categories).getByRole('tab', { name: /视频/ }));

    expect(screen.getByText('launch-film-final.mov')).toBeTruthy();
    expect(screen.queryByText('ext4.vhdx')).toBeNull();
  });

  it('requires permanent confirmation and reports the real delete result', async () => {
    const onDeleteLargeFiles = vi.fn(async (
      ids: string[],
      onProgress: (progress: LargeFileDeleteProgress) => void,
    ) => {
      const item = largeFiles.find((entry) => entry.id === ids[0])!;
      onProgress({
        phase: 'running',
        completed: 0,
        total: ids.length,
        currentItemId: item.id,
        currentName: item.name,
        currentPath: item.path,
        deletedBytes: 0,
        failed: 0,
      });
      return { deletedBytes: item.sizeBytes, succeededIds: ids, failed: [] };
    });
    render(
      <FileDiscovery
        largeFiles={largeFiles}
        duplicateGroups={[]}
        scanStatus='complete'
        onDeleteLargeFiles={onDeleteLargeFiles}
      />,
    );

    fireEvent.click(screen.getByRole('checkbox', { name: '选择 launch-film-final.mov' }));
    fireEvent.click(screen.getByRole('button', { name: /一键清理/ }));
    const confirmButton = screen.getByRole('button', { name: '永久删除所选文件' });
    expect((confirmButton as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole('checkbox', { name: '我确认永久删除以上已选择的大文件' }));
    fireEvent.click(confirmButton);

    await waitFor(() => expect(onDeleteLargeFiles).toHaveBeenCalledWith(
      ['lf3'],
      expect.any(Function),
    ));
    expect(await screen.findByRole('heading', { name: '大文件清理完成' })).toBeTruthy();
    expect(screen.getByText('所选大文件已永久删除')).toBeTruthy();
    expect(screen.getByText('1 个')).toBeTruthy();
  });
});
