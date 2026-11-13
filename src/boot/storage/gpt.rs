use alloc::{string::String, vec::Vec};

use super::{device::BlockDevice, error::StorageFailure};

const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";

const GPT_HEADER_MIN_SIZE: usize = 92;

const GPT_ENTRY_MIN_SIZE: usize = 128;

const GPT_MAX_ENTRIES: u32 = 16_384;

const GPT_MAX_ARRAY_SIZE: u64 = 16 * 1024 * 1024;

const EFI_SYSTEM_GUID: [u8; 16] = [
    0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b,
];

const MICROSOFT_BASIC_DATA_GUID: [u8; 16] = [
    0xa2, 0xa0, 0xd0, 0xeb, 0xe5, 0xb9, 0x33, 0x44, 0x87, 0xc0, 0x68, 0xb6, 0xb7, 0x26, 0x99, 0xc7,
];

const LINUX_FILESYSTEM_GUID: [u8; 16] = [
    0xaf, 0x3d, 0xc6, 0x0f, 0x83, 0x84, 0x72, 0x47, 0x8e, 0x79, 0x3d, 0x69, 0xd8, 0x47, 0x7d, 0xe4,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GptPartitionKind {
    EfiSystem,
    MicrosoftBasicData,
    LinuxFilesystem,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GptError {
    NotGpt,

    InvalidHeader,
    HeaderCrcMismatch,

    InvalidPartitionArray,
    PartitionArrayCrcMismatch,

    InvalidPartitionEntry,

    Io(StorageFailure),
}

#[derive(Debug, Clone)]
pub struct GptHeader {
    current_lba: u64,

    backup_lba: u64,

    first_usable_lba: u64,

    last_usable_lba: u64,

    disk_guid: [u8; 16],

    entries_lba: u64,

    entry_count: u32,

    entry_size: u32,
}

impl GptHeader {
    pub const fn current_lba(&self) -> u64 {
        self.current_lba
    }

    pub const fn backup_lba(&self) -> u64 {
        self.backup_lba
    }

    pub const fn first_usable_lba(&self) -> u64 {
        self.first_usable_lba
    }

    pub const fn last_usable_lba(&self) -> u64 {
        self.last_usable_lba
    }

    pub const fn disk_guid(&self) -> &[u8; 16] {
        &self.disk_guid
    }

    pub const fn entries_lba(&self) -> u64 {
        self.entries_lba
    }

    pub const fn entry_count(&self) -> u32 {
        self.entry_count
    }

    pub const fn entry_size(&self) -> u32 {
        self.entry_size
    }
}

#[derive(Debug, Clone)]
pub struct GptPartition {
    index: u32,

    type_guid: [u8; 16],

    unique_guid: [u8; 16],

    first_lba: u64,

    last_lba: u64,

    attributes: u64,

    name: String,
}

impl GptPartition {
    pub const fn index(&self) -> u32 {
        self.index
    }

    pub const fn type_guid(&self) -> &[u8; 16] {
        &self.type_guid
    }

    pub const fn unique_guid(&self) -> &[u8; 16] {
        &self.unique_guid
    }

    pub const fn first_lba(&self) -> u64 {
        self.first_lba
    }

    pub const fn last_lba(&self) -> u64 {
        self.last_lba
    }

    pub const fn attributes(&self) -> u64 {
        self.attributes
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn block_count(&self) -> Option<u64> {
        self.last_lba.checked_sub(self.first_lba)?.checked_add(1)
    }

    pub fn kind(&self) -> GptPartitionKind {
        if self.type_guid == EFI_SYSTEM_GUID {
            GptPartitionKind::EfiSystem
        } else if self.type_guid == MICROSOFT_BASIC_DATA_GUID {
            GptPartitionKind::MicrosoftBasicData
        } else if self.type_guid == LINUX_FILESYSTEM_GUID {
            GptPartitionKind::LinuxFilesystem
        } else {
            GptPartitionKind::Other
        }
    }
}

#[derive(Debug, Clone)]
pub struct GptTable {
    header: GptHeader,

    partitions: Vec<GptPartition>,

    backup_header: bool,
}

impl GptTable {
    pub fn header(&self) -> &GptHeader {
        &self.header
    }

    pub fn partitions(&self) -> &[GptPartition] {
        &self.partitions
    }

    pub const fn uses_backup_header(&self) -> bool {
        self.backup_header
    }

    pub fn read(device: &BlockDevice) -> Result<Option<Self>, GptError> {
        if device.block_size() < 512 {
            return Ok(None);
        }

        let last_lba = device.last_block();

        if last_lba < 2 {
            return Ok(None);
        }

        let primary_error = match Self::read_copy(device, 1, false) {
            Ok(table) => return Ok(Some(table)),
            Err(error) => error,
        };

        let backup = Self::read_copy(device, last_lba, true);

        match backup {
            Ok(table) => Ok(Some(table)),

            Err(GptError::NotGpt) if primary_error == GptError::NotGpt => Ok(None),

            Err(GptError::NotGpt) => Err(primary_error),

            Err(error) => Err(error),
        }
    }

    fn read_copy(device: &BlockDevice, header_lba: u64, backup: bool) -> Result<Self, GptError> {
        let sector = device.read_blocks(header_lba, 1).map_err(GptError::Io)?;

        let header = parse_header(&sector, header_lba, device.last_block())?;

        if backup {
            if header.backup_lba != 1 {
                return Err(GptError::InvalidHeader);
            }
        } else if header.backup_lba != device.last_block() {
            return Err(GptError::InvalidHeader);
        }

        let block_size = device.block_size() as u64;

        let entry_bytes = (header.entry_count as u64)
            .checked_mul(header.entry_size as u64)
            .ok_or(GptError::InvalidPartitionArray)?;

        if entry_bytes == 0 || entry_bytes > GPT_MAX_ARRAY_SIZE {
            return Err(GptError::InvalidPartitionArray);
        }

        let entry_blocks = entry_bytes
            .checked_add(block_size - 1)
            .ok_or(GptError::InvalidPartitionArray)?
            / block_size;

        let total_blocks = device
            .last_block()
            .checked_add(1)
            .ok_or(GptError::InvalidPartitionArray)?;

        let entries_end = header
            .entries_lba
            .checked_add(entry_blocks)
            .ok_or(GptError::InvalidPartitionArray)?;

        if header.entries_lba == 0 || entries_end > total_blocks {
            return Err(GptError::InvalidPartitionArray);
        }

        let array_position_valid = if backup {
            header.entries_lba > header.last_usable_lba && entries_end <= header.current_lba
        } else {
            header.entries_lba > header.current_lba && entries_end == header.first_usable_lba
        };

        if !array_position_valid {
            return Err(GptError::InvalidPartitionArray);
        }

        let entries = device
            .read_blocks(
                header.entries_lba,
                usize::try_from(entry_blocks).map_err(|_| GptError::InvalidPartitionArray)?,
            )
            .map_err(GptError::Io)?;

        let entry_bytes =
            usize::try_from(entry_bytes).map_err(|_| GptError::InvalidPartitionArray)?;

        let entries = &entries[..entry_bytes];

        let expected_crc = read_u32(&sector, 88).ok_or(GptError::InvalidHeader)?;

        if crc32(entries) != expected_crc {
            return Err(GptError::PartitionArrayCrcMismatch);
        }

        let partitions = parse_partitions(entries, &header, device.last_block())?;

        Ok(Self {
            header,
            partitions,
            backup_header: backup,
        })
    }
}

fn parse_header(sector: &[u8], expected_lba: u64, last_lba: u64) -> Result<GptHeader, GptError> {
    if sector.len() < GPT_HEADER_MIN_SIZE || sector.get(..8) != Some(GPT_SIGNATURE.as_slice()) {
        return Err(GptError::NotGpt);
    }

    let header_size = read_u32(sector, 12).ok_or(GptError::InvalidHeader)? as usize;

    if !(GPT_HEADER_MIN_SIZE..=sector.len()).contains(&header_size) {
        return Err(GptError::InvalidHeader);
    }

    let stored_crc = read_u32(sector, 16).ok_or(GptError::InvalidHeader)?;

    if crc32_header(sector, header_size) != stored_crc {
        return Err(GptError::HeaderCrcMismatch);
    }

    let current_lba = read_u64(sector, 24).ok_or(GptError::InvalidHeader)?;

    let backup_lba = read_u64(sector, 32).ok_or(GptError::InvalidHeader)?;

    let first_usable_lba = read_u64(sector, 40).ok_or(GptError::InvalidHeader)?;

    let last_usable_lba = read_u64(sector, 48).ok_or(GptError::InvalidHeader)?;

    let disk_guid: [u8; 16] = sector
        .get(56..72)
        .ok_or(GptError::InvalidHeader)?
        .try_into()
        .map_err(|_| GptError::InvalidHeader)?;

    let entries_lba = read_u64(sector, 72).ok_or(GptError::InvalidHeader)?;

    let entry_count = read_u32(sector, 80).ok_or(GptError::InvalidHeader)?;

    let entry_size = read_u32(sector, 84).ok_or(GptError::InvalidHeader)?;

    if current_lba != expected_lba
        || backup_lba > last_lba
        || backup_lba == current_lba
        || first_usable_lba > last_usable_lba
        || last_usable_lba > last_lba
        || entry_count == 0
        || entry_count > GPT_MAX_ENTRIES
        || entry_size < GPT_ENTRY_MIN_SIZE as u32
        || entry_size % 8 != 0
    {
        return Err(GptError::InvalidHeader);
    }

    Ok(GptHeader {
        current_lba,
        backup_lba,
        first_usable_lba,
        last_usable_lba,
        disk_guid,
        entries_lba,
        entry_count,
        entry_size,
    })
}

fn parse_partitions(
    entries: &[u8],
    header: &GptHeader,
    disk_last_lba: u64,
) -> Result<Vec<GptPartition>, GptError> {
    let entry_size = header.entry_size as usize;

    let expected_size = (header.entry_count as usize)
        .checked_mul(entry_size)
        .ok_or(GptError::InvalidPartitionArray)?;

    if entries.len() < expected_size {
        return Err(GptError::InvalidPartitionArray);
    }

    let mut partitions = Vec::new();

    for index in 0..header.entry_count as usize {
        let offset = index
            .checked_mul(entry_size)
            .ok_or(GptError::InvalidPartitionArray)?;

        let entry = entries
            .get(offset..offset + entry_size)
            .ok_or(GptError::InvalidPartitionArray)?;

        let type_guid: [u8; 16] = entry
            .get(0..16)
            .ok_or(GptError::InvalidPartitionEntry)?
            .try_into()
            .map_err(|_| GptError::InvalidPartitionEntry)?;

        if type_guid.iter().all(|byte| *byte == 0) {
            continue;
        }

        let unique_guid: [u8; 16] = entry
            .get(16..32)
            .ok_or(GptError::InvalidPartitionEntry)?
            .try_into()
            .map_err(|_| GptError::InvalidPartitionEntry)?;

        let first_lba = read_u64(entry, 32).ok_or(GptError::InvalidPartitionEntry)?;

        let last_lba = read_u64(entry, 40).ok_or(GptError::InvalidPartitionEntry)?;

        let attributes = read_u64(entry, 48).ok_or(GptError::InvalidPartitionEntry)?;

        if first_lba > last_lba
            || first_lba < header.first_usable_lba
            || last_lba > header.last_usable_lba
            || last_lba > disk_last_lba
        {
            return Err(GptError::InvalidPartitionEntry);
        }

        let name_bytes = entry.get(56..128).ok_or(GptError::InvalidPartitionEntry)?;

        let mut name_utf16 = Vec::new();

        for pair in name_bytes.chunks_exact(2) {
            let unit = u16::from_le_bytes([pair[0], pair[1]]);

            if unit == 0 {
                break;
            }

            name_utf16.push(unit);
        }

        let name = String::from_utf16_lossy(&name_utf16);

        partitions.push(GptPartition {
            index: index as u32 + 1,
            type_guid,
            unique_guid,
            first_lba,
            last_lba,
            attributes,
            name,
        });
    }

    for i in 0..partitions.len() {
        for j in i + 1..partitions.len() {
            let a = &partitions[i];
            let b = &partitions[j];

            let overlaps = a.first_lba() <= b.last_lba() && b.first_lba() <= a.last_lba();

            if overlaps {
                return Err(GptError::InvalidPartitionEntry);
            }
        }
    }

    Ok(partitions)
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        data.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;

    for byte in data {
        crc ^= *byte as u32;

        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);

            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }

    !crc
}

fn crc32_header(header: &[u8], header_size: usize) -> u32 {
    let mut crc = !0u32;

    for (index, byte) in header[..header_size].iter().enumerate() {
        let byte = if (16..20).contains(&index) { 0 } else { *byte };

        crc ^= byte as u32;

        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);

            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }

    !crc
}
