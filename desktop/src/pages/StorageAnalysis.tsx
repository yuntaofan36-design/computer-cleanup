import { useEffect, useMemo, useState } from 'react';
import {
  ChevronRight,
  CircleStop,
  Folder,
  HardDrive,
  Info,
  ScanSearch,
  ShieldCheck,
} from 'lucide-react';
import { formatBytes } from '../format';
import type { DirectoryUsage, DiskInfo, StorageCategory } from '../types';

export interface StorageAnalysisProps {
  disk: DiskInfo;
  directories: readonly DirectoryUsage[];
  categories: readonly StorageCategory[];
  initialPath?: string;
  scanStatus?: 'idle' | 'scanning' | 'complete';
  scannedAt?: string;
  onScan?: () => void;
  onCancel?: () => void;
  onAnalyzeDirectory?: (directory: DirectoryUsage) => void;
}

interface BreadcrumbItem {
  label: string;
  path: string;
}

interface TreemapRect {
  entry: DirectoryUsage;
  x: number;
  y: number;
  width: number;
  height: number;
}

function normalizePath(path: string): string {
  let normalized = path.trim().split('/').join('\\');
  if (normalized.toLocaleLowerCase().startsWith('\\\\?\\unc\\')) {
    normalized = `\\\\${normalized.slice(8)}`;
  } else if (normalized.startsWith('\\\\?\\')) {
    normalized = normalized.slice(4);
  }
  if (/^[a-z]:$/i.test(normalized)) return `${normalized}\\`;
  if (/^[a-z]:\\$/i.test(normalized)) return normalized;
  return normalized.replace(/\\+$/, '');
}

function isDirectChild(parentPath: string, candidatePath: string): boolean {
  const parent = normalizePath(parentPath);
  const candidate = normalizePath(candidatePath);
  const prefix = parent.endsWith('\\') ? parent : `${parent}\\`;
  if (!candidate.toLocaleLowerCase().startsWith(prefix.toLocaleLowerCase())) return false;
  const remainder = candidate.slice(prefix.length);
  return Boolean(remainder) && !remainder.includes('\\');
}

function buildBreadcrumbs(rootPath: string, currentPath: string): BreadcrumbItem[] {
  const root = normalizePath(rootPath);
  const current = normalizePath(currentPath);
  const items: BreadcrumbItem[] = [{ label: root, path: root }];
  if (current.toLocaleLowerCase() === root.toLocaleLowerCase()) return items;
  if (!current.toLocaleLowerCase().startsWith(root.toLocaleLowerCase())) {
    return [{ label: current, path: current }];
  }
  const relative = current.slice(root.length).replace(/^\\+/, '');
  let accumulated = root;
  relative.split('\\').filter(Boolean).forEach((segment) => {
    accumulated = accumulated.endsWith('\\') ? `${accumulated}${segment}` : `${accumulated}\\${segment}`;
    items.push({ label: segment, path: accumulated });
  });
  return items;
}

function layoutTreemap(
  entries: readonly DirectoryUsage[],
  x = 0,
  y = 0,
  width = 760,
  height = 320,
): TreemapRect[] {
  if (!entries.length) return [];
  if (entries.length === 1) return [{ entry: entries[0], x, y, width, height }];
  const total = entries.reduce((sum, entry) => sum + Math.max(0, entry.sizeBytes), 0);
  if (!total) return [];
  let firstTotal = 0;
  let splitIndex = 1;
  for (let index = 0; index < entries.length - 1; index += 1) {
    firstTotal += Math.max(0, entries[index].sizeBytes);
    splitIndex = index + 1;
    if (firstTotal >= total / 2) break;
  }
  const first = entries.slice(0, splitIndex);
  const second = entries.slice(splitIndex);
  const ratio = Math.min(0.95, Math.max(0.05, firstTotal / total));
  if (width >= height) {
    const firstWidth = width * ratio;
    return [
      ...layoutTreemap(first, x, y, firstWidth, height),
      ...layoutTreemap(second, x + firstWidth, y, width - firstWidth, height),
    ];
  }
  const firstHeight = height * ratio;
  return [
    ...layoutTreemap(first, x, y, width, firstHeight),
    ...layoutTreemap(second, x, y + firstHeight, width, height - firstHeight),
  ];
}

function DirectoryTreemap({
  entries,
  onDrill,
}: {
  entries: readonly DirectoryUsage[];
  onDrill: (entry: DirectoryUsage) => void;
}): JSX.Element {
  const rectangles = useMemo(
    () => layoutTreemap(entries.slice().sort((a, b) => b.sizeBytes - a.sizeBytes)),
    [entries],
  );
  if (!rectangles.length) {
    return (
      <div className='analysis-empty treemap-empty'>
        <Folder />
        <p>当前目录没有可显示的下一级目录。</p>
      </div>
    );
  }
  return (
    <svg
      className='directory-treemap'
      viewBox='0 0 760 320'
      role='img'
      aria-label='目录空间占用矩形树图'
    >
      {rectangles.map(({ entry, x, y, width, height }) => {
        const showLabel = width > 90 && height > 48;
        const showSize = width > 120 && height > 76;
        return (
          <g
            className='treemap-node'
            key={entry.id}
            role='button'
            tabIndex={0}
            aria-label={`分析 ${entry.path}，占用 ${formatBytes(entry.sizeBytes)}`}
            onClick={() => onDrill(entry)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') onDrill(entry);
            }}
          >
            <rect
              className='treemap-rect'
              x={x + 2}
              y={y + 2}
              width={Math.max(0, width - 4)}
              height={Math.max(0, height - 4)}
              rx={6}
              fill={entry.color}
            />
            <title>{entry.path} · {formatBytes(entry.sizeBytes)} · {entry.fileCount} 个文件</title>
            {showLabel && (
              <text className='treemap-name' x={x + 13} y={y + 24}>
                {entry.name}
              </text>
            )}
            {showSize && (
              <text className='treemap-size' x={x + 13} y={y + 45}>
                {formatBytes(entry.sizeBytes)}
              </text>
            )}
          </g>
        );
      })}
    </svg>
  );
}

function CategoryDonut({
  categories,
}: {
  categories: readonly StorageCategory[];
}): JSX.Element {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const total = categories.reduce((sum, category) => sum + Math.max(0, category.sizeBytes), 0);
  const selected = categories.find((category) => category.id === selectedId) ?? null;
  let offset = 0;
  return (
    <div className='category-analysis'>
      <div className='category-donut-wrap'>
        <svg className='category-donut' viewBox='0 0 152 152' aria-label='存储分类环图'>
          <circle className='donut-track' cx={76} cy={76} r={52} fill='none' pathLength={100} />
          {categories.map((category) => {
            const share = total ? category.sizeBytes / total * 100 : 0;
            const dashOffset = -offset;
            offset += share;
            return (
              <circle
                key={category.id}
                className={`donut-segment ${selectedId === category.id ? 'active' : ''}`}
                cx={76}
                cy={76}
                r={52}
                fill='none'
                pathLength={100}
                stroke={category.color}
                strokeDasharray={`${share} ${100 - share}`}
                strokeDashoffset={dashOffset}
                transform='rotate(-90 76 76)'
                role='button'
                tabIndex={0}
                aria-label={`${category.label}，${formatBytes(category.sizeBytes)}`}
                onClick={() => setSelectedId(category.id)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') setSelectedId(category.id);
                }}
              />
            );
          })}
          <text className='donut-total-label' x={76} y={70} textAnchor='middle'>已分析</text>
          <text className='donut-total-value' x={76} y={91} textAnchor='middle'>{formatBytes(total)}</text>
        </svg>
      </div>
      <div className='category-legend'>
        {categories.map((category) => (
          <button
            className={selectedId === category.id ? 'active' : ''}
            type='button'
            key={category.id}
            onClick={() => setSelectedId(category.id)}
          >
            <svg className='category-swatch' viewBox='0 0 10 10' aria-hidden='true'>
              <circle cx={5} cy={5} r={5} fill={category.color} />
            </svg>
            <span>{category.label}</span>
            <strong>{formatBytes(category.sizeBytes)}</strong>
          </button>
        ))}
      </div>
      <p className='category-description'>
        {selected ? `${selected.label}：${selected.description}` : '点击环图或图例查看分类说明；此操作仅分析数据。'}
      </p>
    </div>
  );
}

export default function StorageAnalysis({
  disk,
  directories,
  categories,
  initialPath,
  scanStatus = 'idle',
  scannedAt,
  onScan,
  onCancel,
  onAnalyzeDirectory,
}: StorageAnalysisProps): JSX.Element {
  const rootPath = normalizePath(initialPath ?? disk.mount);
  const [currentPath, setCurrentPath] = useState(rootPath);

  useEffect(() => {
    setCurrentPath(rootPath);
  }, [rootPath]);

  const usedBytes = Math.max(0, disk.totalBytes - disk.freeBytes);
  const usedPercent = disk.totalBytes ? Math.min(100, usedBytes / disk.totalBytes * 100) : 0;
  const visibleDirectories = useMemo(
    () => directories
      .filter((entry) => isDirectChild(currentPath, entry.path))
      .slice()
      .sort((a, b) => b.sizeBytes - a.sizeBytes),
    [currentPath, directories],
  );
  const breadcrumbs = useMemo(() => buildBreadcrumbs(rootPath, currentPath), [currentPath, rootPath]);
  const currentDirectory = directories.find(
    (entry) => normalizePath(entry.path).toLocaleLowerCase() === currentPath.toLocaleLowerCase(),
  );
  const visibleBytes = visibleDirectories.reduce((sum, entry) => sum + entry.sizeBytes, 0);
  const scanning = scanStatus === 'scanning';

  const drillTo = (entry: DirectoryUsage): void => {
    setCurrentPath(normalizePath(entry.path));
    onAnalyzeDirectory?.(entry);
  };

  return (
    <section className='storage-analysis-page'>
      <header className='page-head storage-analysis-head'>
        <div>
          <p className='eyebrow'>空间地图</p>
          <h1>磁盘占用分析</h1>
          <p>
            从磁盘到目录逐层定位空间去向。
            {scannedAt && <span className='scan-time'>最近分析：{scannedAt}</span>}
          </p>
        </div>
        <button
          className='button primary'
          type='button'
          disabled={scanning ? !onCancel : !onScan}
          onClick={() => {
            if (scanning) {
              onCancel?.();
            } else {
              setCurrentPath(rootPath);
              onScan?.();
            }
          }}
        >
          {scanning ? <CircleStop size={18} /> : <ScanSearch size={18} />}
          {scanning ? '取消分析' : '重新分析磁盘'}
        </button>
      </header>

      <div className='notice analysis-readonly-notice'>
        <ShieldCheck />
        <div>
          <strong>可视化仅用于分析</strong>
          <span>点击图表和目录只会切换分析范围，不会选择、移动或删除任何文件。</span>
        </div>
      </div>

      <section className='disk-overview-card' aria-label='磁盘占用概览'>
        <header>
          <span className='drive-icon'><HardDrive /></span>
          <div>
            <h2>{disk.name} <small>{disk.mount}</small></h2>
            <p>{formatBytes(disk.freeBytes)} 可用，共 {formatBytes(disk.totalBytes)}</p>
          </div>
          <strong>{Math.round(usedPercent)}%</strong>
        </header>
        <progress
          className='disk-capacity-progress'
          max={Math.max(1, disk.totalBytes)}
          value={usedBytes}
          aria-label={`磁盘已使用 ${Math.round(usedPercent)}%`}
        />
        <div className='disk-overview-stats'>
          <span><small>已使用</small><strong>{formatBytes(usedBytes)}</strong></span>
          <span><small>可用空间</small><strong>{formatBytes(disk.freeBytes)}</strong></span>
          <span><small>总容量</small><strong>{formatBytes(disk.totalBytes)}</strong></span>
        </div>
      </section>

      <div className='storage-analysis-grid'>
        <div className='storage-analysis-main'>
          <section className='treemap-card'>
            <header className='analysis-card-head'>
              <div>
                <h2>目录矩形树图</h2>
                <p>矩形面积对应目录占用，点击色块向下分析。</p>
              </div>
              <span className='analysis-total'>
                <small>当前层级</small>
                <strong>{formatBytes(currentDirectory?.sizeBytes ?? visibleBytes)}</strong>
              </span>
            </header>
            <nav className='path-breadcrumbs' aria-label='当前分析路径'>
              {breadcrumbs.map((item, index) => (
                <span key={item.path}>
                  {index > 0 && <ChevronRight size={14} />}
                  <button
                    type='button'
                    className={index === breadcrumbs.length - 1 ? 'active' : ''}
                    onClick={() => setCurrentPath(item.path)}
                    aria-current={index === breadcrumbs.length - 1 ? 'location' : undefined}
                  >
                    {item.label}
                  </button>
                </span>
              ))}
            </nav>
            <DirectoryTreemap entries={visibleDirectories} onDrill={drillTo} />
            <p className='chart-safety-caption'>
              <Info size={15} /> 色块点击仅进入该目录继续分析，不会触发清理。
            </p>
          </section>

          <section className='directory-table-card'>
            <header className='analysis-card-head'>
              <div>
                <h2>当前目录明细</h2>
                <p>{currentPath}</p>
              </div>
              <span>{visibleDirectories.length} 个子目录</span>
            </header>
            {visibleDirectories.length ? (
              <div className='directory-table'>
                <div className='directory-table-head'>
                  <span>目录</span>
                  <span>类型</span>
                  <span>文件数</span>
                  <span>占用</span>
                  <span>比例</span>
                  <span />
                </div>
                {visibleDirectories.map((entry) => (
                  <button className='directory-table-row' type='button' key={entry.id} onClick={() => drillTo(entry)}>
                    <span className='directory-name'>
                      <span className='folder-icon'><Folder size={17} /></span>
                      <span><strong>{entry.name}</strong><small>{entry.path}</small></span>
                    </span>
                    <span>{entry.kind}</span>
                    <span>{entry.fileCount.toLocaleString()}</span>
                    <strong>{formatBytes(entry.sizeBytes)}</strong>
                    <span>{entry.percent.toFixed(1)}%</span>
                    <ChevronRight size={17} />
                  </button>
                ))}
              </div>
            ) : (
              <div className='analysis-empty directory-empty'>
                <Folder />
                <h3>没有下一级目录数据</h3>
                <p>可通过上方面包屑返回上一层继续查看。</p>
              </div>
            )}
          </section>
        </div>

        <aside className='storage-category-card'>
          <header className='analysis-card-head'>
            <div>
              <h2>按类别查看</h2>
              <p>分类统计当前磁盘中的主要内容。</p>
            </div>
          </header>
          <CategoryDonut categories={categories} />
          <div className='analysis-safety-card'>
            <ShieldCheck />
            <div>
              <strong>分析与清理分离</strong>
              <p>本页没有删除入口。确认文件用途后，再前往相应的安全清理功能。</p>
            </div>
          </div>
        </aside>
      </div>
    </section>
  );
}
