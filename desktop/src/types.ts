export type Page = 'overview' | 'cleanup' | 'files' | 'analysis' | 'apps' | 'recovery' | 'settings';
export type CleanupScope = 'system' | 'browser' | 'apps';
export type RiskLevel = 'low' | 'medium' | 'high';
export type Confidence = 'high' | 'medium' | 'low';
export type Impact = 'none' | 'rebuild' | 'signout' | 'user_data';
export type Recoverability = 'rebuildable' | 'recoverable' | 'irreversible' | 'protected';
export type DeleteMode = 'permanent' | 'recycle_bin';

export interface DiskInfo {
  id: string;
  name: string;
  mount: string;
  totalBytes: number;
  freeBytes: number;
}

export interface CleanupItem {
  id: string;
  scope: CleanupScope;
  category: string;
  product: string;
  name: string;
  path: string;
  description: string;
  reason: string;
  sizeBytes: number;
  fileCount: number;
  risk: RiskLevel;
  confidence: Confidence;
  impact: Impact;
  recoverability: Recoverability;
  deleteMode: DeleteMode;
  selectable: boolean;
  selected?: boolean;
}

export interface AppEntry {
  id: string;
  name: string;
  publisher: string;
  version: string;
  sizeBytes: number;
  cacheBytes: number;
  installedAt: string;
  lastUsed: string;
  uninstallable?: boolean;
}

export interface StartupEntry {
  id: string;
  name: string;
  publisher: string;
  command: string;
  enabled: boolean;
  impact: '低' | '中' | '高' | '未知';
  scope: string;
}

export interface OperationRecord {
  id: string;
  kind: 'cleanup' | 'restore' | 'uninstall';
  title: string;
  createdAt: string;
  reclaimedBytes: number;
  stagedBytes: number;
  status: 'success' | 'partial' | 'failed';
  detail: string;
}

export interface ScanProgress {
  taskId: string;
  phase: string;
  currentPath: string;
  completed: number;
  total: number;
  foundBytes: number;
}

export interface LargeFileEntry {
  id: string;
  name: string;
  path: string;
  sizeBytes: number;
  allocatedBytes: number;
  modifiedAt: string;
  type: string;
  sensitivity: 'normal' | 'attention' | 'protected';
  note?: string;
}

export interface DuplicateMember {
  id: string;
  name: string;
  path: string;
  modifiedAt: string;
  suggestedKeep: boolean;
  protected?: boolean;
}

export interface DuplicateGroup {
  id: string;
  hash: string;
  sizeBytes: number;
  reclaimableBytes: number;
  match: 'full_hash';
  members: DuplicateMember[];
}

export interface DirectoryUsage {
  id: string;
  name: string;
  path: string;
  sizeBytes: number;
  percent: number;
  color: string;
  kind: string;
  fileCount: number;
}

export interface StorageCategory {
  id: string;
  label: string;
  sizeBytes: number;
  color: string;
  description: string;
}

export interface ExecuteResult {
  reclaimedBytes: number;
  succeeded: number;
  failed: Array<{ id: string; error: string }>;
}

export interface ScanStats {
  scannedFiles: number;
  skipped: number;
  cancelled: boolean;
  limitReached: boolean;
}

export interface LargeFileScanResult extends ScanStats {
  files: LargeFileEntry[];
}

export interface DuplicateScanResult extends ScanStats {
  groups: DuplicateGroup[];
}

export interface StorageAnalysisResult extends ScanStats {
  directories: DirectoryUsage[];
  categories: StorageCategory[];
}

export interface UninstallLaunchResult {
  appId: string;
  pid: number;
  status: string;
}
