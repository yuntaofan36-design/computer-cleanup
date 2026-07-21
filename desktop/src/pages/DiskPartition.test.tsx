// @vitest-environment jsdom

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { partitionDisks } from '../mockData';
import type { PartitionDisk } from '../types';
import DiskPartition, { buildPartitionSegments } from './DiskPartition';

describe('disk partition layout', () => {
  it('ignores tiny metadata gaps and reports meaningful unallocated space', () => {
    const MB = 1024 ** 2;
    const disk: PartitionDisk = {
      ...partitionDisks[0],
      sizeBytes: 1000 * MB,
      partitions: [{
        ...partitionDisks[0].partitions[1],
        partitionNumber: 1,
        offsetBytes: 1 * MB,
        sizeBytes: 600 * MB,
      }],
    };

    const segments = buildPartitionSegments(disk);

    expect(segments).toHaveLength(2);
    expect(segments[0].kind).toBe('partition');
    expect(segments[1].kind).toBe('unallocated');
    expect(segments[1].sizeBytes).toBe(399 * MB);
  });

  it('renders physical disks and delegates mutations to Windows Disk Management', () => {
    const onOpenDiskManagement = vi.fn();
    render(
      <DiskPartition
        disks={partitionDisks}
        loading={false}
        error=""
        onRefresh={vi.fn()}
        onOpenDiskManagement={onOpenDiskManagement}
      />,
    );

    expect(screen.getByText(/磁盘 0 · Qingpan NVMe/)).toBeTruthy();
    expect(screen.getAllByText('未分配空间').length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('button', { name: /打开磁盘管理/ }));
    expect(onOpenDiskManagement).toHaveBeenCalledOnce();
  });
});
