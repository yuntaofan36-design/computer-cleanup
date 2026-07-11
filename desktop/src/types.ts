export type Page = 'overview' | 'cleanup' | 'storage' | 'apps' | 'startup' | 'history' | 'settings';
export type RiskLevel = 'low' | 'medium' | 'high';
export type DeleteMode = 'permanent' | 'recycle_bin';

export interface DiskInfo { id: string; name: string; mount: string; totalBytes: number; freeBytes: number; }
export interface CleanupItem { id: string; category: string; name: string; path: string; description: string; sizeBytes: number; risk: RiskLevel; deleteMode: DeleteMode; selected?: boolean; }
export interface AppEntry { id: string; name: string; publisher: string; version: string; sizeBytes: number; installedAt: string; }
export interface StartupEntry { id: string; name: string; publisher: string; command: string; enabled: boolean; impact: '低' | '中' | '高'; scope: string; }
export interface OperationRecord { id: string; kind: string; title: string; createdAt: string; reclaimedBytes: number; status: 'success' | 'partial' | 'failed'; detail: string; }
export interface ScanProgress { taskId: string; phase: string; currentPath: string; completed: number; total: number; foundBytes: number; }
