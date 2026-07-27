use std::{fs::File, fs::Metadata, io};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct FileIdentity {
    volume: u64,
    index: [u8; 16],
}

impl FileIdentity {
    pub(crate) fn same_volume(self, other: Self) -> bool {
        self.volume == other.volume
    }
}

#[cfg(windows)]
pub(crate) fn file_identity_from_file(
    file: &File,
    _metadata: &Metadata,
) -> io::Result<FileIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{FileIdInfo, GetFileInformationByHandleEx, FILE_ID_INFO},
    };

    let mut information = FILE_ID_INFO::default();
    let handle = HANDLE(file.as_raw_handle());
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            &mut information as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(FileIdentity {
        volume: information.VolumeSerialNumber,
        index: information.FileId.Identifier,
    })
}

#[cfg(unix)]
pub(crate) fn file_identity_from_file(
    _file: &File,
    metadata: &Metadata,
) -> io::Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt;

    let mut index = [0u8; 16];
    index[..8].copy_from_slice(&metadata.ino().to_le_bytes());
    Ok(FileIdentity {
        volume: metadata.dev(),
        index,
    })
}

#[cfg(not(any(windows, unix)))]
pub(crate) fn file_identity_from_file(
    _file: &File,
    _metadata: &Metadata,
) -> io::Result<FileIdentity> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "stable file identity is unsupported on this platform",
    ))
}

#[cfg(windows)]
pub(crate) fn hard_link_count_from_file(file: &File, _metadata: &Metadata) -> io::Result<u64> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{FileStandardInfo, GetFileInformationByHandleEx, FILE_STANDARD_INFO},
    };

    let mut information = FILE_STANDARD_INFO::default();
    let handle = HANDLE(file.as_raw_handle());
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileStandardInfo,
            &mut information as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
        )
    }
    .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(u64::from(information.NumberOfLinks))
}

#[cfg(unix)]
pub(crate) fn hard_link_count_from_file(_file: &File, metadata: &Metadata) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;

    Ok(metadata.nlink())
}

#[cfg(not(any(windows, unix)))]
pub(crate) fn hard_link_count_from_file(_file: &File, _metadata: &Metadata) -> io::Result<u64> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "hard-link count is unsupported on this platform",
    ))
}
