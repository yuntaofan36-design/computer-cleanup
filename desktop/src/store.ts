import { create } from 'zustand';
import type { CleanupItem, DiskInfo, Page } from './types';

interface AppState {
  page: Page;
  theme: 'system' | 'light' | 'dark';
  disks: DiskInfo[];
  activeDiskId: string;
  cleanupItems: CleanupItem[];
  selected: Set<string>;
  scanning: boolean;
  progress: number;
  scanPath: string;
  lastScanAt: string;
  basketOpen: boolean;
  setPage: (page: Page) => void;
  setTheme: (theme: AppState['theme']) => void;
  setDisks: (disks: DiskInfo[]) => void;
  setActiveDiskId: (id: string) => void;
  setCleanupItems: (items: CleanupItem[]) => void;
  setScanning: (value: boolean) => void;
  setProgress: (value: number) => void;
  setScanPath: (path: string) => void;
  setLastScanAt: (value: string) => void;
  setBasketOpen: (open: boolean) => void;
  toggleItem: (id: string) => void;
  setSafeDefaults: (items: CleanupItem[]) => void;
  clearSelection: () => void;
  removeSelected: (ids: string[]) => void;
}

function readInitialTheme(): AppState['theme'] {
  try {
    if (typeof localStorage === 'undefined') return 'system';
    const stored = localStorage.getItem('luminaTheme') ?? localStorage.getItem('qingpanTheme');
    return stored === 'light' || stored === 'dark' || stored === 'system' ? stored : 'system';
  } catch {
    return 'system';
  }
}

function defaultDiskId(disks: DiskInfo[]): string {
  const systemDisk = disks.find((disk) => /^c:[\\/]*$/i.test(disk.mount.trim()));
  return systemDisk?.id ?? disks[0]?.id ?? '';
}

export const useAppStore = create<AppState>((set) => ({
  page: 'overview',
  theme: readInitialTheme(),
  disks: [],
  activeDiskId: '',
  cleanupItems: [],
  selected: new Set(),
  scanning: false,
  progress: 0,
  scanPath: '',
  lastScanAt: '尚未扫描',
  basketOpen: false,
  setPage: (page) => set({ page }),
  setTheme: (theme) => set({ theme }),
  setDisks: (disks) => set((state) => ({
    disks,
    activeDiskId: disks.some((disk) => disk.id === state.activeDiskId)
      ? state.activeDiskId
      : defaultDiskId(disks),
  })),
  setActiveDiskId: (activeDiskId) => set({ activeDiskId }),
  setCleanupItems: (cleanupItems) => set({ cleanupItems }),
  setScanning: (scanning) => set({ scanning }),
  setProgress: (progress) => set({ progress }),
  setScanPath: (scanPath) => set({ scanPath }),
  setLastScanAt: (lastScanAt) => set({ lastScanAt }),
  setBasketOpen: (basketOpen) => set({ basketOpen }),
  toggleItem: (id) => set((state) => {
    const item = state.cleanupItems.find((entry) => entry.id === id);
    if (!item?.selectable) return state;
    const selected = new Set(state.selected);
    selected.has(id) ? selected.delete(id) : selected.add(id);
    return { selected };
  }),
  setSafeDefaults: (items) => set({
    selected: new Set(items
      .filter((item) => item.selectable && item.risk === 'low' && item.confidence === 'high' && item.recoverability === 'rebuildable')
      .map((item) => item.id)),
  }),
  clearSelection: () => set({ selected: new Set() }),
  removeSelected: (ids) => set((state) => {
    const selected = new Set(state.selected);
    ids.forEach((id) => selected.delete(id));
    return { selected };
  }),
}));
