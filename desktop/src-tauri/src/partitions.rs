use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

const MAX_LAYOUT_JSON_BYTES: usize = 4 * 1024 * 1024;
const MAX_DISKS: usize = 64;
const MAX_PARTITIONS_PER_DISK: usize = 256;
const MAX_TEXT_LENGTH: usize = 256;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartitionDisk {
    pub number: u32,
    pub friendly_name: String,
    pub partition_style: String,
    pub bus_type: String,
    pub health_status: String,
    pub operational_status: String,
    pub size_bytes: u64,
    pub is_boot: bool,
    pub is_system: bool,
    pub is_offline: bool,
    pub is_read_only: bool,
    pub partitions: Vec<DiskPartition>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskPartition {
    pub partition_number: u32,
    pub drive_letter: Option<String>,
    pub offset_bytes: u64,
    pub size_bytes: u64,
    pub partition_type: String,
    pub gpt_type: String,
    pub is_system: bool,
    pub is_boot: bool,
    pub is_active: bool,
    pub is_hidden: bool,
    pub is_read_only: bool,
    pub no_default_drive_letter: bool,
    pub file_system: String,
    pub label: String,
    pub health_status: String,
    pub free_bytes: u64,
}

fn validate_text(value: &mut String, field: &str) -> Result<(), String> {
    *value = value.trim().to_string();
    if value.len() > MAX_TEXT_LENGTH || value.chars().any(char::is_control) {
        return Err(format!("磁盘布局中的 {field} 字段无效"));
    }
    Ok(())
}

fn validate_drive_letter(value: &mut Option<String>) -> Result<(), String> {
    let Some(letter) = value else {
        return Ok(());
    };
    let normalized = letter.trim().to_ascii_uppercase();
    if normalized.len() != 1 || !normalized.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err("磁盘布局包含无效盘符".into());
    }
    *letter = normalized;
    Ok(())
}

fn validate_partition(
    partition: &mut DiskPartition,
    disk_size: u64,
    previous_end: u64,
) -> Result<u64, String> {
    if partition.partition_number == 0 || partition.size_bytes == 0 {
        return Err("磁盘布局包含无效分区".into());
    }
    let end = partition
        .offset_bytes
        .checked_add(partition.size_bytes)
        .ok_or_else(|| "分区边界发生整数溢出".to_string())?;
    if partition.offset_bytes < previous_end || end > disk_size {
        return Err("磁盘布局包含重叠或越界分区".into());
    }
    if partition.free_bytes > partition.size_bytes {
        return Err("卷可用空间大于分区容量".into());
    }

    validate_drive_letter(&mut partition.drive_letter)?;
    validate_text(&mut partition.partition_type, "分区类型")?;
    validate_text(&mut partition.gpt_type, "GPT 类型")?;
    validate_text(&mut partition.file_system, "文件系统")?;
    validate_text(&mut partition.label, "卷标")?;
    validate_text(&mut partition.health_status, "卷健康状态")?;
    Ok(end)
}

fn parse_partition_layout(bytes: &[u8]) -> Result<Vec<PartitionDisk>, String> {
    if bytes.is_empty() || bytes.len() > MAX_LAYOUT_JSON_BYTES {
        return Err("Windows 返回的磁盘布局数据为空或过大".into());
    }
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim().trim_start_matches('\u{feff}');
    let mut disks: Vec<PartitionDisk> =
        serde_json::from_str(text).map_err(|error| format!("无法解析磁盘布局: {error}"))?;
    if disks.len() > MAX_DISKS {
        return Err("磁盘数量超过安全上限".into());
    }

    disks.sort_by_key(|disk| disk.number);
    let mut disk_numbers = HashSet::new();
    for disk in &mut disks {
        if !disk_numbers.insert(disk.number) || disk.size_bytes == 0 {
            return Err("磁盘布局包含重复编号或无效容量".into());
        }
        if disk.partitions.len() > MAX_PARTITIONS_PER_DISK {
            return Err("单个磁盘的分区数量超过安全上限".into());
        }
        validate_text(&mut disk.friendly_name, "磁盘名称")?;
        validate_text(&mut disk.partition_style, "分区样式")?;
        validate_text(&mut disk.bus_type, "总线类型")?;
        validate_text(&mut disk.health_status, "磁盘健康状态")?;
        validate_text(&mut disk.operational_status, "磁盘运行状态")?;

        disk.partitions
            .sort_by_key(|partition| partition.offset_bytes);
        let mut partition_numbers = HashSet::new();
        let mut previous_end = 0u64;
        for partition in &mut disk.partitions {
            if !partition_numbers.insert(partition.partition_number) {
                return Err("磁盘布局包含重复分区编号".into());
            }
            previous_end = validate_partition(partition, disk.size_bytes, previous_end)?;
        }
    }
    Ok(disks)
}

#[cfg(windows)]
const PARTITION_LAYOUT_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$items = @(
  Get-Disk -ErrorAction Stop | Sort-Object Number | ForEach-Object {
    $disk = $_
    $partitions = @(
      Get-Partition -DiskNumber $disk.Number -ErrorAction SilentlyContinue | Sort-Object Offset | ForEach-Object {
        $partition = $_
        $volume = $partition | Get-Volume -ErrorAction SilentlyContinue | Select-Object -First 1
        [PSCustomObject]@{
          partitionNumber = [uint32]$partition.PartitionNumber
          driveLetter = if ($partition.DriveLetter) { [string]$partition.DriveLetter } else { $null }
          offsetBytes = [uint64]$partition.Offset
          sizeBytes = [uint64]$partition.Size
          partitionType = [string]$partition.Type
          gptType = [string]$partition.GptType
          isSystem = [bool]$partition.IsSystem
          isBoot = [bool]$partition.IsBoot
          isActive = [bool]$partition.IsActive
          isHidden = [bool]$partition.IsHidden
          isReadOnly = [bool]$partition.IsReadOnly
          noDefaultDriveLetter = [bool]$partition.NoDefaultDriveLetter
          fileSystem = if ($volume) { [string]$volume.FileSystem } else { '' }
          label = if ($volume) { [string]$volume.FileSystemLabel } else { '' }
          healthStatus = if ($volume) { [string]$volume.HealthStatus } else { 'Unknown' }
          freeBytes = if ($volume -and $null -ne $volume.SizeRemaining) { [uint64]$volume.SizeRemaining } else { 0 }
        }
      }
    )
    [PSCustomObject]@{
      number = [uint32]$disk.Number
      friendlyName = [string]$disk.FriendlyName
      partitionStyle = [string]$disk.PartitionStyle
      busType = [string]$disk.BusType
      healthStatus = [string]$disk.HealthStatus
      operationalStatus = [string]($disk.OperationalStatus -join ', ')
      sizeBytes = [uint64]$disk.Size
      isBoot = [bool]$disk.IsBoot
      isSystem = [bool]$disk.IsSystem
      isOffline = [bool]$disk.IsOffline
      isReadOnly = [bool]$disk.IsReadOnly
      partitions = $partitions
    }
  }
)
ConvertTo-Json -InputObject $items -Depth 5 -Compress
"#;

#[cfg(windows)]
fn windows_system_file(relative: &Path) -> Result<std::path::PathBuf, String> {
    use std::{env, fs};

    let windows = env::var_os("WINDIR").ok_or_else(|| "无法定位 Windows 系统目录".to_string())?;
    let windows =
        fs::canonicalize(windows).map_err(|error| format!("无法验证 Windows 系统目录: {error}"))?;
    let target = windows.join("System32").join(relative);
    let metadata = fs::symlink_metadata(&target)
        .map_err(|error| format!("无法读取 Windows 系统工具: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Windows 系统工具路径不可信".into());
    }
    let canonical =
        fs::canonicalize(&target).map_err(|error| format!("无法验证 Windows 系统工具: {error}"))?;
    if !canonical.starts_with(&windows) {
        return Err("Windows 系统工具不在系统目录内".into());
    }
    Ok(canonical)
}

#[cfg(windows)]
#[tauri::command]
pub fn list_partition_disks() -> Result<Vec<PartitionDisk>, String> {
    use std::{os::windows::process::CommandExt, path::PathBuf, process::Command};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let powershell = windows_system_file(&PathBuf::from("WindowsPowerShell/v1.0/powershell.exe"))?;
    let output = Command::new(powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            PARTITION_LAYOUT_SCRIPT,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("无法启动 Windows 磁盘查询: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            "Windows 磁盘查询未成功完成".into()
        } else {
            format!("Windows 磁盘查询失败: {detail}")
        });
    }
    parse_partition_layout(&output.stdout)
}

#[cfg(not(windows))]
#[tauri::command]
pub fn list_partition_disks() -> Result<Vec<PartitionDisk>, String> {
    Err("磁盘分区功能仅支持 Windows".into())
}

#[cfg(windows)]
#[tauri::command]
pub fn open_windows_disk_management() -> Result<(), String> {
    use std::{path::PathBuf, process::Command};

    let mmc = windows_system_file(&PathBuf::from("mmc.exe"))?;
    let disk_management = windows_system_file(&PathBuf::from("diskmgmt.msc"))?;
    Command::new(mmc)
        .arg(disk_management)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法打开 Windows 磁盘管理: {error}"))
}

#[cfg(not(windows))]
#[tauri::command]
pub fn open_windows_disk_management() -> Result<(), String> {
    Err("磁盘分区功能仅支持 Windows".into())
}

#[cfg(test)]
mod tests {
    use super::parse_partition_layout;

    #[test]
    fn parses_and_sorts_a_valid_partition_layout() {
        let json = br#"[{"number":1,"friendlyName":"NVMe","partitionStyle":"GPT","busType":"NVMe","healthStatus":"Healthy","operationalStatus":"Online","sizeBytes":1000000,"isBoot":true,"isSystem":true,"isOffline":false,"isReadOnly":false,"partitions":[{"partitionNumber":2,"driveLetter":"c","offsetBytes":200000,"sizeBytes":700000,"partitionType":"Basic","gptType":"basic","isSystem":false,"isBoot":true,"isActive":false,"isHidden":false,"isReadOnly":false,"noDefaultDriveLetter":false,"fileSystem":"NTFS","label":"Windows","healthStatus":"Healthy","freeBytes":300000},{"partitionNumber":1,"driveLetter":null,"offsetBytes":1000,"sizeBytes":100000,"partitionType":"System","gptType":"efi","isSystem":true,"isBoot":false,"isActive":false,"isHidden":true,"isReadOnly":false,"noDefaultDriveLetter":true,"fileSystem":"FAT32","label":"EFI","healthStatus":"Healthy","freeBytes":10000}]}]"#;

        let disks = parse_partition_layout(json).expect("layout should parse");

        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].partitions[0].partition_number, 1);
        assert_eq!(disks[0].partitions[1].drive_letter.as_deref(), Some("C"));
    }

    #[test]
    fn rejects_overlapping_or_out_of_bounds_partitions() {
        let overlapping = br#"[{"number":0,"friendlyName":"Disk","partitionStyle":"GPT","busType":"SATA","healthStatus":"Healthy","operationalStatus":"Online","sizeBytes":1000,"isBoot":false,"isSystem":false,"isOffline":false,"isReadOnly":false,"partitions":[{"partitionNumber":1,"driveLetter":"D","offsetBytes":100,"sizeBytes":700,"partitionType":"Basic","gptType":"basic","isSystem":false,"isBoot":false,"isActive":false,"isHidden":false,"isReadOnly":false,"noDefaultDriveLetter":false,"fileSystem":"NTFS","label":"","healthStatus":"Healthy","freeBytes":100},{"partitionNumber":2,"driveLetter":"E","offsetBytes":700,"sizeBytes":200,"partitionType":"Basic","gptType":"basic","isSystem":false,"isBoot":false,"isActive":false,"isHidden":false,"isReadOnly":false,"noDefaultDriveLetter":false,"fileSystem":"NTFS","label":"","healthStatus":"Healthy","freeBytes":100}]}]"#;
        assert!(parse_partition_layout(overlapping)
            .expect_err("overlap should be rejected")
            .contains("重叠"));
    }
}
