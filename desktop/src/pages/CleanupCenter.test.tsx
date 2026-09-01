// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CleanupItem } from '../types';
import CleanupCenter from './CleanupCenter';

afterEach(cleanup);

const wechatItem: CleanupItem = {
  id: 'wechat-local-wechat-cache',
  scope: 'wechat',
  category: '微信运行缓存',
  product: '微信',
  name: '网络缓存',
  path: '%LOCALAPPDATA%\\Tencent\\WeChat\\Cache',
  description: '可重新下载的网络资源',
  reason: '仅命中明确缓存叶子目录',
  sizeBytes: 1024,
  fileCount: 2,
  risk: 'low',
  confidence: 'high',
  impact: 'rebuild',
  recoverability: 'rebuildable',
  deleteMode: 'permanent',
  selectable: true,
};

const chatItem: CleanupItem = {
  ...wechatItem,
  id: 'wechat-user-demo-chat-records',
  category: '微信聊天记录',
  name: '聊天记录',
  path: '%USERPROFILE%\\Documents\\WeChat Files\\wxid_demo\\Msg',
  description: '本地聊天数据库与索引',
  reason: '用户主动决定是否清理',
  sizeBytes: 4096,
  risk: 'high',
  impact: 'user_data',
  recoverability: 'irreversible',
};

const runningBrowserItem: CleanupItem = {
  ...wechatItem,
  id: 'browser-vivaldi-default-cache',
  scope: 'browser',
  category: '浏览器缓存',
  product: 'Vivaldi',
  name: 'Vivaldi · Default · Cache',
  path: '%LOCALAPPDATA%\\Vivaldi\\User Data\\Default\\Cache',
  description: '可由浏览器自动重新生成',
  blockedReason: '检测到 Vivaldi 正在运行；为避免误清理正在使用的数据，本次已安全跳过',
  reason: '检测到 Vivaldi 正在运行；为避免误清理正在使用的数据，本次已安全跳过',
  selectable: false,
};

const quarantinedTempItem: CleanupItem = {
  ...wechatItem,
  id: 'temp',
  scope: 'system',
  category: '系统临时文件',
  product: 'Windows',
  name: '用户临时文件',
  path: '%LOCALAPPDATA%\\Temp',
  description: '超过最小年龄的临时文件',
  reason: '命中 temp 低风险实验规则',
  recoverability: 'recoverable',
  deleteMode: 'quarantine',
};

describe('WeChat cleanup scope', () => {
  it('shows optional user data unselected while keeping safe WeChat cache selected', () => {
    const onToggle = vi.fn();
    const { container } = render(
      <CleanupCenter
        items={[wechatItem, chatItem]}
        selected={new Set([wechatItem.id])}
        scanning={false}
        progress={0}
        scanPath=""
        onScan={vi.fn()}
        onToggle={onToggle}
        onOpenBasket={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('tab', { name: /微信专清/ }));

    expect(screen.getByText('聊天与媒体数据由你决定是否清理')).toBeTruthy();
    expect(screen.getByText(/默认不勾选/)).toBeTruthy();
    expect(screen.getAllByText('网络缓存')).toHaveLength(2);
    expect(screen.getByText('聊天记录')).toBeTruthy();
    expect(container.querySelector('.item-symbol.wechat svg')).toBeTruthy();

    const checkboxes = screen.getAllByRole('checkbox');
    expect(checkboxes[0].getAttribute('aria-checked')).toBe('true');
    expect(checkboxes[1].getAttribute('aria-checked')).toBe('false');
    fireEvent.click(checkboxes[1]);
    expect(onToggle).toHaveBeenCalledWith(chatItem.id);
    fireEvent.click(checkboxes[0]);
    expect(onToggle).toHaveBeenCalledWith(wechatItem.id);
  });
});

describe('browser cleanup scope', () => {
  it('shows running browsers in scan results while keeping them unavailable', () => {
    const onToggle = vi.fn();
    render(
      <CleanupCenter
        items={[runningBrowserItem]}
        selected={new Set()}
        scanning={false}
        progress={0}
        scanPath=""
        onScan={vi.fn()}
        onToggle={onToggle}
        onOpenBasket={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('tab', { name: /浏览器数据/ }));

    expect(screen.getByText('使用中')).toBeTruthy();
    expect(screen.getByText('浏览器正在使用')).toBeTruthy();
    const checkbox = screen.getByRole('checkbox');
    expect(checkbox.getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(checkbox);
    expect(onToggle).not.toHaveBeenCalled();
  });
});

describe('developer cache cleanup scope', () => {
  const developerItem: CleanupItem = {
    ...wechatItem,
    id: 'maven-repository',
    scope: 'devtools',
    category: '开发者缓存',
    product: 'Maven',
    name: 'Maven · 本地仓库',
    path: '%USERPROFILE%\\.m2\\repository',
    description: '构建时可重新下载的依赖',
    reason: '命中可再生的依赖缓存目录',
    sizeBytes: 1_500_000_000,
    fileCount: 18309,
  };

  it('gives developer caches their own tab instead of listing them as Windows items', () => {
    render(
      <CleanupCenter
        items={[developerItem]}
        selected={new Set()}
        scanning={false}
        progress={0}
        scanPath=""
        onScan={vi.fn()}
        onToggle={vi.fn()}
        onOpenBasket={vi.fn()}
      />,
    );

    const tab = screen.getByRole('tab', { name: /开发者缓存/ });
    fireEvent.click(tab);

    // The tool name must be shown, not the generic Windows product label.
    expect(screen.getByText('Maven')).toBeTruthy();
    expect(screen.queryByText('Windows')).toBeNull();
    // The rebuild cost has to be stated before the user cleans a build cache.
    expect(screen.getByText(/首次构建会重新下载依赖/)).toBeTruthy();
    // pnpm's store must be called out as excluded, since deleting it breaks projects.
    expect(screen.getByText(/pnpm store prune/)).toBeTruthy();
  });

  it('keeps every scope tab reachable once developer and QQ scopes exist', () => {
    render(
      <CleanupCenter
        items={[developerItem, wechatItem, runningBrowserItem, quarantinedTempItem]}
        selected={new Set()}
        scanning={false}
        progress={0}
        scanPath=""
        onScan={vi.fn()}
        onToggle={vi.fn()}
        onOpenBasket={vi.fn()}
      />,
    );

    for (const label of [/全部项目/, /系统盘清理/, /软件缓存/, /浏览器数据/, /开发者缓存/, /微信专清/, /QQ 专清/]) {
      expect(screen.getByRole('tab', { name: label })).toBeTruthy();
    }
  });
});

describe('cleanup plan controls', () => {
  it('selects every recommended item in the current group', () => {
    const secondSafeItem = { ...wechatItem, id: 'wechat-local-wechat-code-cache', name: '代码缓存' };
    const onToggle = vi.fn();
    render(
      <CleanupCenter
        items={[wechatItem, secondSafeItem, chatItem]}
        selected={new Set()}
        scanning={false}
        progress={0}
        scanPath=""
        onScan={vi.fn()}
        onToggle={onToggle}
        onOpenBasket={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '全选本组' }));

    expect(onToggle).toHaveBeenCalledTimes(2);
    expect(onToggle).toHaveBeenNthCalledWith(1, wechatItem.id);
    expect(onToggle).toHaveBeenNthCalledWith(2, secondSafeItem.id);
  });

  it('opens final review from the one-click cleanup action', () => {
    const onClean = vi.fn();
    render(
      <CleanupCenter
        items={[wechatItem]}
        selected={new Set([wechatItem.id])}
        scanning={false}
        progress={0}
        scanPath=""
        onScan={vi.fn()}
        onToggle={vi.fn()}
        onOpenBasket={vi.fn()}
        onClean={onClean}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: /一键安全清理/ }));

    expect(onClean).toHaveBeenCalledOnce();
  });

  it('describes quarantine as exportable storage instead of permanent deletion', () => {
    render(
      <CleanupCenter
        items={[quarantinedTempItem]}
        selected={new Set([quarantinedTempItem.id])}
        scanning={false}
        progress={0}
        scanPath=""
        onScan={vi.fn()}
        onToggle={vi.fn()}
        onOpenBasket={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', {
      name: '查看 用户临时文件 的清理依据',
    }));

    expect(screen.getByText('可导出副本')).toBeTruthy();
    expect(screen.getByText(/移入本机隔离仓库/)).toBeTruthy();
    expect(screen.queryByText('永久删除，必须在最终弹窗再次确认')).toBeNull();
  });
});
