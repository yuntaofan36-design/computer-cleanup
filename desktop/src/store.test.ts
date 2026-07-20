import { beforeEach, describe, expect, it } from 'vitest';
import { useAppStore } from './store';
import type { CleanupItem } from './types';

function item(id: string, overrides: Partial<CleanupItem> = {}): CleanupItem {
  return {
    id,
    scope: 'system',
    category: '测试',
    product: 'Windows',
    name: id,
    path: `C:\\safe-cache\\${id}`,
    description: '测试缓存',
    reason: '测试规则',
    sizeBytes: 1024,
    fileCount: 1,
    risk: 'low',
    confidence: 'high',
    impact: 'rebuild',
    recoverability: 'rebuildable',
    deleteMode: 'permanent',
    selectable: true,
    ...overrides,
  };
}

describe('safe cleanup selection', () => {
  beforeEach(() => {
    useAppStore.setState({ cleanupItems: [], selected: new Set() });
  });

  it('only selects low-risk, high-confidence, rebuildable items by default', () => {
    const candidates = [
      item('safe'),
      item('medium-risk', { risk: 'medium' }),
      item('low-confidence', { confidence: 'low' }),
      item('irreversible', { recoverability: 'irreversible' }),
      item('protected', { selectable: false, recoverability: 'protected' }),
    ];

    useAppStore.getState().setSafeDefaults(candidates);

    expect([...useAppStore.getState().selected]).toEqual(['safe']);
  });

  it('refuses to toggle a protected result into the cleanup basket', () => {
    const protectedItem = item('protected', { selectable: false, recoverability: 'protected' });
    useAppStore.setState({ cleanupItems: [protectedItem] });

    useAppStore.getState().toggleItem(protectedItem.id);

    expect(useAppStore.getState().selected.size).toBe(0);
  });
});
