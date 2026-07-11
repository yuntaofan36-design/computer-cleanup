import { describe, expect, it } from 'vitest';
import { formatBytes, percent } from './format';

describe('formatBytes', () => {
  it('formats binary units for the interface', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(5 * 1024 ** 3)).toBe('5.0 GB');
  });
  it('clamps percentage values', () => {
    expect(percent(25, 100)).toBe(25);
    expect(percent(200, 100)).toBe(100);
    expect(percent(1, 0)).toBe(0);
  });
});
