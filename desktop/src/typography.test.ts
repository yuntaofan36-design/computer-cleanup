import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const styles = readFileSync(new URL('./styles.css', import.meta.url), 'utf8');

describe('typography scale', () => {
  it('keeps the smallest visible text token at 12px', () => {
    expect(styles).toContain('--font-caption: 12px;');
    expect(styles).toContain('--font-small: 13px;');
    expect(styles).toContain('--font-body: 14px;');
  });

  it('does not reintroduce explicit text below 12px', () => {
    const fontSizes = [...styles.matchAll(/font-size:\s*(\d+)px/g)]
      .map((match) => Number(match[1]));
    const shorthandSizes = [...styles.matchAll(/font:\s*(?:\d+\s+)?(\d+)px/g)]
      .map((match) => Number(match[1]));

    expect([...fontSizes, ...shorthandSizes].filter((size) => size < 12)).toEqual([]);
  });

  it('keeps the installed-app table responsive without an internal horizontal scrollbar', () => {
    expect(styles).toMatch(/\.app-management-table \{[^}]*overflow-x: hidden;/);
    expect(styles).toMatch(/\.table-head, \.table-row \{[^}]*min-width: 0;/);
    expect(styles).toMatch(
      /\.app-management-table \.row-actions \{[^}]*flex-direction: column;/,
    );
  });
});
