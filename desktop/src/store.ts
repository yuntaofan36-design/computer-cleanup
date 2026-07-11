import { create } from 'zustand';
import type { CleanupItem, Page } from './types';

interface AppState {
  page: Page; theme: 'system' | 'light' | 'dark'; scanning: boolean; progress: number; selected: Set<string>;
  setPage: (page: Page) => void; setTheme: (theme: AppState['theme']) => void; setScanning: (value: boolean) => void;
  setProgress: (value: number) => void; toggleItem: (id: string) => void; setSafeDefaults: (items: CleanupItem[]) => void; clearSelection: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  page: 'overview', theme: 'system', scanning: false, progress: 0, selected: new Set(),
  setPage: (page) => set({ page }), setTheme: (theme) => set({ theme }), setScanning: (scanning) => set({ scanning }),
  setProgress: (progress) => set({ progress }),
  toggleItem: (id) => set((s) => { const selected = new Set(s.selected); selected.has(id) ? selected.delete(id) : selected.add(id); return { selected }; }),
  setSafeDefaults: (items) => set({ selected: new Set(items.filter((i) => i.risk === 'low').map((i) => i.id)) }),
  clearSelection: () => set({ selected: new Set() }),
}));
