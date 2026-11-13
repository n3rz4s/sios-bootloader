use uefi::{Handle, Status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageStage {
    EnumerateFileSystems,
    EnumerateBlockDevices,

    OpenSimpleFileSystem,
    OpenRootDirectory,
    OpenBlockIo,

    OpenFile,
    ReadFile,
    ReadDirectory,

    ReadBlocks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageFailure {
    handle: Option<Handle>,
    stage: StorageStage,
    status: Status,
}

impl StorageFailure {
    pub const fn new(handle: Option<Handle>, stage: StorageStage, status: Status) -> Self {
        Self {
            handle,
            stage,
            status,
        }
    }

    pub const fn handle(&self) -> Option<Handle> {
        self.handle
    }

    pub const fn stage(&self) -> StorageStage {
        self.stage
    }

    pub const fn status(&self) -> Status {
        self.status
    }
}

pub type StorageResult<T> = Result<T, StorageFailure>;
