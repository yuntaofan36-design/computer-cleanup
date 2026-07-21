import { useEffect, useMemo, useState } from 'react';
import { AlertTriangle, CheckCircle2, ExternalLink, HardDrive, LoaderCircle, RefreshCw, ShieldCheck } from 'lucide-react';
import { EmptyState, PageHeader, SafetyNotice } from '../components';
import { formatBytes } from '../format';
import type { DiskPartition as DiskPartitionInfo, PartitionDisk } from '../types';

const MIN_VISIBLE_UNALLOCATED_BYTES = 8 * 1024 ** 2;

export interface PartitionSegment {
  id: string;
  kind: 'partition' | 'unallocated';
  offsetBytes: number;
  sizeBytes: number;
  partition?: DiskPartitionInfo;
}

interface DiskPartitionProps {
  disks: PartitionDisk[];
  loading: boolean;
  error: string;
  onRefresh: () => void;
  onOpenDiskManagement: () => void;
}

export function buildPartitionSegments(disk: PartitionDisk): PartitionSegment[] {
  const partitions = [...disk.partitions].sort((left, right) => left.offsetBytes - right.offsetBytes);
  const segments: PartitionSegment[] = [];
  let cursor = 0;
  for (const partition of partitions) {
    const gap = Math.max(0, partition.offsetBytes - cursor);
    if (gap >= MIN_VISIBLE_UNALLOCATED_BYTES) {
      segments.push({
        id: `disk-${disk.number}-unallocated-${cursor}`,
        kind: 'unallocated',
        offsetBytes: cursor,
        sizeBytes: gap,
      });
    }
    segments.push({
      id: `disk-${disk.number}-partition-${partition.partitionNumber}`,
      kind: 'partition',
      offsetBytes: partition.offsetBytes,
      sizeBytes: partition.sizeBytes,
      partition,
    });
    cursor = Math.max(cursor, partition.offsetBytes + partition.sizeBytes);
  }
  const tail = Math.max(0, disk.sizeBytes - cursor);
  if (tail >= MIN_VISIBLE_UNALLOCATED_BYTES) {
    segments.push({
      id: `disk-${disk.number}-unallocated-${cursor}`,
      kind: 'unallocated',
      offsetBytes: cursor,
      sizeBytes: tail,
    });
  }
  return segments;
}

function partitionName(partition: DiskPartitionInfo): string {
  if (partition.driveLetter) return `${partition.driveLetter}:`;
  if (partition.isSystem) return 'EFI 系统分区';
  if (partition.partitionType.toLowerCase().includes('recovery')) return '恢复分区';
  return `分区 ${partition.partitionNumber}`;
}

function partitionClass(partition: DiskPartitionInfo): string {
  if (partition.isSystem) return 'system';
  if (partition.isBoot) return 'boot';
  if (partition.partitionType.toLowerCase().includes('recovery')) return 'recovery';
  if (partition.isHidden) return 'hidden';
  return 'basic';
}

function healthLabel(status: string): string {
  const normalized = status.toLowerCase();
  if (normalized === 'healthy') return '正常';
  if (normalized === 'warning') return '警告';
  if (normalized === 'unhealthy') return '异常';
  return status || '未知';
}

function partitionFlags(partition: DiskPartitionInfo): string[] {
  const flags: string[] = [];
  if (partition.isBoot) flags.push('启动');
  if (partition.isSystem) flags.push('系统');
  if (partition.isHidden) flags.push('隐藏');
  if (partition.isReadOnly) flags.push('只读');
  if (partition.noDefaultDriveLetter) flags.push('无默认盘符');
  return flags;
}

export default function DiskPartition({ disks, loading, error, onRefresh, onOpenDiskManagement }: DiskPartitionProps) {
  const allSegments = useMemo(() => disks.flatMap(buildPartitionSegments), [disks]);
  const partitions = disks.flatMap((disk) => disk.partitions);
  const unallocatedBytes = allSegments
    .filter((segment) => segment.kind === 'unallocated')
    .reduce((total, segment) => total + segment.sizeBytes, 0);
  const [selectedId, setSelectedId] = useState('');

  useEffect(() => {
    if (!allSegments.some((segment) => segment.id === selectedId)) {
      setSelectedId(allSegments.find((segment) => segment.kind === 'partition')?.id || allSegments[0]?.id || '');
    }
  }, [allSegments, selectedId]);

  return <section className="page partition-page">
    <PageHeader
      eyebrow="物理磁盘 → 分区边界 → 卷状态"
      title="磁盘分区"
      description="查看物理磁盘布局、卷状态与未分配空间。"
      actions={<>
        <button className="button secondary" onClick={onRefresh} disabled={loading}><RefreshCw className={loading ? 'spin' : ''} />刷新布局</button>
        <button className="button primary" onClick={onOpenDiskManagement}><ExternalLink />打开磁盘管理</button>
      </>}
    />

    <SafetyNotice tone="info"><strong>分区写入由 Windows 官方组件执行</strong><p>当前页面只读。新建、压缩、扩展、格式化和删除分区将在 Windows 磁盘管理中完成，并继续接受 UAC 与系统卷保护。</p></SafetyNotice>

    {loading && !disks.length ? <div className="partition-loading"><LoaderCircle className="spin" /><strong>正在读取物理磁盘布局</strong></div> : error && !disks.length ? <EmptyState icon={<AlertTriangle />} title="无法读取磁盘布局" description={error} action={<button className="button secondary" onClick={onRefresh}><RefreshCw />重试</button>} /> : !disks.length ? <EmptyState icon={<HardDrive />} title="没有发现可管理的磁盘" description="Windows Storage API 没有返回物理磁盘。" /> : <>
      {error && <div className="partition-inline-error"><AlertTriangle /><span>{error}</span></div>}
      <div className="partition-summary">
        <div><small>物理磁盘</small><strong>{disks.length}</strong></div>
        <div><small>已识别分区</small><strong>{partitions.length}</strong></div>
        <div><small>未分配空间</small><strong>{formatBytes(unallocatedBytes)}</strong></div>
        <span><ShieldCheck />{disks.filter((disk) => disk.healthStatus.toLowerCase() === 'healthy').length} 块磁盘状态正常</span>
      </div>

      <div className="partition-disk-list">{disks.map((disk) => {
        const segments = buildPartitionSegments(disk);
        const selected = segments.find((segment) => segment.id === selectedId);
        return <article className="partition-disk" key={disk.number}>
          <header>
            <div className="partition-disk-title"><span><HardDrive /></span><div><strong>磁盘 {disk.number} · {disk.friendlyName || '未知设备'}</strong><small>{disk.busType || '未知总线'} · {disk.partitionStyle || '未知样式'} · {formatBytes(disk.sizeBytes)}</small></div></div>
            <div className="partition-disk-status"><span className={disk.healthStatus.toLowerCase() === 'healthy' ? 'healthy' : 'warning'}><CheckCircle2 />{healthLabel(disk.healthStatus)}</span>{disk.isBoot && <b>启动磁盘</b>}{disk.isReadOnly && <b>只读</b>}{disk.isOffline && <b>脱机</b>}</div>
          </header>

          <div className="partition-map" aria-label={`磁盘 ${disk.number} 分区布局`}>{segments.map((segment) => {
            const left = disk.sizeBytes > 0 ? (segment.offsetBytes / disk.sizeBytes) * 100 : 0;
            const width = disk.sizeBytes > 0 ? (segment.sizeBytes / disk.sizeBytes) * 100 : 0;
            const label = segment.partition ? partitionName(segment.partition) : '未分配';
            const className = segment.partition ? partitionClass(segment.partition) : 'unallocated';
            return <button
              type="button"
              key={segment.id}
              className={`partition-segment ${className} ${selectedId === segment.id ? 'active' : ''}`}
              style={{ left: `${left}%`, width: `${width}%` }}
              onClick={() => setSelectedId(segment.id)}
              aria-label={`${label}，${formatBytes(segment.sizeBytes)}`}
              title={`${label} · ${formatBytes(segment.sizeBytes)}`}
            ><strong>{label}</strong><small>{formatBytes(segment.sizeBytes)}</small></button>;
          })}</div>

          <div className="partition-grid">
            <div className="partition-table">
              <div className="partition-table-head"><span>卷 / 分区</span><span>文件系统</span><span>容量</span><span>可用</span><span>状态</span></div>
              {segments.map((segment) => segment.partition ? <button type="button" className={`partition-table-row ${selectedId === segment.id ? 'active' : ''}`} key={segment.id} onClick={() => setSelectedId(segment.id)}>
                <span><b>{partitionName(segment.partition)}</b><small>{segment.partition.label || segment.partition.partitionType || '普通分区'}</small></span>
                <span>{segment.partition.fileSystem || '--'}</span>
                <span>{formatBytes(segment.sizeBytes)}</span>
                <span>{segment.partition.fileSystem ? formatBytes(segment.partition.freeBytes) : '--'}</span>
                <span>{healthLabel(segment.partition.healthStatus)}</span>
              </button> : <button type="button" className={`partition-table-row unallocated ${selectedId === segment.id ? 'active' : ''}`} key={segment.id} onClick={() => setSelectedId(segment.id)}>
                <span><b>未分配空间</b><small>可用于新建或扩展分区</small></span><span>--</span><span>{formatBytes(segment.sizeBytes)}</span><span>--</span><span>未使用</span>
              </button>)}
            </div>

            <aside className="partition-detail">{selected ? selected.partition ? <>
              <p className="eyebrow">当前选择</p><h2>{partitionName(selected.partition)}</h2><p>{selected.partition.label || '无卷标'}</p>
              <dl>
                <div><dt>分区位置</dt><dd>磁盘 {disk.number} · 分区 {selected.partition.partitionNumber}</dd></div>
                <div><dt>文件系统</dt><dd>{selected.partition.fileSystem || '未识别'}</dd></div>
                <div><dt>容量 / 可用</dt><dd>{formatBytes(selected.partition.sizeBytes)} / {selected.partition.fileSystem ? formatBytes(selected.partition.freeBytes) : '--'}</dd></div>
                <div><dt>保护属性</dt><dd>{partitionFlags(selected.partition).join('、') || '普通数据分区'}</dd></div>
              </dl>
            </> : <><p className="eyebrow">当前选择</p><h2>未分配空间</h2><p>{formatBytes(selected.sizeBytes)}</p><div className="partition-unallocated-proof"><ShieldCheck /><span>尚未建立文件系统或盘符</span></div></> : <p>选择分区查看详情</p>}</aside>
          </div>
        </article>;
      })}</div>
    </>}
  </section>;
}
