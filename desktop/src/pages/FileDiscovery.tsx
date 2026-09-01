import { useEffect, useMemo, useState } from 'react';
import type { CSSProperties } from 'react';
import { createPortal } from 'react-dom';
import {
  AlertTriangle,
  Archive,
  Ban,
  Check,
  CheckCircle2,
  CircleStop,
  ExternalLink,
  File,
  FileImage,
  FileSearch,
  FileText,
  Files,
  Film,
  FolderX,
  HardDrive,
  Info,
  LoaderCircle,
  Music,
  Search,
  ShieldCheck,
  Trash2,
  X,
} from 'lucide-react';
import { Dialog } from '../components';
import { formatBytes } from '../format';
import SelectMenu from '../SelectMenu';
import type {
  DuplicateGroup,
  LargeFileDeleteProgress,
  LargeFileDeleteResult,
  LargeFileEntry,
} from '../types';

export type FileDiscoveryTab = 'large-files' | 'duplicates';

export interface FileDiscoveryProps {
  largeFiles: readonly LargeFileEntry[];
  duplicateGroups: readonly DuplicateGroup[];
  initialTab?: FileDiscoveryTab;
  scanStatus?: 'idle' | 'scanning' | 'complete';
  scannedAt?: string;
  onScan?: (tab: FileDiscoveryTab) => void;
  onCancel?: () => void;
  onDeleteLargeFiles?: (
    ids: string[],
    onProgress: (progress: LargeFileDeleteProgress) => void,
  ) => Promise<LargeFileDeleteResult>;
  onRevealInExplorer?: (path: string) => void;
  onAddExclusion?: (path: string) => void;
}

const MIB = 1024 ** 2;
const GIB = 1024 ** 3;
const largeFileSizeFilters = [
  { label: '≥100MB', value: 100 * MIB },
  { label: '≥500MB', value: 500 * MIB },
  { label: '≥1GB', value: GIB },
  { label: '≥5GB', value: 5 * GIB },
  { label: '≥10GB', value: 10 * GIB },
] as const;
const duplicateSizeFilters = [
  { label: '全部可释放空间', value: 0 },
  { label: '可释放 500 MB 以上', value: 500 * 1024 ** 2 },
  { label: '可释放 1 GB 以上', value: GIB },
  { label: '可释放 5 GB 以上', value: 5 * GIB },
] as const;
type LargeFileCategory = 'all' | 'video' | 'audio' | 'documents' | 'images' | 'archives' | 'other';
type LargeFileSort = 'size_desc' | 'size_asc' | 'modified_desc';

const largeFileCategories: Array<{ id: LargeFileCategory; label: string }> = [
  { id: 'all', label: '全部' },
  { id: 'video', label: '视频' },
  { id: 'audio', label: '音乐' },
  { id: 'documents', label: '文档' },
  { id: 'images', label: '图片' },
  { id: 'archives', label: '压缩包' },
  { id: 'other', label: '其他' },
];

function largeFileCategory(entry: LargeFileEntry): Exclude<LargeFileCategory, 'all'> {
  if (entry.type === '视频') return 'video';
  if (entry.type === '音频') return 'audio';
  if (entry.type === '图片') return 'images';
  if (['文档', '设计文件', '数据库', '邮件存档'].includes(entry.type)) return 'documents';
  if (
    ['磁盘镜像', '程序或安装包'].includes(entry.type)
    || entry.type.includes('压缩')
    || entry.type.includes('备份')
  ) return 'archives';
  return 'other';
}

function driveLabel(path: string): string {
  const match = path.match(/^([a-z]):[\\/]/i);
  return match ? match[1].toUpperCase() + '盘' : '本地盘';
}

function initialLargeDeleteProgress(items: readonly LargeFileEntry[]): LargeFileDeleteProgress {
  return {
    phase: 'starting',
    completed: 0,
    total: items.length,
    currentItemId: '',
    currentName: '',
    currentPath: '',
    deletedBytes: 0,
    failed: 0,
  };
}

function LargeTypeIcon({ category }: { category: Exclude<LargeFileCategory, 'all'> }): JSX.Element {
  if (category === 'video') return <Film />;
  if (category === 'audio') return <Music />;
  if (category === 'images') return <FileImage />;
  if (category === 'documents') return <FileText />;
  if (category === 'archives') return <Archive />;
  return <File />;
}

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

function LargeFileDeleteDialog({
  plan,
  progress,
  result,
  error,
  running,
  confirmed,
  onConfirmedChange,
  onClose,
  onConfirm,
}: {
  plan: readonly LargeFileEntry[];
  progress: LargeFileDeleteProgress;
  result: LargeFileDeleteResult | null;
  error: string;
  running: boolean;
  confirmed: boolean;
  onConfirmedChange: (confirmed: boolean) => void;
  onClose: () => void;
  onConfirm: () => void;
}): JSX.Element {
  const showExecution = running || Boolean(result) || Boolean(error);
  const completed = result ? progress.total : progress.completed;
  const percent = progress.total ? Math.round(completed / progress.total * 100) : 0;
  const failedIds = new Set(result?.failed.map((failure) => failure.id) || []);
  const succeededIds = new Set(result?.succeededIds || []);
  const totalBytes = plan.reduce((sum, item) => sum + item.sizeBytes, 0);

  return createPortal(
    <Dialog
      title={running ? '正在永久删除所选大文件' : result ? '大文件清理完成' : error ? '大文件清理未完成' : '确认清理所选大文件？'}
      danger
      busy={running}
      confirmDisabled={!confirmed || !plan.length}
      confirmLabel='永久删除所选文件'
      hideActions={showExecution}
      closeDisabled={running}
      wide={showExecution}
      onClose={onClose}
      onConfirm={onConfirm}
    >
      {showExecution ? (
        <div className={'large-delete-execution ' + (result ? 'finished' : error ? 'failed' : 'running')} role='status' aria-live='polite'>
          <div className='large-delete-progress-head'>
            <div className='large-delete-orbit' style={{ '--large-delete-progress': percent + '%' } as CSSProperties}>
              {result ? <CheckCircle2 /> : error ? <AlertTriangle /> : <LoaderCircle />}
              <strong>{percent}%</strong>
            </div>
            <div>
              <p className='eyebrow'>{result ? '执行结果' : error ? '执行中断' : '逐文件安全复检'}</p>
              <h3>{result ? '所选文件已处理完成' : error || progress.currentName || '正在准备删除计划'}</h3>
              <p title={progress.currentPath}>{result ? '每个文件都已按最近一次扫描快照重新确认' : progress.currentPath || '正在验证文件身份与路径'}</p>
              <div className='large-delete-track'><span style={{ width: percent + '%' }} /></div>
              <small>{progress.completed} / {progress.total} 个文件已处理</small>
            </div>
          </div>
          <div className='large-delete-metrics'>
            <span><small>永久删除</small><strong>{formatBytes(result?.deletedBytes ?? progress.deletedBytes)}</strong></span>
            <span><small>已完成</small><strong>{result?.succeededIds.length ?? progress.completed - progress.failed} 个</strong></span>
            <span className={(result?.failed.length ?? progress.failed) ? 'attention' : ''}><small>安全保留</small><strong>{result?.failed.length ?? progress.failed} 个</strong></span>
          </div>
          <div className='large-delete-list'>
            {plan.map((item, index) => {
              const failed = failedIds.has(item.id);
              const succeeded = succeededIds.has(item.id);
              const active = running && progress.currentItemId === item.id;
              const processed = !result && index < progress.completed;
              const status = failed ? '安全保留' : succeeded || processed ? '已删除' : active ? '删除中' : '等待中';
              return (
                <div className={'large-delete-item ' + (failed ? 'retained' : succeeded || processed ? 'deleted' : active ? 'active' : '')} key={item.id}>
                  {failed ? <AlertTriangle /> : succeeded || processed ? <CheckCircle2 /> : active ? <LoaderCircle /> : <File />}
                  <span><strong>{item.name}</strong><small title={item.path}>{item.path}</small></span>
                  <b>{formatBytes(item.sizeBytes)}</b>
                  <em>{status}</em>
                </div>
              );
            })}
          </div>
          {(result || error) && (
            <div className='large-delete-finish'>
              <span>{error ? <AlertTriangle /> : <ShieldCheck />}{error || (result?.failed.length ? '发生变化、被占用或无法复检的文件已安全保留' : '所选大文件已永久删除')}</span>
              <button className='button primary' type='button' onClick={onClose}><Check />完成</button>
            </div>
          )}
        </div>
      ) : (
        <div className='large-delete-confirm'>
          <p>将永久删除 <strong>{plan.length} 个文件</strong>，文件大小合计 <strong>{formatBytes(totalBytes)}</strong>。</p>
          <div className='confirm-warning'>
            <AlertTriangle />
            <span><strong>此操作不会进入回收站</strong><small>执行前会重新核对路径、大小、修改时间和文件身份；任何变化都会跳过并保留。</small></span>
          </div>
          <div className='large-delete-review-list'>
            {plan.map((item) => <div key={item.id}><span><strong>{item.name}</strong><small>{item.path}</small></span><b>{formatBytes(item.sizeBytes)}</b></div>)}
          </div>
          <label className='irreversible-confirmation'>
            <input type='checkbox' checked={confirmed} onChange={(event) => onConfirmedChange(event.target.checked)} />
            <span>我确认永久删除以上已选择的大文件</span>
          </label>
        </div>
      )}
    </Dialog>,
    document.body,
  );
}

function LargeFilesExperience({
  entries,
  scanStatus,
  scannedAt,
  onScan,
  onCancel,
  onOpenDuplicates,
  onDeleteLargeFiles,
  onRevealInExplorer,
  onAddExclusion,
}: {
  entries: readonly LargeFileEntry[];
  scanStatus: 'idle' | 'scanning' | 'complete';
  scannedAt?: string;
  onScan?: () => void;
  onCancel?: () => void;
  onOpenDuplicates: () => void;
  onDeleteLargeFiles?: FileDiscoveryProps['onDeleteLargeFiles'];
  onRevealInExplorer?: (path: string) => void;
  onAddExclusion?: (path: string) => void;
}): JSX.Element {
  const [category, setCategory] = useState<LargeFileCategory>('all');
  const [query, setQuery] = useState('');
  const [drive, setDrive] = useState('all');
  const [minimumSize, setMinimumSize] = useState<number>(largeFileSizeFilters[0].value);
  const [sort, setSort] = useState<LargeFileSort>('size_desc');
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [cleanupOpen, setCleanupOpen] = useState(false);
  const [cleanupPlan, setCleanupPlan] = useState<LargeFileEntry[]>([]);
  const [deleteProgress, setDeleteProgress] = useState<LargeFileDeleteProgress>(initialLargeDeleteProgress([]));
  const [deleteResult, setDeleteResult] = useState<LargeFileDeleteResult | null>(null);
  const [deleteError, setDeleteError] = useState('');
  const [deleting, setDeleting] = useState(false);
  const [permanentConfirmed, setPermanentConfirmed] = useState(false);

  useEffect(() => {
    const currentIds = new Set(entries.map((entry) => entry.id));
    setSelected((current) => new Set([...current].filter((id) => currentIds.has(id))));
  }, [entries]);

  const totalBytes = entries.reduce((sum, entry) => sum + entry.sizeBytes, 0);
  const categoryStats = useMemo(() => {
    const stats = new Map<LargeFileCategory, { count: number; bytes: number }>();
    largeFileCategories.forEach((item) => stats.set(item.id, { count: 0, bytes: 0 }));
    for (const entry of entries) {
      const itemCategory = largeFileCategory(entry);
      const all = stats.get('all')!;
      const scoped = stats.get(itemCategory)!;
      all.count += 1;
      all.bytes += entry.sizeBytes;
      scoped.count += 1;
      scoped.bytes += entry.sizeBytes;
    }
    return stats;
  }, [entries]);
  const drives = useMemo(() => Array.from(new Set(entries.map((entry) => driveLabel(entry.path))).values()).sort(), [entries]);
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const filtered = useMemo(() => entries
    .filter((entry) => category === 'all' || largeFileCategory(entry) === category)
    .filter((entry) => drive === 'all' || driveLabel(entry.path) === drive)
    .filter((entry) => entry.sizeBytes >= minimumSize)
    .filter((entry) => includesQuery([entry.name, entry.path, entry.type], normalizedQuery))
    .slice()
    .sort((left, right) => {
      if (sort === 'size_asc') return left.sizeBytes - right.sizeBytes;
      if (sort === 'modified_desc') return right.modifiedAt.localeCompare(left.modifiedAt);
      return right.sizeBytes - left.sizeBytes;
    }), [category, drive, entries, minimumSize, normalizedQuery, sort]);
  const selectableFiltered = filtered.filter((entry) => entry.sensitivity !== 'protected');
  const allFilteredSelected = selectableFiltered.length > 0 && selectableFiltered.every((entry) => selected.has(entry.id));
  const selectedFiles = useMemo(() => entries.filter((entry) => selected.has(entry.id)), [entries, selected]);
  const selectedBytes = selectedFiles.reduce((sum, entry) => sum + entry.sizeBytes, 0);
  const scanning = scanStatus === 'scanning';

  const toggleFile = (entry: LargeFileEntry): void => {
    if (entry.sensitivity === 'protected') return;
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(entry.id)) next.delete(entry.id);
      else next.add(entry.id);
      return next;
    });
  };

  const toggleVisible = (): void => {
    setSelected((current) => {
      const next = new Set(current);
      selectableFiltered.forEach((entry) => {
        if (allFilteredSelected) next.delete(entry.id);
        else next.add(entry.id);
      });
      return next;
    });
  };

  const openCleanup = (): void => {
    if (!selectedFiles.length) return;
    const plan = [...selectedFiles];
    setCleanupPlan(plan);
    setDeleteProgress(initialLargeDeleteProgress(plan));
    setDeleteResult(null);
    setDeleteError('');
    setPermanentConfirmed(false);
    setCleanupOpen(true);
  };

  const closeCleanup = (): void => {
    if (deleting) return;
    setCleanupOpen(false);
    setCleanupPlan([]);
    setDeleteResult(null);
    setDeleteError('');
    setPermanentConfirmed(false);
  };

  const runDelete = async (): Promise<void> => {
    if (!permanentConfirmed || !cleanupPlan.length || deleting) return;
    if (!onDeleteLargeFiles) {
      setDeleteError('当前环境未提供大文件删除能力');
      return;
    }
    setDeleting(true);
    setDeleteError('');
    setDeleteResult(null);
    try {
      const result = await onDeleteLargeFiles(cleanupPlan.map((item) => item.id), setDeleteProgress);
      setDeleteResult(result);
      setDeleteProgress((current) => ({
        ...current,
        phase: 'complete',
        completed: cleanupPlan.length,
        total: cleanupPlan.length,
        currentItemId: '',
        currentName: '',
        currentPath: '',
        deletedBytes: result.deletedBytes,
        failed: result.failed.length,
      }));
      const deletedIds = new Set(result.succeededIds);
      setSelected((current) => new Set([...current].filter((id) => !deletedIds.has(id))));
    } catch (error) {
      setDeleteError(error instanceof Error ? error.message : '大文件清理未执行');
    } finally {
      setDeleting(false);
    }
  };

  return (
    <>
      <header className='large-file-hero'>
        <div>
          <p className='eyebrow'>大文件专清理</p>
          <h1>发现 <strong>{entries.length.toLocaleString()}</strong> 个大文件，共 <strong>{formatBytes(totalBytes)}</strong></h1>
          <p>勾选确认不再需要的文件；受保护内容只能查看，不能从此入口删除。</p>
          {scannedAt && <span className='scan-time'>最近分析：{scannedAt}</span>}
        </div>
        <div className='large-file-hero-actions'>
          <div className='file-mode-switch' role='tablist' aria-label='文件发现模式'>
            <button type='button' role='tab' aria-selected='true' className='active'><HardDrive />大文件</button>
            <button type='button' role='tab' aria-selected='false' onClick={onOpenDuplicates}><Files />重复文件</button>
          </div>
          <button className='button large-clean-button' type='button' disabled={!selectedFiles.length || scanning} onClick={openCleanup}>
            <Trash2 /><span><strong>一键清理</strong><small>{selectedFiles.length ? selectedFiles.length + ' 项 · ' + formatBytes(selectedBytes) : '请先选择文件'}</small></span>
          </button>
        </div>
      </header>

      <div className='large-file-category-tabs' role='tablist' aria-label='大文件分类'>
        {largeFileCategories.map((item) => {
          const stats = categoryStats.get(item.id)!;
          return <button type='button' role='tab' aria-selected={category === item.id} className={category === item.id ? 'active' : ''} onClick={() => setCategory(item.id)} key={item.id}><strong>{item.label}</strong><span>{stats.count ? formatBytes(stats.bytes) : '0'}</span></button>;
        })}
      </div>

      <div className='large-file-toolbar'>
        <label className='searchbox large-file-search'>
          <Search />
          <input type='search' value={query} onChange={(event) => setQuery(event.target.value)} placeholder='搜索文件名或路径' aria-label='搜索大文件' />
        </label>
        <SelectMenu className='filter-field' ariaLabel='大文件所在磁盘' label='磁盘' value={drive} options={[{ value: 'all', label: '全部磁盘' }, ...drives.map((item) => ({ value: item, label: item }))]} onChange={setDrive} />
        <SelectMenu className='filter-field' ariaLabel='大文件最小大小' label='大小' value={String(minimumSize)} options={largeFileSizeFilters.map((item) => ({ value: String(item.value), label: item.label }))} onChange={(value) => setMinimumSize(Number(value))} />
        <SelectMenu className='filter-field' ariaLabel='大文件排序方式' label='排序' value={sort} options={[{ value: 'size_desc', label: '文件从大到小' }, { value: 'size_asc', label: '文件从小到大' }, { value: 'modified_desc', label: '最近修改优先' }]} onChange={(value) => setSort(value as LargeFileSort)} />
        <span className='large-file-safety'><ShieldCheck />执行前复检文件身份，变化项自动保留</span>
        <button className='button secondary small' type='button' disabled={scanning ? !onCancel : !onScan} onClick={() => scanning ? onCancel?.() : onScan?.()}>{scanning ? <CircleStop /> : <Search />}{scanning ? '取消分析' : '重新分析'}</button>
      </div>

      {!filtered.length ? (
        <div className='discovery-empty'><FileSearch /><h2>没有符合条件的大文件</h2><p>调整分类、磁盘或搜索条件；受保护和已排除路径不会进入可清理计划。</p></div>
      ) : (
        <div className='large-file-table-wrap'>
          <table className='large-file-table'>
            <thead><tr><th><input type='checkbox' checked={allFilteredSelected} onChange={toggleVisible} aria-label='选择当前结果中的所有可清理文件' /></th><th>文件名称</th><th>磁盘</th><th>文件大小</th><th>修改时间</th><th><span className='visually-hidden'>操作</span></th></tr></thead>
            <tbody>{filtered.map((entry) => {
              const protectedFile = entry.sensitivity === 'protected';
              const selectedFile = selected.has(entry.id);
              const itemCategory = largeFileCategory(entry);
              return (
                <tr className={(selectedFile ? 'selected ' : '') + (protectedFile ? 'protected' : '')} key={entry.id}>
                  <td><input type='checkbox' checked={selectedFile} disabled={protectedFile} onChange={() => toggleFile(entry)} aria-label={protectedFile ? entry.name + ' 受保护，不可选择' : '选择 ' + entry.name} /></td>
                  <td><span className={'large-file-kind ' + itemCategory}><LargeTypeIcon category={itemCategory} /></span><span className='large-file-name'><strong>{entry.name}</strong><small title={entry.path}>{entry.path}</small>{entry.note && <em title={entry.note}>{entry.note}</em>}</span></td>
                  <td><span className='large-file-drive'>{driveLabel(entry.path)}</span></td>
                  <td><strong>{formatBytes(entry.sizeBytes)}</strong></td>
                  <td><time>{entry.modifiedAt}</time></td>
                  <td><span className='large-file-row-actions'><button className='icon-button' type='button' disabled={!onRevealInExplorer} onClick={() => onRevealInExplorer?.(entry.path)} aria-label={'查看 ' + entry.name + ' 的位置'} title='在文件资源管理器中查看'><ExternalLink /></button><button className='icon-button' type='button' disabled={!onAddExclusion} onClick={() => onAddExclusion?.(entry.path)} aria-label={'排除 ' + entry.name} title='后续扫描排除此文件'><FolderX /></button></span></td>
                </tr>
              );
            })}</tbody>
          </table>
        </div>
      )}

      {selectedFiles.length > 0 && <div className='large-file-selection-bar'><span><CheckCircle2 /><strong>已选择 {selectedFiles.length} 项</strong><small>{formatBytes(selectedBytes)} 将永久删除</small></span><div><button className='button secondary' type='button' onClick={() => setSelected(new Set())}><X />清空选择</button><button className='button primary' type='button' onClick={openCleanup}><Trash2 />复检并清理</button></div></div>}

      {cleanupOpen && <LargeFileDeleteDialog plan={cleanupPlan} progress={deleteProgress} result={deleteResult} error={deleteError} running={deleting} confirmed={permanentConfirmed} onConfirmedChange={setPermanentConfirmed} onClose={closeCleanup} onConfirm={() => void runDelete()} />}
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
            Lumina Clean 先比较文件大小，再读取每个文件的全部内容计算哈希；只有完整哈希一致才归为同组。
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
  onDeleteLargeFiles,
  onRevealInExplorer,
  onAddExclusion,
}: FileDiscoveryProps): JSX.Element {
  const [activeTab, setActiveTab] = useState<FileDiscoveryTab>(initialTab);
  const [query, setQuery] = useState('');
  const [minimumSize, setMinimumSize] = useState(0);

  const normalizedQuery = query.trim().toLocaleLowerCase();
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
  const reclaimableBytes = filteredDuplicateGroups.reduce((sum, group) => sum + group.reclaimableBytes, 0);
  const scanning = scanStatus === 'scanning';

  const changeTab = (tab: FileDiscoveryTab): void => {
    setActiveTab(tab);
    setMinimumSize(0);
    setQuery('');
  };

  if (activeTab === 'large-files') {
    return (
      <section className='file-discovery-page large-files-experience'>
        <LargeFilesExperience
          entries={largeFiles}
          scanStatus={scanStatus}
          scannedAt={scannedAt}
          onScan={() => onScan?.('large-files')}
          onCancel={onCancel}
          onOpenDuplicates={() => changeTab('duplicates')}
          onDeleteLargeFiles={onDeleteLargeFiles}
          onRevealInExplorer={onRevealInExplorer}
          onAddExclusion={onAddExclusion}
        />
      </section>
    );
  }

  return (
    <section className='file-discovery-page'>
      <header className='page-head discovery-page-head'>
        <div>
          <p className='eyebrow'>完整内容哈希核验</p>
          <h1>重复文件</h1>
          <p>
            只展示完整内容一致的文件组，不根据文件名或时间猜测。
            {scannedAt && <span className='scan-time'>最近分析：{scannedAt}</span>}
          </p>
        </div>
        <button
          className='button primary'
          type='button'
          disabled={scanning ? !onCancel : !onScan}
          onClick={() => scanning ? onCancel?.() : onScan?.('duplicates')}
        >
          {scanning ? <CircleStop size={18} /> : <Search size={18} />}
          {scanning ? '取消分析' : '重新分析'}
        </button>
      </header>

      <div className='notice discovery-safety-notice'>
        <ShieldCheck />
        <div>
          <strong>安全模式：结果只读</strong>
          <span>重复文件不会默认选中，也不提供直接删除；请先查看位置并确认每个副本的用途。</span>
        </div>
      </div>

      <div className='discovery-tabs' role='tablist' aria-label='文件查找类型'>
        <button
          type='button'
          role='tab'
          aria-selected='false'
          onClick={() => changeTab('large-files')}
        >
          <HardDrive size={18} /> 大文件 <span>{largeFiles.length}</span>
        </button>
        <button
          type='button'
          role='tab'
          aria-selected='true'
          className='active'
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
            placeholder='搜索文件名、路径或完整哈希'
            aria-label='搜索分析结果'
          />
        </label>
        <SelectMenu
          className='filter-field'
          ariaLabel='重复文件可释放空间'
          label='可释放空间'
          value={String(minimumSize)}
          options={duplicateSizeFilters.map((option) => ({ value: String(option.value), label: option.label }))}
          onChange={(value) => setMinimumSize(Number(value))}
        />
        <span className='toolbar-help' title='筛选只改变当前显示结果'>
          <Info size={16} /> 筛选不会修改文件
        </span>
      </div>

      <div role='tabpanel' className='discovery-panel'>
        <DuplicateGroupsPanel
          groups={filteredDuplicateGroups}
          reclaimableBytes={reclaimableBytes}
          onRevealInExplorer={onRevealInExplorer}
          onAddExclusion={onAddExclusion}
        />
      </div>
    </section>
  );
}
