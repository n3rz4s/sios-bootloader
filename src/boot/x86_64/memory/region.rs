use crate::boot::x86_64::PhysicalAddress;

use super::error::MemoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalRange {
    start: PhysicalAddress,

    length: u64,
}

impl PhysicalRange {
    pub fn new(start: PhysicalAddress, length: u64) -> Result<Self, MemoryError> {
        if length == 0 {
            return Err(MemoryError::InvalidRange);
        }

        start
            .as_u64()
            .checked_add(length)
            .ok_or(MemoryError::AddressOverflow)?;

        Ok(Self { start, length })
    }

    pub const fn start(&self) -> PhysicalAddress {
        self.start
    }

    pub const fn length(&self) -> u64 {
        self.length
    }

    pub fn end_exclusive(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.start.as_u64() + self.length)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,

    Reserved,

    AcpiReclaimable,
    AcpiNvs,

    Unusable,

    Persistent,

    Firmware,

    Mmio,

    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    range: PhysicalRange,

    kind: MemoryRegionKind,
}

impl MemoryRegion {
    pub fn new(
        start: PhysicalAddress,
        length: u64,
        kind: MemoryRegionKind,
    ) -> Result<Self, MemoryError> {
        Ok(Self {
            range: PhysicalRange::new(start, length)?,

            kind,
        })
    }

    pub const fn start(&self) -> PhysicalAddress {
        self.range.start()
    }

    pub const fn length(&self) -> u64 {
        self.range.length()
    }

    pub fn end_exclusive(&self) -> PhysicalAddress {
        self.range.end_exclusive()
    }

    pub const fn kind(&self) -> MemoryRegionKind {
        self.kind
    }

    pub const fn range(&self) -> PhysicalRange {
        self.range
    }
}
