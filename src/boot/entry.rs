use alloc::{string::String, vec::Vec};

use super::error::{BootError, BootResult};

pub const MAX_BOOT_PATH_LEN: usize = 1024;

pub const MAX_BOOT_ENTRY_NAME_LEN: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BootEntryId(u32);

impl BootEntryId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootPath {
    path: String,
}

impl BootPath {
    pub fn new(path: impl Into<String>) -> BootResult<Self> {
        let path = path.into();

        if path.is_empty() {
            return Err(BootError::InvalidPath);
        }

        if path.len() > MAX_BOOT_PATH_LEN {
            return Err(BootError::PathTooLong);
        }

        if path.contains('\0') {
            return Err(BootError::EmbeddedNul);
        }

        if !path.starts_with('\\') && !path.starts_with('/') {
            return Err(BootError::InvalidPath);
        }

        Ok(Self { path })
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }

    pub fn into_string(self) -> String {
        self.path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxNativeTarget {
    kernel_path: BootPath,

    initrd_paths: Vec<BootPath>,

    command_line: String,
}

impl LinuxNativeTarget {
    pub fn new(
        kernel_path: BootPath,
        initrd_paths: Vec<BootPath>,
        command_line: String,
    ) -> BootResult<Self> {
        if command_line.contains('\0') {
            return Err(BootError::EmbeddedNul);
        }

        Ok(Self {
            kernel_path,
            initrd_paths,
            command_line,
        })
    }

    pub fn kernel_path(&self) -> &BootPath {
        &self.kernel_path
    }

    pub fn initrd_paths(&self) -> &[BootPath] {
        &self.initrd_paths
    }

    pub fn command_line(&self) -> &str {
        &self.command_line
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EfiTarget {
    image_path: BootPath,

    load_options: Option<String>,
}

impl EfiTarget {
    pub fn new(image_path: BootPath, load_options: Option<String>) -> BootResult<Self> {
        if load_options
            .as_deref()
            .is_some_and(|value| value.contains('\0'))
        {
            return Err(BootError::EmbeddedNul);
        }

        Ok(Self {
            image_path,
            load_options,
        })
    }

    pub fn image_path(&self) -> &BootPath {
        &self.image_path
    }

    pub fn load_options(&self) -> Option<&str> {
        self.load_options.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootEntryKind {
    LinuxNative,
    LinuxEfiStub,
    EfiChainload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootTarget {
    LinuxNative(LinuxNativeTarget),

    LinuxEfiStub(EfiTarget),

    EfiChainload(EfiTarget),
}

impl BootTarget {
    pub const fn kind(&self) -> BootEntryKind {
        match self {
            Self::LinuxNative(_) => BootEntryKind::LinuxNative,

            Self::LinuxEfiStub(_) => BootEntryKind::LinuxEfiStub,

            Self::EfiChainload(_) => BootEntryKind::EfiChainload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootEntry {
    id: BootEntryId,

    name: String,

    target: BootTarget,

    enabled: bool,
}

impl BootEntry {
    pub fn new(id: BootEntryId, name: impl Into<String>, target: BootTarget) -> BootResult<Self> {
        let name = name.into();

        if name.trim().is_empty() {
            return Err(BootError::EmptyEntryName);
        }

        if name.len() > MAX_BOOT_ENTRY_NAME_LEN {
            return Err(BootError::EntryNameTooLong);
        }

        if name.contains('\0') {
            return Err(BootError::EmbeddedNul);
        }

        Ok(Self {
            id,
            name,
            target,
            enabled: true,
        })
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub const fn id(&self) -> BootEntryId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn target(&self) -> &BootTarget {
        &self.target
    }

    pub const fn kind(&self) -> BootEntryKind {
        self.target.kind()
    }

    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}
