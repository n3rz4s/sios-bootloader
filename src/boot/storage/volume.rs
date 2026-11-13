use alloc::vec::Vec;

use uefi::{
    CString16, Handle, Status,
    boot::ScopedProtocol,
    proto::media::{
        file::{Directory, File, FileAttribute, FileHandle, FileInfo, FileMode, FileType},
        fs::SimpleFileSystem,
    },
};

use super::{
    device_path::DevicePathInfo,
    error::{StorageFailure, StorageResult, StorageStage},
    protocols::open_simple_file_system,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMetadata {
    size: u64,
    physical_size: u64,
    directory: bool,
}

impl FileMetadata {
    pub const fn size(&self) -> u64 {
        self.size
    }

    pub const fn physical_size(&self) -> u64 {
        self.physical_size
    }

    pub const fn is_directory(&self) -> bool {
        self.directory
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    name_utf16: Vec<u16>,
    size: u64,
    directory: bool,
}

impl DirectoryEntry {
    pub fn name_utf16(&self) -> &[u16] {
        &self.name_utf16
    }

    pub const fn size(&self) -> u64 {
        self.size
    }

    pub const fn is_directory(&self) -> bool {
        self.directory
    }

    pub fn name_equals_ascii(&self, expected: &str) -> bool {
        if self.name_utf16.len() != expected.len() {
            return false;
        }

        self.name_utf16
            .iter()
            .zip(expected.as_bytes())
            .all(|(actual, expected)| {
                *actual <= 0x7F
                    && (*actual as u8).to_ascii_lowercase() == expected.to_ascii_lowercase()
            })
    }
}

#[derive(Debug, Clone)]
pub struct DirectoryListing {
    entries: Vec<DirectoryEntry>,
    truncated: bool,
}

impl DirectoryListing {
    pub fn entries(&self) -> &[DirectoryEntry] {
        &self.entries
    }

    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub struct StorageVolume {
    handle: Handle,

    device_path: Option<DevicePathInfo>,

    root: Directory,

    _protocol: ScopedProtocol<SimpleFileSystem>,
}

impl StorageVolume {
    pub fn open(handle: Handle) -> StorageResult<Self> {
        let mut protocol = open_simple_file_system(handle).map_err(|error| {
            StorageFailure::new(
                Some(handle),
                StorageStage::OpenSimpleFileSystem,
                error.status(),
            )
        })?;

        let root = protocol.open_volume().map_err(|error| {
            StorageFailure::new(
                Some(handle),
                StorageStage::OpenRootDirectory,
                error.status(),
            )
        })?;

        let device_path = DevicePathInfo::discover(handle);

        Ok(Self {
            handle,
            device_path,
            root,
            _protocol: protocol,
        })
    }

    pub const fn handle(&self) -> Handle {
        self.handle
    }

    pub fn device_path(&self) -> Option<&DevicePathInfo> {
        self.device_path.as_ref()
    }

    pub fn root_mut(&mut self) -> &mut Directory {
        &mut self.root
    }

    fn make_path(&self, path: &str, stage: StorageStage) -> StorageResult<CString16> {
        let handle = Some(self.handle);

        if path.is_empty() || path.len() > 2048 || path.contains('\0') {
            return Err(StorageFailure::new(
                handle,
                stage,
                Status::INVALID_PARAMETER,
            ));
        }

        let mut normalized = alloc::string::String::new();

        normalized
            .try_reserve_exact(path.len() + 1)
            .map_err(|_| StorageFailure::new(handle, stage, Status::OUT_OF_RESOURCES))?;

        if !path.starts_with('\\') && !path.starts_with('/') {
            normalized.push('\\');
        }

        for character in path.chars() {
            if character == '/' {
                normalized.push('\\');
            } else {
                normalized.push(character);
            }
        }

        if normalized
            .split('\\')
            .any(|component| component == "." || component == "..")
        {
            return Err(StorageFailure::new(
                handle,
                stage,
                Status::INVALID_PARAMETER,
            ));
        }

        CString16::try_from(normalized.as_str())
            .map_err(|_| StorageFailure::new(handle, stage, Status::INVALID_PARAMETER))
    }

    fn open_path(&mut self, path: &str, stage: StorageStage) -> StorageResult<FileHandle> {
        let path = self.make_path(path, stage)?;

        self.root
            .open(path.as_ref(), FileMode::Read, FileAttribute::empty())
            .map_err(|error| StorageFailure::new(Some(self.handle), stage, error.status()))
    }

    pub fn metadata(&mut self, path: &str) -> StorageResult<FileMetadata> {
        let mut file = self.open_path(path, StorageStage::OpenFile)?;

        let info = file.get_boxed_info::<FileInfo>().map_err(|error| {
            StorageFailure::new(Some(self.handle), StorageStage::OpenFile, error.status())
        })?;

        Ok(FileMetadata {
            size: info.file_size(),
            physical_size: info.physical_size(),
            directory: info.is_directory(),
        })
    }

    pub fn file_exists(&mut self, path: &str) -> StorageResult<bool> {
        match self.open_path(path, StorageStage::OpenFile) {
            Ok(_) => Ok(true),

            Err(error) if error.status() == Status::NOT_FOUND => Ok(false),

            Err(error) => Err(error),
        }
    }

    pub fn read_file(&mut self, path: &str, max_size: usize) -> StorageResult<Vec<u8>> {
        let mut handle = self.open_path(path, StorageStage::OpenFile)?;

        let info = handle.get_boxed_info::<FileInfo>().map_err(|error| {
            StorageFailure::new(Some(self.handle), StorageStage::ReadFile, error.status())
        })?;

        if info.is_directory() {
            return Err(StorageFailure::new(
                Some(self.handle),
                StorageStage::ReadFile,
                Status::INVALID_PARAMETER,
            ));
        }

        let file_size = info.file_size();

        drop(info);

        if file_size > max_size as u64 {
            return Err(StorageFailure::new(
                Some(self.handle),
                StorageStage::ReadFile,
                Status::BAD_BUFFER_SIZE,
            ));
        }

        let file_size = usize::try_from(file_size).map_err(|_| {
            StorageFailure::new(
                Some(self.handle),
                StorageStage::ReadFile,
                Status::BAD_BUFFER_SIZE,
            )
        })?;

        let mut buffer = Vec::new();

        buffer.try_reserve_exact(file_size).map_err(|_| {
            StorageFailure::new(
                Some(self.handle),
                StorageStage::ReadFile,
                Status::OUT_OF_RESOURCES,
            )
        })?;

        buffer.resize(file_size, 0);

        let mut file = match handle.into_type().map_err(|error| {
            StorageFailure::new(Some(self.handle), StorageStage::ReadFile, error.status())
        })? {
            FileType::Regular(file) => file,

            FileType::Dir(_) => {
                return Err(StorageFailure::new(
                    Some(self.handle),
                    StorageStage::ReadFile,
                    Status::INVALID_PARAMETER,
                ));
            }
        };

        let mut offset = 0;

        while offset < file_size {
            let bytes_read = file.read(&mut buffer[offset..]).map_err(|error| {
                StorageFailure::new(Some(self.handle), StorageStage::ReadFile, error.status())
            })?;

            if bytes_read == 0 {
                return Err(StorageFailure::new(
                    Some(self.handle),
                    StorageStage::ReadFile,
                    Status::DEVICE_ERROR,
                ));
            }

            offset += bytes_read;
        }

        Ok(buffer)
    }

    pub fn read_directory(
        &mut self,
        path: &str,
        max_entries: usize,
    ) -> StorageResult<DirectoryListing> {
        let handle = self.open_path(path, StorageStage::ReadDirectory)?;

        let mut directory = match handle.into_type().map_err(|error| {
            StorageFailure::new(
                Some(self.handle),
                StorageStage::ReadDirectory,
                error.status(),
            )
        })? {
            FileType::Dir(directory) => directory,

            FileType::Regular(_) => {
                return Err(StorageFailure::new(
                    Some(self.handle),
                    StorageStage::ReadDirectory,
                    Status::INVALID_PARAMETER,
                ));
            }
        };

        let mut entries = Vec::new();

        while entries.len() < max_entries {
            let info = directory.read_entry_boxed().map_err(|error| {
                StorageFailure::new(
                    Some(self.handle),
                    StorageStage::ReadDirectory,
                    error.status(),
                )
            })?;

            let Some(info) = info else {
                return Ok(DirectoryListing {
                    entries,
                    truncated: false,
                });
            };

            let name_source = info.file_name().to_u16_slice();

            let mut name_utf16 = Vec::new();

            name_utf16
                .try_reserve_exact(name_source.len())
                .map_err(|_| {
                    StorageFailure::new(
                        Some(self.handle),
                        StorageStage::ReadDirectory,
                        Status::OUT_OF_RESOURCES,
                    )
                })?;

            name_utf16.extend_from_slice(name_source);

            entries.try_reserve(1).map_err(|_| {
                StorageFailure::new(
                    Some(self.handle),
                    StorageStage::ReadDirectory,
                    Status::OUT_OF_RESOURCES,
                )
            })?;

            entries.push(DirectoryEntry {
                name_utf16,
                size: info.file_size(),
                directory: info.is_directory(),
            });
        }

        let truncated = directory
            .read_entry_boxed()
            .map_err(|error| {
                StorageFailure::new(
                    Some(self.handle),
                    StorageStage::ReadDirectory,
                    error.status(),
                )
            })?
            .is_some();

        Ok(DirectoryListing { entries, truncated })
    }
}
