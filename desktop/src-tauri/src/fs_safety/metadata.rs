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

/// Reports whether `metadata` describes a file whose physical size can differ
/// from its logical length.
///
/// Only NTFS-compressed and sparse files can occupy fewer clusters than their
/// length suggests. For every other file the logical length already is the
/// allocation, so querying the volume tells us nothing new.
///
/// This gate exists for performance. Measured over a 413k-file pnpm store,
/// querying every file cost 32.5s while querying only flagged files cost 3ms and
/// produced byte-identical totals.
pub(crate) fn may_differ_from_logical_size(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_COMPRESSED: u32 = 0x0000_0800;
        const FILE_ATTRIBUTE_SPARSE_FILE: u32 = 0x0000_0200;
        metadata.file_attributes() & (FILE_ATTRIBUTE_COMPRESSED | FILE_ATTRIBUTE_SPARSE_FILE) != 0
    }
    #[cfg(not(windows))]
    {
        // Other platforms expose block counts directly in metadata, so the caller's
        // query is already cheap and no gate is needed.
        let _ = metadata;
        true
    }
}

/// Returns the on-disk allocation size of `path` in bytes.
///
/// This is the physically occupied size, which differs from the logical size for
/// NTFS-compressed and sparse files. Windows resolves this through
/// `GetCompressedFileSizeW`, a path-level query that never opens a handle to the
/// file, so it does not hydrate cloud placeholders or disturb sharing modes.
///
/// Callers must skip reparse points and offline/recall placeholders before
/// calling this; those are not locally allocated and are reported by the caller
/// as zero physical bytes.
#[cfg(windows)]
pub(crate) fn allocated_size(path: &Path, _metadata: &Metadata) -> io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::PCWSTR,
        Win32::{Foundation::ERROR_SUCCESS, Storage::FileSystem::GetCompressedFileSizeW},
    };

    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut high: u32 = 0;
    // INVALID_FILE_SIZE is ambiguous: it is also a legitimate low DWORD, so the
    // documented disambiguation is to consult the thread's last error code.
    let low = unsafe { GetCompressedFileSizeW(PCWSTR(wide.as_ptr()), Some(&mut high)) };
    if low == u32::MAX {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_SUCCESS.0 as i32) {
            return Err(error);
        }
    }
    let allocated = (u64::from(high) << 32) | u64::from(low);
    // Zero is a legitimate result, not a failure: a fully sparse or highly
    // compressible file can occupy no clusters at all. Reporting the logical
    // length here would erase exactly the saving this function exists to expose,
    // so the measured value is returned verbatim. Query failures are surfaced as
    // Err above and handled by the caller.
    Ok(allocated)
}

#[cfg(not(windows))]
pub(crate) fn allocated_size(_path: &Path, metadata: &Metadata) -> io::Result<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // st_blocks counts 512-byte units by POSIX definition. Zero is again a
        // valid answer for a sparse file and is reported as measured.
        return Ok(metadata.blocks().saturating_mul(512));
    }
    #[cfg(not(unix))]
    Ok(metadata.len())
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
