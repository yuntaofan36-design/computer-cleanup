import {
  AppWindow,
  Clock3,
  FolderLock,
  ListFilter,
  LockKeyhole,
  Moon,
  ShieldCheck,
  Sun,
} from 'lucide-react';

export type ThemeMode = 'system' | 'light' | 'dark';

export interface SettingsPageProps {
  protectedDirectories: string[];
  exclusionRules: string[];
  autoCleanupEnabled?: boolean;
  onAutoCleanupChange?: (enabled: boolean) => void;
  theme: ThemeMode;
  setTheme: (theme: ThemeMode) => void;
}

const themeOptions: Array<{
  value: ThemeMode;
  label: string;
  icon: typeof AppWindow;
}> = [
  { value: 'system', label: '跟随系统', icon: AppWindow },
  { value: 'light', label: '浅色', icon: Sun },
  { value: 'dark', label: '深色', icon: Moon },
];

function PathList({ items, emptyText }: { items: string[]; emptyText: string }): JSX.Element {
  if (items.length === 0) {
    return <p className="setting-empty">{emptyText}</p>;
  }

  return (
    <ul className="path-list">
      {items.map((item) => (
        <li key={item}>
          <code>{item}</code>
        </li>
      ))}
    </ul>
  );
}

export default function SettingsPage({
  protectedDirectories,
  exclusionRules,
  autoCleanupEnabled = false,
  onAutoCleanupChange,
  theme,
  setTheme,
}: SettingsPageProps): JSX.Element {
  return (
    <section className="page-section settings-page">
      <header className="page-head">
        <div className="page-title-block">
          <h1>设置</h1>
          <p>查看安全边界，管理排除规则、自动清理和本地隐私偏好。</p>
        </div>
      </header>

      <div className="settings-group">
        <div className="settings-heading">
          <span className="settings-icon" aria-hidden="true"><AppWindow /></span>
          <div>
            <h2>外观</h2>
            <p>主题变更立即应用，不影响扫描和清理任务。</p>
          </div>
        </div>
        <div className="setting-row">
          <span>
            <strong>应用主题</strong>
            <small>选择清盘的显示方式</small>
          </span>
          <div className="segments" aria-label="应用主题">
            {themeOptions.map((option) => {
              const Icon = option.icon;
              return (
                <button
                  type="button"
                  key={option.value}
                  className={theme === option.value ? 'active' : ''}
                  onClick={() => setTheme(option.value)}
                  aria-pressed={theme === option.value}
                >
                  <Icon aria-hidden="true" />
                  {option.label}
                </button>
              );
            })}
          </div>
        </div>
      </div>

      <div className="settings-group">
        <div className="settings-heading">
          <span className="settings-icon protected" aria-hidden="true"><FolderLock /></span>
          <div>
            <h2>数据保护</h2>
            <p>保护目录在所有阶段生效；明确路径排除会在只读扫描入口直接剪枝。</p>
          </div>
        </div>
        <div className="setting-row setting-row-stacked">
          <span>
            <strong>受保护目录</strong>
            <small>这些路径只做容量统计，不会成为清理候选项。</small>
          </span>
          <PathList items={protectedDirectories} emptyText="当前没有从扫描策略获取到受保护目录。" />
        </div>
        <div className="setting-row setting-row-stacked">
          <span>
            <strong>排除规则</strong>
            <small>绝对路径会在扫描入口跳过；文件类型策略用于风险标记，不会据此自动删除。</small>
          </span>
          <PathList items={exclusionRules} emptyText="尚未添加额外排除规则。" />
        </div>
      </div>

      <div className="settings-group">
        <div className="settings-heading">
          <span className="settings-icon" aria-hidden="true"><Clock3 /></span>
          <div>
            <h2>自动化</h2>
            <p>自动清理默认关闭；开启后也只处理安全策略允许的低风险候选项。</p>
          </div>
        </div>
        <div className="setting-row">
          <span>
            <strong>自动清理</strong>
            <small>{autoCleanupEnabled ? '已开启，将沿用受保护目录和排除规则。' : '默认关闭，不会在后台自行删除内容。'}</small>
          </span>
          <span className="setting-control">
            <span className="setting-state">{autoCleanupEnabled ? '已开启' : '关闭（默认）'}</span>
            <button
              type="button"
              role="switch"
              aria-label="自动清理"
              aria-checked={autoCleanupEnabled}
              className={`switch ${autoCleanupEnabled ? 'on' : ''}`}
              onClick={() => onAutoCleanupChange?.(!autoCleanupEnabled)}
              disabled={!onAutoCleanupChange}
              title={!onAutoCleanupChange ? '当前未提供自动清理设置接口' : undefined}
            >
              <span />
            </button>
          </span>
        </div>
      </div>

      <div className="settings-group">
        <div className="settings-heading">
          <span className="settings-icon safe" aria-hidden="true"><LockKeyhole /></span>
          <div>
            <h2>隐私</h2>
            <p>文件扫描、哈希比对和清理结果均在本机处理。</p>
          </div>
        </div>
        <div className="setting-row">
          <span>
            <strong>本地处理</strong>
            <small>默认不上传文件内容、文件名、完整路径或清理记录。</small>
          </span>
          <span className="locked"><ShieldCheck />仅限本机</span>
        </div>
        <div className="setting-row">
          <span>
            <strong>删除前确认</strong>
            <small>涉及用户数据或不可逆操作时，确认步骤不会被自动清理绕过。</small>
          </span>
          <span className="locked"><ListFilter />始终启用</span>
        </div>
      </div>
    </section>
  );
}
