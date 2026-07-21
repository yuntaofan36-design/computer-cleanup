import { useState } from 'react';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import {
  AppWindow,
  Clock3,
  Download,
  FolderLock,
  ListFilter,
  LockKeyhole,
  Moon,
  RefreshCw,
  ShieldCheck,
  Sun,
} from 'lucide-react';
import { isNativeRuntime } from '../native';

export type ThemeMode = 'system' | 'light' | 'dark';

export interface SettingsPageProps {
  protectedDirectories: string[];
  exclusionRules: string[];
  autoCleanupEnabled?: boolean;
  onAutoCleanupChange?: (enabled: boolean) => void;
  theme: ThemeMode;
  setTheme: (theme: ThemeMode) => void;
}

type UpdateState = 'idle' | 'checking' | 'available' | 'latest' | 'downloading' | 'error';

const themeOptions: Array<{ value: ThemeMode; label: string; icon: typeof AppWindow }> = [
  { value: 'system', label: '跟随系统', icon: AppWindow },
  { value: 'light', label: '浅色', icon: Sun },
  { value: 'dark', label: '深色', icon: Moon },
];

function PathList({ items, emptyText }: { items: string[]; emptyText: string }): JSX.Element {
  if (items.length === 0) return <p className="setting-empty">{emptyText}</p>;
  return <ul className="path-list">{items.map((item) => <li key={item}><code>{item}</code></li>)}</ul>;
}

export default function SettingsPage({
  protectedDirectories,
  exclusionRules,
  autoCleanupEnabled = false,
  onAutoCleanupChange,
  theme,
  setTheme,
}: SettingsPageProps): JSX.Element {
  const [updateState, setUpdateState] = useState<UpdateState>('idle');
  const [update, setUpdate] = useState<Update | null>(null);
  const [updateMessage, setUpdateMessage] = useState('仅在 Windows 桌面版中检查签名更新。');
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState<number | null>(null);
  const desktopRuntime = isNativeRuntime();

  async function checkForUpdate() {
    if (!desktopRuntime) {
      setUpdateState('error');
      setUpdateMessage('浏览器预览不支持更新；请在安装后的清盘桌面版中操作。');
      return;
    }
    setUpdateState('checking');
    setUpdateMessage('正在校验发布清单和更新签名…');
    try {
      const available = await check();
      setUpdate(available);
      if (available) {
        setUpdateState('available');
        setUpdateMessage(`发现清盘 ${available.version}，下载后将在重启时安装。`);
      } else {
        setUpdateState('latest');
        setUpdateMessage('当前已是最新版本。');
      }
    } catch (error) {
      setUpdateState('error');
      setUpdateMessage(error instanceof Error ? `无法检查更新：${error.message}` : '无法检查更新，请稍后重试。');
    }
  }

  async function installUpdate() {
    if (!update || updateState === 'downloading') return;
    setUpdateState('downloading');
    setDownloadedBytes(0);
    setTotalBytes(null);
    setUpdateMessage(`正在下载 ${update.version}，完成后将安全重启安装。`);
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') setTotalBytes(event.data.contentLength ?? null);
        if (event.event === 'Progress') setDownloadedBytes((bytes) => bytes + event.data.chunkLength);
      });
      await relaunch();
    } catch (error) {
      setUpdateState('error');
      setUpdateMessage(error instanceof Error ? `更新未安装：${error.message}` : '更新未安装，请稍后重试。');
    }
  }

  const progress = totalBytes && totalBytes > 0 ? `（${Math.min(100, Math.floor(downloadedBytes / totalBytes * 100))}%）` : '';

  return (
    <section className="page-section settings-page">
      <header className="page-head"><div className="page-title-block"><h1>设置</h1><p>查看安全边界，管理排除规则、自动清理和本地隐私偏好。</p></div></header>

      <div className="settings-group">
        <div className="settings-heading"><span className="settings-icon" aria-hidden="true"><AppWindow /></span><div><h2>外观</h2><p>主题变更立即应用，不影响扫描和清理任务。</p></div></div>
        <div className="setting-row"><span><strong>应用主题</strong><small>选择清盘的显示方式</small></span><div className="segments" aria-label="应用主题">{themeOptions.map((option) => { const Icon = option.icon; return <button type="button" key={option.value} className={theme === option.value ? 'active' : ''} onClick={() => setTheme(option.value)} aria-pressed={theme === option.value}><Icon aria-hidden="true" />{option.label}</button>; })}</div></div>
      </div>

      <div className="settings-group">
        <div className="settings-heading"><span className="settings-icon protected" aria-hidden="true"><FolderLock /></span><div><h2>数据保护</h2><p>保护目录在所有阶段生效；明确路径排除会在只读扫描入口直接剪枝。</p></div></div>
        <div className="setting-row setting-row-stacked"><span><strong>受保护目录</strong><small>这些目录永不进入扫描、清理或分析候选项。</small></span><PathList items={protectedDirectories} emptyText="当前没有额外受保护目录。" /></div>
        <div className="setting-row setting-row-stacked"><span><strong>排除规则</strong><small>本机排除项会在扫描入口被跳过。</small></span><PathList items={exclusionRules} emptyText="当前没有额外排除规则。" /></div>
      </div>

      <div className="settings-group">
        <div className="settings-heading"><span className="settings-icon" aria-hidden="true"><Clock3 /></span><div><h2>自动化</h2><p>自动清理默认关闭；开启后也只处理安全策略允许的低风险候选项。</p></div></div>
        <div className="setting-row"><span><strong>自动清理</strong><small>不会绕过用户数据与不可恢复操作的确认步骤。</small></span><span className="setting-control"><span className="setting-state">{autoCleanupEnabled ? '已开启' : '已关闭'}</span><button type="button" role="switch" aria-label="自动清理" aria-checked={autoCleanupEnabled} className={`switch ${autoCleanupEnabled ? 'on' : ''}`} onClick={() => onAutoCleanupChange?.(!autoCleanupEnabled)} disabled={!onAutoCleanupChange} title={!onAutoCleanupChange ? '当前未提供自动清理设置接口' : undefined}><span /></button></span></div>
      </div>

      <div className="settings-group">
        <div className="settings-heading"><span className="settings-icon" aria-hidden="true"><Download /></span><div><h2>程序更新</h2><p>仅安装来自发布源、且通过内置公钥校验的 Windows 更新包。</p></div></div>
        <div className="setting-row"><span><strong>{update?.version ? `发现新版本 ${update.version}` : '检查桌面端更新'}</strong><small>{updateMessage} {updateState === 'downloading' ? progress : ''}</small></span><div className="segments">{updateState === 'available' && update ? <button type="button" onClick={() => void installUpdate()}><Download aria-hidden="true" />下载并重启</button> : <button type="button" onClick={() => void checkForUpdate()} disabled={updateState === 'checking' || updateState === 'downloading'}><RefreshCw className={updateState === 'checking' ? 'spin' : ''} aria-hidden="true" />{updateState === 'checking' ? '检查中' : '检查更新'}</button>}</div></div>
      </div>

      <div className="settings-group">
        <div className="settings-heading"><span className="settings-icon safe" aria-hidden="true"><LockKeyhole /></span><div><h2>隐私</h2><p>文件扫描、哈希比对和清理结果均在本机处理。</p></div></div>
        <div className="setting-row"><span><strong>本地处理</strong><small>默认不上传文件内容、文件名、完整路径或清理记录。</small></span><span className="locked"><ShieldCheck />仅限本机</span></div>
        <div className="setting-row"><span><strong>删除前确认</strong><small>涉及用户数据或不可逆操作时，确认步骤不会被自动清理绕过。</small></span><span className="locked"><ListFilter />始终启用</span></div>
      </div>
    </section>
  );
}
