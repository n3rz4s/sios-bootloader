use alloc::vec::Vec;

use super::header::SetupHeader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelCommandLineError {
    ContainsNul,

    TooLong,

    AllocationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelCommandLine {
    bytes: Vec<u8>,
}

impl KernelCommandLine {
    pub fn new(value: &str, header: &SetupHeader) -> Result<Self, KernelCommandLineError> {
        if value.contains('\0') {
            return Err(KernelCommandLineError::ContainsNul);
        }

        let maximum = header.cmdline_size().unwrap_or(255) as usize;

        if value.len() > maximum {
            return Err(KernelCommandLineError::TooLong);
        }

        let capacity = value
            .len()
            .checked_add(1)
            .ok_or(KernelCommandLineError::TooLong)?;

        let mut bytes = Vec::new();

        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| KernelCommandLineError::AllocationFailed)?;

        bytes.extend_from_slice(value.as_bytes());

        bytes.push(0);

        Ok(Self { bytes })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.bytes.len() - 1]
    }

    pub fn as_bytes_with_nul(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
