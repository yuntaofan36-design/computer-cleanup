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

    fireEvent.click(screen.getByRole('button', { name: /微信专清/ }));

    expect(screen.getByText('聊天与媒体数据由你决定是否清理')).toBeTruthy();
    expect(screen.getByText(/会分类展示但默认不勾选/)).toBeTruthy();
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

    fireEvent.click(screen.getByRole('button', { name: /浏览器数据/ }));

    expect(screen.getByText('使用中')).toBeTruthy();
    expect(screen.getByText('浏览器正在使用')).toBeTruthy();
    const checkbox = screen.getByRole('checkbox');
    expect(checkbox.getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(checkbox);
    expect(onToggle).not.toHaveBeenCalled();
  });
});
