export type QuarantineRecordState =
  | 'committed'
  | 'sourceRetained'
  | 'damaged'
  | 'recoveryRequired';

export interface QuarantineRecord {
  recordId: string;
  fileName: string;
  ruleId: string;
  planId: string;
  createdAtMs: number;
  sizeBytes: number;
  state: QuarantineRecordState;
  exportable: boolean;
  sourceRetained: boolean;
}

export interface QuarantineListResponse {
  records: QuarantineRecord[];
  corruptRecords: number;
}

export interface QuarantineExportResult {
  operationId: string;
  recordId: string;
  exportedDirectory: string;
  exportedFileName: string;
  bytes: number;
  quarantineSourceRetained: boolean;
  auditPersisted: boolean;
}

export interface QuarantineApi {
  list: (limit?: number) => Promise<QuarantineListResponse>;
  exportCopy: (recordId: string) => Promise<QuarantineExportResult>;
}

export type QuarantineListStatus = 'idle' | 'loading' | 'ready' | 'refreshing' | 'error';
