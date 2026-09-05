import type {
  AppEntry,
  CleanupItem,
  DirectoryUsage,
  DiskInfo,
  DuplicateGroup,
  LargeFileEntry,
  OperationRecord,
  PartitionDisk,
  StartupEntry,
  StorageCategory,
} from './types';

const GB = 1024 ** 3;
const MB = 1024 ** 2;

export const partitionDisks: PartitionDisk[] = [
  {
    number: 0, friendlyName: 'Qingpan NVMe 512GB', partitionStyle: 'GPT', busType: 'NVMe', healthStatus: 'Healthy', operationalStatus: 'Online', sizeBytes: 512 * GB,
    isBoot: true, isSystem: true, isOffline: false, isReadOnly: false,
    partitions: [
      { partitionNumber: 1, driveLetter: null, offsetBytes: 1 * MB, sizeBytes: 100 * MB, partitionType: 'System', gptType: 'EFI', isSystem: true, isBoot: false, isActive: false, isHidden: true, isReadOnly: false, noDefaultDriveLetter: true, fileSystem: 'FAT32', label: 'EFI SYSTEM', healthStatus: 'Healthy', freeBytes: 62 * MB },
      { partitionNumber: 2, driveLetter: 'C', offsetBytes: 101 * MB, sizeBytes: 300 * GB, partitionType: 'Basic', gptType: 'Basic data', isSystem: false, isBoot: true, isActive: false, isHidden: false, isReadOnly: false, noDefaultDriveLetter: false, fileSystem: 'NTFS', label: 'Windows', healthStatus: 'Healthy', freeBytes: 112 * GB },
      { partitionNumber: 3, driveLetter: null, offsetBytes: (300 * GB) + (101 * MB), sizeBytes: 900 * MB, partitionType: 'Recovery', gptType: 'Recovery', isSystem: false, isBoot: false, isActive: false, isHidden: true, isReadOnly: false, noDefaultDriveLetter: true, fileSystem: 'NTFS', label: 'Windows RE', healthStatus: 'Healthy', freeBytes: 124 * MB },
    ],
  },
  {
    number: 1, friendlyName: 'Qingpan Data 1TB', partitionStyle: 'GPT', busType: 'SATA', healthStatus: 'Healthy', operationalStatus: 'Online', sizeBytes: 1024 * GB,
    isBoot: false, isSystem: false, isOffline: false, isReadOnly: false,
    partitions: [
      { partitionNumber: 1, driveLetter: 'D', offsetBytes: 1 * MB, sizeBytes: 950 * GB, partitionType: 'Basic', gptType: 'Basic data', isSystem: false, isBoot: false, isActive: false, isHidden: false, isReadOnly: false, noDefaultDriveLetter: false, fileSystem: 'NTFS', label: '资料', healthStatus: 'Healthy', freeBytes: 642 * GB },
    ],
  },
];

export const disks: DiskInfo[] = [
  { id: 'c', name: 'Windows', mount: 'C:', totalBytes: 512 * GB, freeBytes: 183.4 * GB },
  { id: 'd', name: '资料盘', mount: 'D:', totalBytes: 1024 * GB, freeBytes: 642.8 * GB },
];

export const cleanupItems: CleanupItem[] = [
  {
    id: 'temp', scope: 'system', category: '系统临时文件', product: 'Windows', name: '用户临时文件',
    path: '%LOCALAPPDATA%\\Temp', description: '超过 72 小时且未被占用的临时文件', reason: 'Windows 与应用会在需要时重新创建',
    sizeBytes: 1.84 * GB, fileCount: 3241, risk: 'low', confidence: 'high', impact: 'none', recoverability: 'recoverable', deleteMode: 'quarantine', selectable: true,
  },
  {
    id: 'thumbs', scope: 'system', category: '系统临时文件', product: '文件资源管理器', name: '缩略图缓存',
    path: '%LOCALAPPDATA%\\Microsoft\\Windows\\Explorer\\thumbcache_*.db', description: '图片和视频预览的派生缓存', reason: '仅匹配 thumbcache 文件，资源管理器会自动重建',
    sizeBytes: 428 * MB, fileCount: 18, risk: 'low', confidence: 'high', impact: 'rebuild', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'd3d', scope: 'system', category: '图形缓存', product: 'DirectX', name: '着色器缓存',
    path: '%LOCALAPPDATA%\\D3DSCache', description: '显卡驱动生成的着色器编译缓存', reason: '内容可重建，首次启动游戏可能多等待几秒',
    sizeBytes: 736 * MB, fileCount: 912, risk: 'low', confidence: 'high', impact: 'rebuild', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'crash', scope: 'system', category: '诊断记录', product: 'Windows', name: '应用崩溃转储',
    path: '%LOCALAPPDATA%\\CrashDumps', description: '用于定位应用崩溃问题的诊断文件', reason: '不是运行所需文件，但删除后无法用于历史故障分析',
    sizeBytes: 1.21 * GB, fileCount: 14, risk: 'medium', confidence: 'high', impact: 'user_data', recoverability: 'irreversible', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'edge-cache', scope: 'browser', category: '网页缓存', product: 'Microsoft Edge · 默认配置', name: 'HTTP 与代码缓存',
    path: '%LOCALAPPDATA%\\Microsoft\\Edge\\User Data\\Default\\Cache', description: '网页图片、脚本和已编译代码', reason: '不包含 Cookie、密码、书签与浏览历史',
    sizeBytes: 1.36 * GB, fileCount: 4382, risk: 'low', confidence: 'high', impact: 'rebuild', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'chrome-cache', scope: 'browser', category: '网页缓存', product: 'Google Chrome · Profile 1', name: 'HTTP 与 GPU 缓存',
    path: '%LOCALAPPDATA%\\Google\\Chrome\\User Data\\Profile 1\\Cache', description: '可重新下载的网页资源和图形缓存', reason: '浏览器运行时锁定的文件会自动跳过',
    sizeBytes: 924 * MB, fileCount: 2981, risk: 'low', confidence: 'high', impact: 'rebuild', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'firefox-cache', scope: 'browser', category: '网页缓存', product: 'Mozilla Firefox · default-release', name: '网页缓存',
    path: '%LOCALAPPDATA%\\Mozilla\\Firefox\\Profiles\\*\\cache2', description: 'Firefox 可重建的网络缓存', reason: '仅进入 cache2，不扫描配置文件中的用户数据',
    sizeBytes: 386 * MB, fileCount: 1280, risk: 'low', confidence: 'high', impact: 'rebuild', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'browser-state', scope: 'browser', category: '受保护数据', product: '所有浏览器', name: 'Cookie、会话与站点数据',
    path: '浏览器配置文件', description: '保存登录状态、站点偏好和离线数据', reason: '清理会导致退出登录，本产品不会在一键清理中处理',
    sizeBytes: 264 * MB, fileCount: 0, risk: 'high', confidence: 'high', impact: 'signout', recoverability: 'protected', deleteMode: 'permanent', selectable: false,
  },
  {
    id: 'browser-identity', scope: 'browser', category: '受保护数据', product: '所有浏览器', name: '密码、书签与自动填充',
    path: '浏览器配置文件', description: '账号凭据和用户主动保存的数据', reason: '核心用户数据，规则内核永久拒绝普通清理请求',
    sizeBytes: 42 * MB, fileCount: 0, risk: 'high', confidence: 'high', impact: 'user_data', recoverability: 'protected', deleteMode: 'permanent', selectable: false,
  },
  {
    id: 'wechat-local-wechat-cache', scope: 'wechat', category: '微信运行缓存', product: '微信', name: '网络缓存',
    path: '%LOCALAPPDATA%\\Tencent\\WeChat\\Cache', description: '微信运行时生成、可重新下载的网络资源', reason: '仅匹配 AppData 下名为 Cache 的明确叶子目录，不进入 WeChat Files',
    sizeBytes: 1.12 * GB, fileCount: 2864, risk: 'low', confidence: 'high', impact: 'rebuild', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'wechat-roaming-weixin-logs', scope: 'wechat', category: '微信诊断数据', product: '微信 4.x', name: '运行日志',
    path: '%APPDATA%\\Tencent\\Weixin\\Logs', description: '用于排查客户端运行问题的诊断日志', reason: '微信关闭后仅处理 Logs 目录中的诊断数据',
    sizeBytes: 46 * MB, fileCount: 218, risk: 'low', confidence: 'high', impact: 'none', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'wechat-local-xwechat-crash-reports', scope: 'wechat', category: '微信诊断数据', product: '微信 4.x', name: '崩溃报告',
    path: '%LOCALAPPDATA%\\Tencent\\xwechat\\Crashpad\\reports', description: '客户端异常退出后留下的崩溃诊断报告', reason: '仅匹配 Crashpad\\reports 叶子目录，不扫描聊天附件或数据库',
    sizeBytes: 128 * MB, fileCount: 12, risk: 'low', confidence: 'high', impact: 'none', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'wechat-user-demo-chat-records', scope: 'wechat', category: '微信聊天记录', product: '微信 · 当前账户', name: '聊天记录',
    path: '%USERPROFILE%\\Documents\\WeChat Files\\wxid_demo\\Msg', description: '本地聊天数据库与索引', reason: '用户主动创建的数据，默认不勾选且删除后无法恢复',
    sizeBytes: 3.8 * GB, fileCount: 78, risk: 'high', confidence: 'high', impact: 'user_data', recoverability: 'irreversible', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'wechat-user-demo-images', scope: 'wechat', category: '微信图片', product: '微信 · 当前账户', name: '聊天图片',
    path: '%USERPROFILE%\\Documents\\WeChat Files\\wxid_demo\\FileStorage\\Image', description: '聊天中接收和保存的原图与图片附件', reason: '仅匹配 Image 与 MsgAttach 中的 Image/Thumb 目录',
    sizeBytes: 6.4 * GB, fileCount: 12480, risk: 'high', confidence: 'high', impact: 'user_data', recoverability: 'irreversible', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'wechat-user-demo-videos', scope: 'wechat', category: '微信视频', product: '微信 · 当前账户', name: '聊天视频',
    path: '%USERPROFILE%\\Documents\\WeChat Files\\wxid_demo\\FileStorage\\Video', description: '聊天中接收和保存的视频', reason: '仅匹配 Video 目录，不按文件扩展名猜测',
    sizeBytes: 4.7 * GB, fileCount: 426, risk: 'high', confidence: 'high', impact: 'user_data', recoverability: 'irreversible', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'wechat-user-demo-files', scope: 'wechat', category: '微信文件', product: '微信 · 当前账户', name: '聊天文件',
    path: '%USERPROFILE%\\Documents\\WeChat Files\\wxid_demo\\FileStorage\\File', description: '聊天中接收和保存的文档与压缩包', reason: '仅匹配 File 目录，用户主动决定是否清理',
    sizeBytes: 2.3 * GB, fileCount: 684, risk: 'high', confidence: 'high', impact: 'user_data', recoverability: 'irreversible', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'wechat-user-demo-voices', scope: 'wechat', category: '微信语音', product: '微信 · 当前账户', name: '语音消息',
    path: '%USERPROFILE%\\Documents\\WeChat Files\\wxid_demo\\FileStorage\\MsgAttach\\*\\Audio', description: '聊天中的语音消息', reason: '仅匹配 Audio/Voice/Voice2 目录，不使用 .dat 扩展名误判',
    sizeBytes: 386 * MB, fileCount: 1940, risk: 'high', confidence: 'high', impact: 'user_data', recoverability: 'irreversible', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'wechat-user-demo-favorites', scope: 'wechat', category: '微信收藏', product: '微信 · 当前账户', name: '收藏内容',
    path: '%USERPROFILE%\\Documents\\WeChat Files\\wxid_demo\\FileStorage\\Fav', description: '保存在本机的微信收藏内容', reason: '仅匹配 Fav 目录，默认不勾选',
    sizeBytes: 742 * MB, fileCount: 860, risk: 'high', confidence: 'high', impact: 'user_data', recoverability: 'irreversible', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'wechat-user-demo-emotions', scope: 'wechat', category: '微信表情', product: '微信 · 当前账户', name: '自定义表情',
    path: '%USERPROFILE%\\Documents\\WeChat Files\\wxid_demo\\FileStorage\\CustomEmotion', description: '用户保存和下载的自定义表情', reason: '仅匹配 CustomEmotion 目录，默认不勾选',
    sizeBytes: 318 * MB, fileCount: 1101, risk: 'high', confidence: 'high', impact: 'user_data', recoverability: 'irreversible', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'figma-cache', scope: 'apps', category: '应用缓存', product: 'Figma', name: '预览与网络缓存',
    path: '%APPDATA%\\Figma\\Cache', description: '本地预览和可重新下载的网络资源', reason: '不包含草稿、项目和登录凭据',
    sizeBytes: 612 * MB, fileCount: 1640, risk: 'low', confidence: 'high', impact: 'rebuild', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'discord-cache', scope: 'apps', category: '应用缓存', product: 'Discord', name: '媒体与 GPU 缓存',
    path: '%APPDATA%\\discord\\Cache', description: '聊天中已下载的临时媒体预览', reason: '仅清理已知 Cache/Code Cache/GPUCache 目录',
    sizeBytes: 874 * MB, fileCount: 2124, risk: 'low', confidence: 'high', impact: 'rebuild', recoverability: 'rebuildable', deleteMode: 'permanent', selectable: true,
  },
  {
    id: 'orphan-app', scope: 'apps', category: '卸载残留', product: '旧版绘图工具', name: '疑似卸载残留',
    path: '%LOCALAPPDATA%\\OldSketch', description: '目录已 185 天未修改，但安装证据不完整', reason: '识别置信度不足，只展示证据，不允许默认清理',
    sizeBytes: 1.9 * GB, fileCount: 644, risk: 'high', confidence: 'low', impact: 'user_data', recoverability: 'protected', deleteMode: 'recycle_bin', selectable: false,
  },
];

export const apps: AppEntry[] = [
  { id: '1', name: 'Visual Studio Code', publisher: 'Microsoft Corporation', version: '1.107.0', sizeBytes: 1.42 * GB, cacheBytes: 286 * MB, installedAt: '2026-06-18', lastUsed: '今天' },
  { id: '2', name: 'Figma', publisher: 'Figma, Inc.', version: '126.2', sizeBytes: 2.1 * GB, cacheBytes: 612 * MB, installedAt: '2026-05-24', lastUsed: '昨天' },
  { id: '3', name: 'Steam', publisher: 'Valve Corporation', version: '2.10', sizeBytes: 4.8 * GB, cacheBytes: 188 * MB, installedAt: '2026-04-09', lastUsed: '3 天前' },
  { id: '4', name: 'Discord', publisher: 'Discord Inc.', version: '1.0.9182', sizeBytes: 714 * MB, cacheBytes: 874 * MB, installedAt: '2026-03-11', lastUsed: '今天' },
  { id: '5', name: '7-Zip', publisher: 'Igor Pavlov', version: '24.09', sizeBytes: 6 * MB, cacheBytes: 0, installedAt: '2025-12-04', lastUsed: '42 天前' },
];

export const startups: StartupEntry[] = [
  { id: 's1', name: 'Microsoft OneDrive', publisher: 'Microsoft', command: 'OneDrive.exe /background', enabled: true, impact: '中', scope: '当前用户' },
  { id: 's2', name: 'Windows Security', publisher: 'Microsoft', command: 'SecurityHealthSystray.exe', enabled: true, impact: '低', scope: '所有用户' },
  { id: 's3', name: 'Steam Client Bootstrapper', publisher: 'Valve', command: 'steam.exe -silent', enabled: false, impact: '高', scope: '当前用户' },
];

export const records: OperationRecord[] = [
  { id: 'r1', kind: 'cleanup', title: '低风险缓存清理', createdAt: '今天 09:42', reclaimedBytes: 4.82 * GB, stagedBytes: 0, status: 'success', detail: '1,284 个文件已复检并清理，17 个已变化文件被跳过' },
  { id: 'r2', kind: 'restore', title: '恢复重复文件整理', createdAt: '7 月 18 日 16:20', reclaimedBytes: 0, stagedBytes: 684 * MB, status: 'success', detail: '已恢复到备用名称，未覆盖原路径现有文件' },
  { id: 'r3', kind: 'cleanup', title: '浏览器缓存', createdAt: '7 月 15 日 11:08', reclaimedBytes: 1.17 * GB, stagedBytes: 0, status: 'partial', detail: '3 个浏览器锁定文件已安全跳过' },
];

export const largeFiles: LargeFileEntry[] = [
  { id: 'lf1', name: 'Windows11_24H2.iso', path: 'D:\\Downloads\\Windows11_24H2.iso', sizeBytes: 6.38 * GB, allocatedBytes: 6.38 * GB, modifiedAt: '2026-06-12', type: '磁盘镜像', sensitivity: 'attention', note: '安装介质，不是垃圾文件' },
  { id: 'lf2', name: 'ext4.vhdx', path: 'C:\\Users\\User\\AppData\\Local\\Packages\\Ubuntu\\LocalState\\ext4.vhdx', sizeBytes: 42.6 * GB, allocatedBytes: 18.4 * GB, modifiedAt: '今天 10:21', type: 'WSL 虚拟磁盘', sensitivity: 'protected', note: '删除会损坏 Linux 环境' },
  { id: 'lf3', name: 'launch-film-final.mov', path: 'D:\\Media\\Projects\\Launch\\launch-film-final.mov', sizeBytes: 12.8 * GB, allocatedBytes: 12.8 * GB, modifiedAt: '2026-07-08', type: '视频', sensitivity: 'normal' },
  { id: 'lf4', name: 'outlook-archive.pst', path: 'D:\\Mail\\outlook-archive.pst', sizeBytes: 8.1 * GB, allocatedBytes: 8.1 * GB, modifiedAt: '昨天', type: '邮件存档', sensitivity: 'protected', note: '用户邮件数据' },
  { id: 'lf5', name: 'project-backup-2026-05.zip', path: 'D:\\Backups\\project-backup-2026-05.zip', sizeBytes: 5.74 * GB, allocatedBytes: 5.74 * GB, modifiedAt: '2026-05-31', type: '备份压缩包', sensitivity: 'attention' },
  { id: 'lf6', name: 'screen-recording-0412.mp4', path: 'C:\\Users\\User\\Videos\\Captures\\screen-recording-0412.mp4', sizeBytes: 3.26 * GB, allocatedBytes: 3.26 * GB, modifiedAt: '2026-04-12', type: '视频', sensitivity: 'normal' },
];

export const duplicateGroups: DuplicateGroup[] = [
  {
    id: 'dg1', hash: 'SHA-256 7b2e…a91c', sizeBytes: 1.42 * GB, reclaimableBytes: 1.42 * GB, match: 'full_hash',
    members: [
      { id: 'dg1a', name: 'camera-roll-2025.zip', path: 'D:\\Photos\\Archive\\camera-roll-2025.zip', modifiedAt: '2026-01-03', suggestedKeep: true },
      { id: 'dg1b', name: 'camera-roll-2025 - 副本.zip', path: 'D:\\Downloads\\camera-roll-2025 - 副本.zip', modifiedAt: '2026-02-11', suggestedKeep: false },
    ],
  },
  {
    id: 'dg2', hash: 'SHA-256 19d4…e802', sizeBytes: 684 * MB, reclaimableBytes: 1.34 * GB, match: 'full_hash',
    members: [
      { id: 'dg2a', name: 'brand-assets-v8.psd', path: 'D:\\Design\\Master\\brand-assets-v8.psd', modifiedAt: '2026-06-20', suggestedKeep: true },
      { id: 'dg2b', name: 'brand-assets-v8.psd', path: 'D:\\Design\\Exports\\brand-assets-v8.psd', modifiedAt: '2026-06-20', suggestedKeep: false },
      { id: 'dg2c', name: 'brand-assets-final.psd', path: 'D:\\Temp\\brand-assets-final.psd', modifiedAt: '2026-06-20', suggestedKeep: false },
    ],
  },
  {
    id: 'dg3', hash: 'SHA-256 8a11…09bd', sizeBytes: 224 * MB, reclaimableBytes: 224 * MB, match: 'full_hash',
    members: [
      { id: 'dg3a', name: 'family-video.mp4', path: 'D:\\Photos\\Family\\family-video.mp4', modifiedAt: '2025-12-28', suggestedKeep: true, protected: true },
      { id: 'dg3b', name: 'family-video.mp4', path: 'D:\\OneDrive\\Family\\family-video.mp4', modifiedAt: '2025-12-28', suggestedKeep: false, protected: true },
    ],
  },
];

export const directories: DirectoryUsage[] = [
  { id: 'users', name: 'Users', path: 'C:\\Users', sizeBytes: 168.2 * GB, percent: 51, color: '#265dff', kind: '用户文件', fileCount: 284192 },
  { id: 'programs', name: 'Program Files', path: 'C:\\Program Files', sizeBytes: 82.4 * GB, percent: 25, color: '#10a37f', kind: '应用', fileCount: 142833 },
  { id: 'windows', name: 'Windows', path: 'C:\\Windows', sizeBytes: 51.7 * GB, percent: 16, color: '#f0a02f', kind: '系统', fileCount: 198442 },
  { id: 'programdata', name: 'ProgramData', path: 'C:\\ProgramData', sizeBytes: 18.6 * GB, percent: 6, color: '#7b67d8', kind: '应用数据', fileCount: 41822 },
  { id: 'recovery', name: 'Recovery', path: 'C:\\Recovery', sizeBytes: 5.2 * GB, percent: 2, color: '#82909f', kind: '受保护', fileCount: 62 },
];

export const storageCategories: StorageCategory[] = [
  { id: 'apps', label: '应用与游戏', sizeBytes: 112.4 * GB, color: '#265dff', description: '已安装应用和游戏资源' },
  { id: 'media', label: '图片与视频', sizeBytes: 72.1 * GB, color: '#10a37f', description: '用户媒体文件' },
  { id: 'system', label: '系统与保留', sizeBytes: 55.9 * GB, color: '#f0a02f', description: 'Windows 与恢复分区内容' },
  { id: 'docs', label: '文档与项目', sizeBytes: 46.7 * GB, color: '#e05d6f', description: '文档、源代码和设计文件' },
  { id: 'other', label: '其他', sizeBytes: 39.5 * GB, color: '#7b67d8', description: '尚未归类的内容' },
];
