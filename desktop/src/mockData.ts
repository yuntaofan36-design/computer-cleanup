import type { AppEntry, CleanupItem, DiskInfo, OperationRecord, StartupEntry } from './types';
export const disks: DiskInfo[] = [{ id:'c', name:'本地磁盘', mount:'C:', totalBytes:512*1024**3, freeBytes:183.4*1024**3 }];
export const cleanupItems: CleanupItem[] = [
  {id:'temp',category:'系统缓存',name:'临时文件',path:'%TEMP%',description:'应用运行时产生的临时数据',sizeBytes:1.84*1024**3,risk:'low',deleteMode:'permanent'},
  {id:'thumbs',category:'系统缓存',name:'缩略图缓存',path:'Explorer 缓存',description:'Windows 会在需要时重新生成',sizeBytes:428*1024**2,risk:'low',deleteMode:'permanent'},
  {id:'browser',category:'应用缓存',name:'浏览器缓存',path:'Edge · Chrome',description:'网页资源缓存，不会清除登录状态',sizeBytes:2.26*1024**3,risk:'low',deleteMode:'permanent'},
  {id:'recycle',category:'用户文件',name:'回收站',path:'C:\\$Recycle.Bin',description:'清空后无法从回收站恢复',sizeBytes:6.72*1024**3,risk:'medium',deleteMode:'permanent'},
  {id:'updates',category:'系统文件',name:'更新下载残留',path:'Windows Update',description:'可能需要管理员权限',sizeBytes:3.1*1024**3,risk:'medium',deleteMode:'permanent'}
];
export const apps: AppEntry[] = [
  {id:'1',name:'Visual Studio Code',publisher:'Microsoft Corporation',version:'1.107.0',sizeBytes:472*1024**2,installedAt:'2026-06-18'},
  {id:'2',name:'Figma',publisher:'Figma, Inc.',version:'126.2',sizeBytes:388*1024**2,installedAt:'2026-05-24'},
  {id:'3',name:'Steam',publisher:'Valve Corporation',version:'2.10',sizeBytes:1.2*1024**3,installedAt:'2026-04-09'}
];
export const startups: StartupEntry[] = [
  {id:'s1',name:'Microsoft OneDrive',publisher:'Microsoft',command:'OneDrive.exe /background',enabled:true,impact:'中',scope:'当前用户'},
  {id:'s2',name:'Windows Security',publisher:'Microsoft',command:'SecurityHealthSystray.exe',enabled:true,impact:'低',scope:'所有用户'},
  {id:'s3',name:'Steam Client Bootstrapper',publisher:'Valve',command:'steam.exe -silent',enabled:false,impact:'高',scope:'当前用户'}
];
export const records: OperationRecord[] = [
  {id:'r1',kind:'清理',title:'安全清理',createdAt:'今天 09:42',reclaimedBytes:4.82*1024**3,status:'success',detail:'已清理 1,284 个项目'},
  {id:'r2',kind:'卸载',title:'卸载 Zoom Workplace',createdAt:'7月8日 16:20',reclaimedBytes:684*1024**2,status:'success',detail:'已调用应用卸载程序'},
  {id:'r3',kind:'清理',title:'浏览器缓存',createdAt:'7月5日 11:08',reclaimedBytes:1.17*1024**3,status:'partial',detail:'3 个文件正在使用'}
];
