use alloc::{string::String, vec::Vec};

use super::{
    entry::{BootEntry, BootEntryId, BootPath, BootTarget, EfiTarget, LinuxNativeTarget},
    error::{BootError, BootResult},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootConfig {
    entries: Vec<BootEntry>,

    default_entry: Option<BootEntryId>,

    timeout_seconds: u16,
}

impl BootConfig {
    pub fn new(
        entries: Vec<BootEntry>,
        default_entry: Option<BootEntryId>,
        timeout_seconds: u16,
    ) -> BootResult<Self> {
        for index in 0..entries.len() {
            for other in index + 1..entries.len() {
                if entries[index].id() == entries[other].id() {
                    return Err(BootError::DuplicateEntryId);
                }
            }
        }

        if let Some(default_id) = default_entry {
            let default = entries
                .iter()
                .find(|entry| entry.id() == default_id)
                .ok_or(BootError::DefaultEntryNotFound)?;

            if !default.is_enabled() {
                return Err(BootError::DefaultEntryDisabled);
            }
        }

        Ok(Self {
            entries,
            default_entry,
            timeout_seconds,
        })
    }

    pub fn development() -> BootResult<Self> {
        let mut entries = Vec::with_capacity(4);

        entries.push(BootEntry::new(
            BootEntryId::new(1),
            "SIOS Linux",
            BootTarget::EfiChainload(EfiTarget::new(
                BootPath::new("\\EFI\\BOOT\\BOOTX64.EFI")?,
                None,
            )?),
        )?);

        let mut initrds = Vec::new();

        initrds.push(BootPath::new("\\initramfs-linux.img")?);

        entries.push(BootEntry::new(
            BootEntryId::new(2),
            "Arch Linux (native)",
            BootTarget::LinuxNative(LinuxNativeTarget::new(
                BootPath::new("\\vmlinuz-linux")?,
                initrds,
                String::from("rw"),
            )?),
        )?);

        entries.push(BootEntry::new(
            BootEntryId::new(3),
            "Arch Linux (EFI Stub)",
            BootTarget::LinuxEfiStub(EfiTarget::new(
                BootPath::new("\\EFI\\Linux\\arch-linux.efi")?,
                None,
            )?),
        )?);

        entries.push(BootEntry::new(
            BootEntryId::new(4),
            "Windows 10",
            BootTarget::EfiChainload(EfiTarget::new(
                BootPath::new("\\EFI\\Microsoft\\Boot\\bootmgfw.efi")?,
                None,
            )?),
        )?);

        Self::new(entries, Some(BootEntryId::new(1)), 5)
    }

    pub fn entries(&self) -> &[BootEntry] {
        &self.entries
    }

    pub const fn default_entry(&self) -> Option<BootEntryId> {
        self.default_entry
    }

    pub const fn timeout_seconds(&self) -> u16 {
        self.timeout_seconds
    }
}
