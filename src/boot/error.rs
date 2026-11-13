use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootError {
    EmptyEntryName,
    EntryNameTooLong,

    InvalidPath,
    PathTooLong,

    EmbeddedNul,

    DuplicateEntryId,

    DefaultEntryNotFound,
    DefaultEntryDisabled,

    NoEntries,
    NoBootableEntries,

    EntryNotFound,
    EntryDisabled,
}

pub type BootResult<T> = Result<T, BootError>;

impl fmt::Display for BootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyEntryName => "Boot entry name is empty",

            Self::EntryNameTooLong => "Boot entry name exceeds maximum length",

            Self::InvalidPath => "Invalid boot file path",

            Self::PathTooLong => "Boot file path exceeds maximum length",

            Self::EmbeddedNul => "String contains an embedded NUL byte",

            Self::DuplicateEntryId => "Duplicate boot entry ID",

            Self::DefaultEntryNotFound => "Default boot entry does not exist",

            Self::DefaultEntryDisabled => "Default boot entry is disabled",

            Self::NoEntries => "No boot entries available",

            Self::NoBootableEntries => "No enabled boot entries available",

            Self::EntryNotFound => "Boot entry not found",

            Self::EntryDisabled => "Boot entry is disabled",
        };

        f.write_str(message)
    }
}

impl core::error::Error for BootError {}
