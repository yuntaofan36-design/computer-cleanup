use std::{fs::Metadata, io, path::Path};

pub(crate) fn is_link_or_reparse(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub(crate) fn is_offline_or_recall(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
        const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
        const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
        let attributes = metadata.file_attributes();
        attributes
            & (FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_OPEN
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

#[cfg(windows)]
pub(crate) fn has_only_default_data_stream(path: &Path) -> io::Result<bool> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    use windows::{
        core::{HRESULT, PCWSTR},
        Win32::Storage::FileSystem::{
            FindClose, FindFirstStreamW, FindNextStreamW, FindStreamInfoStandard,
            WIN32_FIND_STREAM_DATA,
        },
    };

    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut data = WIN32_FIND_STREAM_DATA::default();
    let handle = unsafe {
        FindFirstStreamW(
            PCWSTR(wide.as_ptr()),
            FindStreamInfoStandard,
            &mut data as *mut _ as *mut c_void,
            0,
        )
    }
    .map_err(|error| io::Error::other(error.to_string()))?;

    let result = loop {
        let end = data
            .cStreamName
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(data.cStreamName.len());
        let stream_name = String::from_utf16_lossy(&data.cStreamName[..end]);
        if !stream_name.eq_ignore_ascii_case("::$DATA") {
            break Ok(false);
        }

        data = WIN32_FIND_STREAM_DATA::default();
        match unsafe { FindNextStreamW(handle, &mut data as *mut _ as *mut c_void) } {
            Ok(()) => {}
            Err(error) if error.code() == HRESULT::from_win32(38) => break Ok(true),
            Err(error) => break Err(io::Error::other(error.to_string())),
        }
    };
    let _ = unsafe { FindClose(handle) };
    result
}

#[cfg(not(windows))]
pub(crate) fn has_only_default_data_stream(_path: &Path) -> io::Result<bool> {
    Ok(true)
}
