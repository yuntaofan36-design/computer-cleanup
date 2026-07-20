import { useMemo, useState } from 'react';
import {
  Ban,
  CircleStop,
  ExternalLink,
  File,
  FileSearch,
  Files,
  FolderX,
  HardDrive,
  Info,
  Search,
  ShieldCheck,
} from 'lucide-react';
import { formatBytes } from '../format';
import type { DuplicateGroup, LargeFileEntry } from '../types';

export type FileDiscoveryTab = 'large-files' | 'duplicates';

export interface FileDiscoveryProps {
  largeFiles: readonly LargeFileEntry[];
  duplicateGroups: readonly DuplicateGroup[];
  initialTab?: FileDiscoveryTab;
  scanStatus?: 'idle' | 'scanning' | 'complete';
  scannedAt?: string;
  onScan?: (tab: FileDiscoveryTab) => void;
  onCancel?: () => void;
  onRevealInExplorer?: (path: string) => void;
  onAddExclusion?: (path: string) => void;
}

type SensitivityFilter = 'all' | LargeFileEntry['sensitivity'];

const GIB = 1024 ** 3;
const sizeFilters = [
  { label: '全部大小', value: 0 },
  { label: '大于 1 GB', value: GIB },
  { label: '大于 5 GB', value: 5 * GIB },
  { label: '大于 10 GB', value: 10 * GIB },
] as const;
const duplicateSizeFilters = [
  { label: '全部可释放空间', value: 0 },
  { label: '可释放 500 MB 以上', value: 500 * 1024 ** 2 },
  { label: '可释放 1 GB 以上', value: GIB },
  { label: '可释放 5 GB 以上', value: 5 * GIB },
] as const;
const sensitivityLabels: Record<LargeFileEntry['sensitivity'], string> = {
  normal: '普通文件',
  attention: '需要留意',
  protected: '受保护',
};

function includesQuery(values: readonly string[], query: string): boolean {
  if (!query) return true;
  return values.some((value) => value.toLocaleLowerCase().includes(query));
}

function ActionButtons({
  path,
  onRevealInExplorer,
  onAddExclusion,
}: {
  path: string;
  onRevealInExplorer?: (path: string) => void;
  onAddExclusion?: (path: string) => void;
}): JSX.Element {
  return (
    <span className='file-row-actions'>
      <button
        className='button secondary small'
        type='button'
        disabled={!onRevealInExplorer}
        onClick={() => onRevealInExplorer?.(path)}
        title='仅在 Windows 文件资源管理器中定位，不执行任何文件操作'
      >
        <ExternalLink size={15} /> 查看位置
      </button>
      <button
        className='button secondary small'
        type='button'
        disabled={!onAddExclusion}
        onClick={() => onAddExclusion?.(path)}
        title='加入排除清单后，后续扫描将跳过此路径'
      >
        <FolderX size={15} /> 加入排除
      </button>
    </span>
  );
}

function LargeFilesPanel({
  entries,
  totalBytes,
  onRevealInExplorer,
  onAddExclusion,
}: {
  entries: readonly LargeFileEntry[];
  totalBytes: number;
  onRevealInExplorer?: (path: string) => void;
  onAddExclusion?: (path: string) => void;
}): JSX.Element {
  if (!entries.length) {
    return (
      <div className='discovery-empty'>
        <FileSearch />
        <h2>没有符合条件的大文件</h2>
        <p>调整关键字或筛选条件，当前结果不会触发任何清理操作。</p>
      </div>
    );
  }
  return (
    <>
      <div className='discovery-result-summary' aria-live='polite'>
        <span>找到 <strong>{entries.length}</strong> 个文件</span>
        <span>文件大小合计 <strong>{formatBytes(totalBytes)}</strong></span>
        <span className='readonly-mark'><Ban size={14} /> 只读结果，无默认选择</span>
      </div>
      <div className='file-table-wrap'>
        <table className='file-table'>
          <thead>
            <tr>
              <th>文件</th>
              <th>大小 / 占用</th>
              <th>修改时间</th>
              <th>安全提示</th>
              <th><span className='visually-hidden'>操作</span></th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => (
              <tr key={entry.id}>
                <td>
                  <span className='file-primary-cell'>
                    <span className='file-type-icon'><File size={18} /></span>
                    <span>
                      <strong>{entry.name}</strong>
                      <small title={entry.path}>{entry.path}</small>
                      <em>{entry.type || '未知类型'}</em>
                    </span>
                  </span>
                </td>
                <td>
                  <span className='file-size-cell'>
                    <strong>{formatBytes(entry.sizeBytes)}</strong>
                    <small>磁盘占用 {formatBytes(entry.allocatedBytes)}</small>
                  </span>
                </td>
                <td><span className='file-date'>{entry.modifiedAt}</span></td>
                <td>
                  <span className={`file-safety-badge ${entry.sensitivity}`}>
                    {sensitivityLabels[entry.sensitivity]}
                  </span>
                  {entry.note && <small className='file-note'>{entry.note}</small>}
                </td>
                <td>
                  <ActionButtons
                    path={entry.path}
                    onRevealInExplorer={onRevealInExplorer}
                    onAddExclusion={onAddExclusion}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

function DuplicateGroupsPanel({
  groups,
  reclaimableBytes,
  onRevealInExplorer,
  onAddExclusion,
}: {
  groups: readonly DuplicateGroup[];
  reclaimableBytes: number;
  onRevealInExplorer?: (path: string) => void;
  onAddExclusion?: (path: string) => void;
}): JSX.Element {
  if (!groups.length) {
    return (
      <div className='discovery-empty'>
        <Files />
        <h2>没有符合条件的重复文件组</h2>
        <p>只有通过完整内容哈希校验的文件才会出现在这里。</p>
      </div>
    );
  }
  return (
    <>
      <div className='hash-explainer'>
        <ShieldCheck />
        <div>
          <h2>完整哈希确认，而不是“看起来相同”</h2>
          <p>
            清盘先比较文件大小，再读取每个文件的全部内容计算哈希；只有完整哈希一致才归为同组。
            文件名、扩展名或修改时间相同都不构成重复判定。结果仍为只读，建议先查看位置并自行确认用途。
          </p>
        </div>
      </div>
      <div className='discovery-result-summary' aria-live='polite'>
        <span><strong>{groups.length}</strong> 组已验证重复项</span>
        <span>理论可释放 <strong>{formatBytes(reclaimableBytes)}</strong></span>
        <span className='readonly-mark'><Ban size={14} /> 不预选、不提供直接删除</span>
      </div>
      <div className='duplicate-groups'>
        {groups.map((group) => (
          <article className='duplicate-group' key={group.id}>
            <header className='duplicate-group-head'>
              <span className='duplicate-icon'><Files /></span>
              <div>
                <h2>{group.members[0]?.name ?? '重复文件组'}</h2>
                <p>
                  {group.members.length} 个完全相同的文件 · 单个 {formatBytes(group.sizeBytes)} ·
                  理论可释放 {formatBytes(group.reclaimableBytes)}
                </p>
              </div>
              <span className='verified-badge'><ShieldCheck size={14} /> 已完整校验</span>
            </header>
            <div className='duplicate-hash'>
              <span>完整内容哈希</span>
              <code>{group.hash}</code>
              <small>此值来自文件全部字节；相同哈希用于确认内容一致，不代表其中任一副本可以安全删除。</small>
            </div>
            <div className='duplicate-members'>
              {group.members.map((member) => (
                <div className='duplicate-member' key={member.id}>
                  <span className='duplicate-member-state'>
                    <File size={17} />
                    {member.suggestedKeep && <b>建议保留</b>}
                    {member.protected && <b className='protected'>受保护</b>}
                  </span>
                  <span className='duplicate-member-main'>
                    <strong>{member.name}</strong>
                    <small title={member.path}>{member.path}</small>
                  </span>
                  <time>{member.modifiedAt}</time>
                  <ActionButtons
                    path={member.path}
                    onRevealInExplorer={onRevealInExplorer}
                    onAddExclusion={onAddExclusion}
                  />
                </div>
              ))}
            </div>
          </article>
        ))}
      </div>
    </>
  );
}

export default function FileDiscovery({
  largeFiles,
  duplicateGroups,
  initialTab = 'large-files',
  scanStatus = 'idle',
  scannedAt,
  onScan,
  onCancel,
  onRevealInExplorer,
  onAddExclusion,
}: FileDiscoveryProps): JSX.Element {
  const [activeTab, setActiveTab] = useState<FileDiscoveryTab>(initialTab);
  const [query, setQuery] = useState('');
  const [minimumSize, setMinimumSize] = useState(0);
  const [sensitivity, setSensitivity] = useState<SensitivityFilter>('all');
  const [fileType, setFileType] = useState('all');

  const fileTypes = useMemo(
    () => Array.from(new Set(largeFiles.map((entry) => entry.type).filter(Boolean))).sort((a, b) => a.localeCompare(b)),
    [largeFiles],
  );
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const filteredLargeFiles = useMemo(
    () => largeFiles
      .filter((entry) => entry.sizeBytes >= minimumSize)
      .filter((entry) => sensitivity === 'all' || entry.sensitivity === sensitivity)
      .filter((entry) => fileType === 'all' || entry.type === fileType)
      .filter((entry) => includesQuery([entry.name, entry.path, entry.type], normalizedQuery))
      .slice()
      .sort((a, b) => b.sizeBytes - a.sizeBytes),
    [fileType, largeFiles, minimumSize, normalizedQuery, sensitivity],
  );
  const filteredDuplicateGroups = useMemo(
    () => duplicateGroups
      .filter((group) => group.reclaimableBytes >= minimumSize)
      .filter((group) => includesQuery(
        [group.hash, ...group.members.flatMap((member) => [member.name, member.path])],
        normalizedQuery,
      ))
      .slice()
      .sort((a, b) => b.reclaimableBytes - a.reclaimableBytes),
    [duplicateGroups, minimumSize, normalizedQuery],
  );
  const largeFileBytes = filteredLargeFiles.reduce((sum, entry) => sum + entry.sizeBytes, 0);
  const reclaimableBytes = filteredDuplicateGroups.reduce((sum, group) => sum + group.reclaimableBytes, 0);
  const scanning = scanStatus === 'scanning';

  const changeTab = (tab: FileDiscoveryTab): void => {
    setActiveTab(tab);
    setMinimumSize(0);
    setSensitivity('all');
    setFileType('all');
  };

  return (
    <section className='file-discovery-page'>
      <header className='page-head discovery-page-head'>
        <div>
          <p className='eyebrow'>精确定位，不自动处置</p>
          <h1>大文件与重复文件</h1>
          <p>
            找出空间占用来源，再由你决定如何处理。
            {scannedAt && <span className='scan-time'>最近分析：{scannedAt}</span>}
          </p>
        </div>
        <button
          className='button primary'
          type='button'
          disabled={scanning ? !onCancel : !onScan}
          onClick={() => scanning ? onCancel?.() : onScan?.(activeTab)}
        >
          {scanning ? <CircleStop size={18} /> : <Search size={18} />}
          {scanning ? '取消分析' : '重新分析'}
        </button>
      </header>

      <div className='notice discovery-safety-notice'>
        <ShieldCheck />
        <div>
          <strong>安全模式：结果只读</strong>
          <span>大文件和重复文件均不会默认选中，也不会在此页面直接删除。系统目录、用户资料和受保护文件需格外谨慎。</span>
        </div>
      </div>

      <div className='discovery-tabs' role='tablist' aria-label='文件查找类型'>
        <button
          type='button'
          role='tab'
          aria-selected={activeTab === 'large-files'}
          className={activeTab === 'large-files' ? 'active' : ''}
          onClick={() => changeTab('large-files')}
        >
          <HardDrive size={18} /> 大文件 <span>{largeFiles.length}</span>
        </button>
        <button
          type='button'
          role='tab'
          aria-selected={activeTab === 'duplicates'}
          className={activeTab === 'duplicates' ? 'active' : ''}
          onClick={() => changeTab('duplicates')}
        >
          <Files size={18} /> 重复文件 <span>{duplicateGroups.length}</span>
        </button>
      </div>

      <div className='discovery-toolbar'>
        <label className='searchbox discovery-search'>
          <Search />
          <input
            type='search'
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={activeTab === 'large-files' ? '搜索文件名、路径或类型' : '搜索文件名、路径或完整哈希'}
            aria-label='搜索分析结果'
          />
        </label>
        <label className='filter-field'>
          <span>{activeTab === 'large-files' ? '文件大小' : '可释放空间'}</span>
          <select value={minimumSize} onChange={(event) => setMinimumSize(Number(event.target.value))}>
            {(activeTab === 'large-files' ? sizeFilters : duplicateSizeFilters).map((option) => (
              <option key={option.value} value={option.value}>{option.label}</option>
            ))}
          </select>
        </label>
        {activeTab === 'large-files' && (
          <>
            <label className='filter-field'>
              <span>文件类型</span>
              <select value={fileType} onChange={(event) => setFileType(event.target.value)}>
                <option value='all'>全部类型</option>
                {fileTypes.map((type) => <option value={type} key={type}>{type}</option>)}
              </select>
            </label>
            <label className='filter-field'>
              <span>安全提示</span>
              <select
                value={sensitivity}
                onChange={(event) => setSensitivity(event.target.value as SensitivityFilter)}
              >
                <option value='all'>全部级别</option>
                <option value='normal'>普通文件</option>
                <option value='attention'>需要留意</option>
                <option value='protected'>受保护</option>
              </select>
            </label>
          </>
        )}
        <span className='toolbar-help' title='筛选只改变当前显示结果'>
          <Info size={16} /> 筛选不会修改文件
        </span>
      </div>

      <div role='tabpanel' className='discovery-panel'>
        {activeTab === 'large-files' ? (
          <LargeFilesPanel
            entries={filteredLargeFiles}
            totalBytes={largeFileBytes}
            onRevealInExplorer={onRevealInExplorer}
            onAddExclusion={onAddExclusion}
          />
        ) : (
          <DuplicateGroupsPanel
            groups={filteredDuplicateGroups}
            reclaimableBytes={reclaimableBytes}
            onRevealInExplorer={onRevealInExplorer}
            onAddExclusion={onAddExclusion}
          />
        )}
      </div>
    </section>
  );
}
