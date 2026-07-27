export type CleanupScope = 'system' | 'browser' | 'apps' | 'wechat';
export type RiskLevel = 'low' | 'medium' | 'high';
export type Confidence = 'high' | 'medium' | 'low';
export type Impact = 'none' | 'rebuild' | 'signout' | 'user_data';
export type Recoverability = 'rebuildable' | 'recoverable' | 'irreversible' | 'protected';
export type DeleteMode = 'permanent' | 'recycle_bin' | 'quarantine';

export interface CleanupItem {
  id: string;
  scope: CleanupScope;
  category: string;
  product: string;
  name: string;
  path: string;
  description: string;
  blockedReason?: string;
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

export interface NativeCleanupItem {
  id: string;
  category: string;
  name: string;
  path: string;
  description: string;
  blockedReason?: string;
  sizeBytes: number;
  fileCount: number;
  risk: RiskLevel;
  deleteMode: DeleteMode;
}

export interface CleanupScan {
  scanId: string;
  ruleVersion: string;
  expiresAtMs: number;
  items: CleanupItem[];
}

export interface NativeCleanupScan extends Omit<CleanupScan, 'items'> {
  items: NativeCleanupItem[];
}

export interface CleanupPlan {
  planId: string;
  scanId: string;
  ruleVersion: string;
  createdAtMs: number;
  expiresAtMs: number;
  items: CleanupItem[];
  totalItems: number;
  totalFiles: number;
  totalBytes: number;
  irreversibleItemIds: string[];
}

export interface NativeCleanupPlan extends Omit<CleanupPlan, 'items'> {
  items: NativeCleanupItem[];
}

export interface ExecuteResult {
  reclaimedBytes: number;
  stagedBytes: number;
  succeeded: number;
  failed: Array<{ id: string; error: string; path?: string }>;
}

export interface CleanupProgress {
  phase: 'starting' | 'running' | 'item_complete' | 'complete';
  completedItems: number;
  totalItems: number;
  completedFiles: number;
  totalFiles: number;
  currentItemId: string;
  currentItemName: string;
  currentPath: string;
  reclaimedBytes: number;
  failedFiles: number;
}

export type CleanupProgressHandler = (progress: CleanupProgress) => void;
