use uefi::Status;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EfiLoadStage {
    ResolveVolume,
    BuildDevicePath,
    LoadImage,
    OpenLoadedImage,
    SetLoadOptions,
    StartImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EfiLoadError {
    stage: EfiLoadStage,
    status: Status,
}

impl EfiLoadError {
    pub(crate) const fn new(stage: EfiLoadStage, status: Status) -> Self {
        Self { stage, status }
    }

    pub const fn stage(&self) -> EfiLoadStage {
        self.stage
    }

    pub const fn status(&self) -> Status {
        self.status
    }
}

pub type EfiLoadResult<T> = Result<T, EfiLoadError>;
