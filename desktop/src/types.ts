export type {
  CleanupItem,
  CleanupProgress,
  CleanupScope,
  Confidence,
  DeleteMode,
  ExecuteResult,
  Impact,
  Recoverability,
  RiskLevel,
} from './features/cleanup-plan';

export type Page = 'overview' | 'cleanup' | 'files' | 'analysis' | 'partition' | 'apps' | 'recovery' | 'settings';

export interface DiskInfo {
  id: string;
  name: string;
  mount: string;
  totalBytes: number;
  freeBytes: number;
}

export interface DiskPartition {
  partitionNumber: number;
  driveLetter: string | null;
  offsetBytes: number;
  sizeBytes: number;
  partitionType: string;
  gptType: string;
  isSystem: boolean;
  isBoot: boolean;
  isActive: boolean;
  isHidden: boolean;
  isReadOnly: boolean;
  noDefaultDriveLetter: boolean;
  fileSystem: string;
  label: string;
  healthStatus: string;
  freeBytes: number;
}

export interface PartitionDisk {
  number: number;
  friendlyName: string;
  partitionStyle: string;
  busType: string;
  healthStatus: string;
  operationalStatus: string;
  sizeBytes: number;
  isBoot: boolean;
  isSystem: boolean;
  isOffline: boolean;
  isReadOnly: boolean;
  partitions: DiskPartition[];
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

export interface ScanStats {
  scannedFiles: number;
  skipped: number;
  cancelled: boolean;
  limitReached: boolean;
}

export interface LargeFileScanResult extends ScanStats {
  files: LargeFileEntry[];
}

export interface LargeFileDeleteProgress {
  phase: 'starting' | 'running' | 'item_complete' | 'complete';
  completed: number;
  total: number;
  currentItemId: string;
  currentName: string;
  currentPath: string;
  deletedBytes: number;
  failed: number;
}

export interface LargeFileDeleteResult {
  deletedBytes: number;
  succeededIds: string[];
  failed: Array<{ id: string; error: string; path?: string }>;
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
