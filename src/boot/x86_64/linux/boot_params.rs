use crate::boot::x86_64::PhysicalAddress;

use super::{
    error::KernelImageError,
    header::{LinuxImageKind, SetupHeader},
    protocol::*,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootParamsError {
    KernelImage(KernelImageError),

    HeaderMismatch,

    InvalidHeaderLength,

    UnsupportedProtocol,

    NotBzImage,

    Not64BitKernel,

    InvalidPhysicalAddress,

    AddressAbove4GUnsupported,

    InvalidInitrdRange,

    InitrdAddressAboveMaximum,

    UnsupportedSetupData,

    InvalidE820Entry,

    TooManyE820Entries,

    E820EntriesNotSorted,

    E820EntriesOverlap,

    InvalidEfiInfo,

    MissingCommandLineAddress,

    MissingMemoryMap,

    InvalidSentinel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct E820Entry {
    base: u64,

    length: u64,

    memory_type: E820MemoryType,
}

impl E820Entry {
    pub fn new(
        base: u64,
        length: u64,
        memory_type: E820MemoryType,
    ) -> Result<Self, BootParamsError> {
        if length == 0 {
            return Err(BootParamsError::InvalidE820Entry);
        }

        if base.checked_add(length).is_none() {
            return Err(BootParamsError::InvalidE820Entry);
        }

        Ok(Self {
            base,
            length,
            memory_type,
        })
    }

    pub const fn base(&self) -> u64 {
        self.base
    }

    pub const fn length(&self) -> u64 {
        self.length
    }

    pub const fn memory_type(&self) -> E820MemoryType {
        self.memory_type
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EfiBootInfo {
    loader_signature: u32,

    system_table: PhysicalAddress,

    memory_map: PhysicalAddress,

    memory_map_size: u32,

    memory_descriptor_size: u32,

    memory_descriptor_version: u32,
}

impl EfiBootInfo {
    pub const fn new(
        loader_signature: u32,
        system_table: PhysicalAddress,
        memory_map: PhysicalAddress,
        memory_map_size: u32,
        memory_descriptor_size: u32,
        memory_descriptor_version: u32,
    ) -> Self {
        Self {
            loader_signature,
            system_table,
            memory_map,
            memory_map_size,
            memory_descriptor_size,
            memory_descriptor_version,
        }
    }
}

#[repr(C, align(4096))]
#[derive(Clone)]
pub struct BootParams {
    bytes: [u8; BOOT_PARAMS_SIZE],
}

impl BootParams {
    pub const SIZE: usize = BOOT_PARAMS_SIZE;

    pub fn as_bytes(&self) -> &[u8; BOOT_PARAMS_SIZE] {
        &self.bytes
    }

    pub fn loader_id(&self) -> u8 {
        self.bytes[TYPE_OF_LOADER_OFFSET]
    }

    pub fn sentinel(&self) -> u8 {
        self.bytes[SENTINEL_OFFSET]
    }

    pub fn e820_entry_count(&self) -> u8 {
        self.bytes[E820_COUNT_OFFSET]
    }

    pub fn command_line_address(&self) -> PhysicalAddress {
        let low = read_u32(&self.bytes, CMD_LINE_PTR_OFFSET) as u64;

        let high = read_u32(&self.bytes, EXT_CMD_LINE_PTR_OFFSET) as u64;

        PhysicalAddress::new((high << 32) | low)
    }

    pub fn initrd_address(&self) -> PhysicalAddress {
        let low = read_u32(&self.bytes, RAMDISK_IMAGE_OFFSET) as u64;

        let high = read_u32(&self.bytes, EXT_RAMDISK_IMAGE_OFFSET) as u64;

        PhysicalAddress::new((high << 32) | low)
    }

    pub fn initrd_size(&self) -> u64 {
        let low = read_u32(&self.bytes, RAMDISK_SIZE_OFFSET) as u64;

        let high = read_u32(&self.bytes, EXT_RAMDISK_SIZE_OFFSET) as u64;

        (high << 32) | low
    }

    pub fn protocol_version(&self) -> u16 {
        read_u16(&self.bytes, 0x206)
    }

    pub fn boot_signature(&self) -> u16 {
        read_u16(&self.bytes, 0x1FE)
    }

    pub fn validate_for_handoff(&self) -> Result<(), BootParamsError> {
        if self.sentinel() != 0 {
            return Err(BootParamsError::InvalidSentinel);
        }

        if self.boot_signature() != 0xAA55 {
            return Err(BootParamsError::HeaderMismatch);
        }

        if self.command_line_address().is_null() {
            return Err(BootParamsError::MissingCommandLineAddress);
        }

        if self.e820_entry_count() == 0 {
            return Err(BootParamsError::MissingMemoryMap);
        }

        if self.initrd_size() != 0 && self.initrd_address().is_null() {
            return Err(BootParamsError::InvalidInitrdRange);
        }

        Ok(())
    }

    pub fn is_handoff_ready(&self) -> bool {
        self.validate_for_handoff().is_ok()
    }
}

pub struct BootParamsBuilder {
    params: BootParams,

    header: SetupHeader,
}

impl BootParamsBuilder {
    pub fn new(kernel_image: &[u8], header: &SetupHeader) -> Result<Self, BootParamsError> {
        let parsed_header =
            SetupHeader::parse(kernel_image).map_err(BootParamsError::KernelImage)?;

        if parsed_header != *header {
            return Err(BootParamsError::HeaderMismatch);
        }

        if header.image_kind() != LinuxImageKind::BzImage {
            return Err(BootParamsError::NotBzImage);
        }

        if !header.protocol_version().at_least(2, 12) {
            return Err(BootParamsError::UnsupportedProtocol);
        }

        if !header.supports_64bit() {
            return Err(BootParamsError::Not64BitKernel);
        }

        let header_length = header.header_length();

        if header_length > SETUP_HEADER_MAX_SIZE {
            return Err(BootParamsError::InvalidHeaderLength);
        }

        let header_end = SETUP_HEADER_OFFSET
            .checked_add(header_length)
            .ok_or(BootParamsError::InvalidHeaderLength)?;

        if header_end > SETUP_HEADER_SLOT_END {
            return Err(BootParamsError::InvalidHeaderLength);
        }

        let source_header = kernel_image
            .get(SETUP_HEADER_OFFSET..header_end)
            .ok_or(BootParamsError::InvalidHeaderLength)?;

        let mut params = BootParams {
            bytes: [0; BOOT_PARAMS_SIZE],
        };

        params.bytes[SETUP_HEADER_OFFSET..header_end].copy_from_slice(source_header);

        params.write_u8(TYPE_OF_LOADER_OFFSET, UNDEFINED_BOOTLOADER_ID);

        params.write_u16(VIDEO_MODE_OFFSET, VGA_MODE_NORMAL);

        params.write_u64_split(RAMDISK_IMAGE_OFFSET, EXT_RAMDISK_IMAGE_OFFSET, 0);

        params.write_u64_split(RAMDISK_SIZE_OFFSET, EXT_RAMDISK_SIZE_OFFSET, 0);

        params.write_u64_split(CMD_LINE_PTR_OFFSET, EXT_CMD_LINE_PTR_OFFSET, 0);

        params.write_u64(SETUP_DATA_OFFSET, 0);

        params.write_u8(E820_COUNT_OFFSET, 0);

        params.bytes[SENTINEL_OFFSET] = 0;

        Ok(Self {
            params,
            header: header.clone(),
        })
    }

    pub fn set_loader_id(&mut self, id: u8) {
        self.params.write_u8(TYPE_OF_LOADER_OFFSET, id);
    }

    pub fn set_video_mode(&mut self, mode: u16) {
        self.params.write_u16(VIDEO_MODE_OFFSET, mode);
    }

    pub fn set_command_line_address(
        &mut self,
        address: PhysicalAddress,
    ) -> Result<(), BootParamsError> {
        if address.is_null() {
            return Err(BootParamsError::InvalidPhysicalAddress);
        }

        if address.is_above_4g() && !self.header.supports_above_4g() {
            return Err(BootParamsError::AddressAbove4GUnsupported);
        }

        self.params.write_u64_split(
            CMD_LINE_PTR_OFFSET,
            EXT_CMD_LINE_PTR_OFFSET,
            address.as_u64(),
        );

        Ok(())
    }

    pub fn set_initrd(
        &mut self,
        address: PhysicalAddress,
        size: u64,
    ) -> Result<(), BootParamsError> {
        if size == 0 {
            if !address.is_null() {
                return Err(BootParamsError::InvalidInitrdRange);
            }

            self.params
                .write_u64_split(RAMDISK_IMAGE_OFFSET, EXT_RAMDISK_IMAGE_OFFSET, 0);

            self.params
                .write_u64_split(RAMDISK_SIZE_OFFSET, EXT_RAMDISK_SIZE_OFFSET, 0);

            return Ok(());
        }

        if address.is_null() {
            return Err(BootParamsError::InvalidInitrdRange);
        }

        let end = address
            .as_u64()
            .checked_add(size)
            .and_then(|value| value.checked_sub(1))
            .ok_or(BootParamsError::InvalidInitrdRange)?;

        if end > u32::MAX as u64 && !self.header.supports_above_4g() {
            return Err(BootParamsError::AddressAbove4GUnsupported);
        }

        if end <= u32::MAX as u64 {
            if let Some(maximum) = self.header.initrd_addr_max() {
                if end > maximum as u64 {
                    return Err(BootParamsError::InitrdAddressAboveMaximum);
                }
            }
        }

        self.params.write_u64_split(
            RAMDISK_IMAGE_OFFSET,
            EXT_RAMDISK_IMAGE_OFFSET,
            address.as_u64(),
        );

        self.params
            .write_u64_split(RAMDISK_SIZE_OFFSET, EXT_RAMDISK_SIZE_OFFSET, size);

        Ok(())
    }

    pub fn set_setup_data_address(
        &mut self,
        address: PhysicalAddress,
    ) -> Result<(), BootParamsError> {
        if !self.header.protocol_version().at_least(2, 9) {
            return Err(BootParamsError::UnsupportedSetupData);
        }

        self.params.write_u64(SETUP_DATA_OFFSET, address.as_u64());

        Ok(())
    }

    pub fn set_acpi_rsdp_address(&mut self, address: Option<PhysicalAddress>) {
        self.params.write_u64(
            ACPI_RSDP_OFFSET,
            address.unwrap_or(PhysicalAddress::NULL).as_u64(),
        );
    }

    pub fn set_screen_info(&mut self, screen_info: &[u8; 64]) {
        self.params.bytes[0x000..0x040].copy_from_slice(screen_info);
    }

    pub fn set_e820_entries(&mut self, entries: &[E820Entry]) -> Result<(), BootParamsError> {
        if entries.len() > E820_MAX_ENTRIES {
            return Err(BootParamsError::TooManyE820Entries);
        }

        for index in 1..entries.len() {
            let previous = &entries[index - 1];

            let current = &entries[index];

            let previous_end = previous
                .base
                .checked_add(previous.length)
                .ok_or(BootParamsError::InvalidE820Entry)?;

            if current.base < previous.base {
                return Err(BootParamsError::E820EntriesNotSorted);
            }

            if current.base < previous_end {
                return Err(BootParamsError::E820EntriesOverlap);
            }
        }

        let table_end = E820_TABLE_OFFSET + E820_MAX_ENTRIES * E820_ENTRY_SIZE;

        self.params.bytes[E820_TABLE_OFFSET..table_end].fill(0);

        for (index, entry) in entries.iter().enumerate() {
            let offset = E820_TABLE_OFFSET + index * E820_ENTRY_SIZE;

            self.params.write_u64(offset, entry.base);

            self.params.write_u64(offset + 8, entry.length);

            self.params.write_u32(offset + 16, entry.memory_type as u32);
        }

        self.params.write_u8(E820_COUNT_OFFSET, entries.len() as u8);

        Ok(())
    }

    pub fn set_secure_boot_mode(&mut self, mode: SecureBootMode) {
        self.params.write_u8(SECURE_BOOT_OFFSET, mode as u8);
    }

    pub fn set_efi_info(&mut self, info: &EfiBootInfo) -> Result<(), BootParamsError> {
        if info.memory_descriptor_size == 0 {
            return Err(BootParamsError::InvalidEfiInfo);
        }

        if info.memory_map_size != 0 && info.memory_map.is_null() {
            return Err(BootParamsError::InvalidEfiInfo);
        }

        let offset = EFI_INFO_OFFSET;

        self.params.write_u32(offset, info.loader_signature);

        self.params
            .write_u64_split(offset + 4, offset + 24, info.system_table.as_u64());

        self.params
            .write_u32(offset + 8, info.memory_descriptor_size);

        self.params
            .write_u32(offset + 12, info.memory_descriptor_version);

        self.params
            .write_u64_split(offset + 16, offset + 28, info.memory_map.as_u64());

        self.params.write_u32(offset + 20, info.memory_map_size);

        Ok(())
    }

    pub fn build(self) -> BootParams {
        self.params
    }

    pub fn header(&self) -> &SetupHeader {
        &self.header
    }
}

impl BootParams {
    fn write_u8(&mut self, offset: usize, value: u8) {
        self.bytes[offset] = value;
    }

    fn write_u16(&mut self, offset: usize, value: u16) {
        self.bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, offset: usize, value: u32) {
        self.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(&mut self, offset: usize, value: u64) {
        self.bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64_split(&mut self, low_offset: usize, high_offset: usize, value: u64) {
        self.write_u32(low_offset, value as u32);

        self.write_u32(high_offset, (value >> 32) as u32);
    }
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    use super::super::{XLF_CAN_BE_LOADED_ABOVE_4G, XLF_KERNEL_64};

    fn put_u16(image: &mut [u8], offset: usize, value: u16) {
        image[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(image: &mut [u8], offset: usize, value: u32) {
        image[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(image: &mut [u8], offset: usize, value: u64) {
        image[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn make_kernel() -> alloc::vec::Vec<u8> {
        let mut image = vec![0u8; 4096];

        image[0x1F1] = 1;

        put_u32(&mut image, 0x1F4, 64);

        put_u16(&mut image, 0x1FE, 0xAA55);

        image[0x200] = 0xEB;
        image[0x201] = 0x6A;

        image[0x202..0x206].copy_from_slice(b"HdrS");

        put_u16(&mut image, 0x206, 0x020F);

        image[0x211] = 1;

        put_u32(&mut image, 0x22C, u32::MAX);

        put_u32(&mut image, 0x230, 0x200000);

        image[0x234] = 1;
        image[0x235] = 21;

        put_u16(
            &mut image,
            0x236,
            XLF_KERNEL_64 | XLF_CAN_BE_LOADED_ABOVE_4G,
        );

        put_u32(&mut image, 0x238, 2047);

        put_u32(&mut image, 0x248, 0);
        put_u32(&mut image, 0x24C, 0);

        put_u64(&mut image, 0x258, 0x2000000);

        put_u32(&mut image, 0x260, 0x400000);

        image
    }

    #[test]
    fn zero_page_has_correct_size_and_header() {
        let image = make_kernel();

        let header = SetupHeader::parse(&image).unwrap();

        let params = BootParamsBuilder::new(&image, &header).unwrap().build();

        assert_eq!(params.as_bytes().len(), BOOT_PARAMS_SIZE,);

        assert_eq!(params.boot_signature(), 0xAA55,);

        assert_eq!(params.protocol_version(), 0x020F,);

        assert_eq!(params.loader_id(), 0xFF,);

        assert_eq!(params.sentinel(), 0,);
    }

    #[test]
    fn extended_addresses_are_serialized_correctly() {
        let image = make_kernel();

        let header = SetupHeader::parse(&image).unwrap();

        let mut builder = BootParamsBuilder::new(&image, &header).unwrap();

        let command_line_address = PhysicalAddress::new(0x1_0000_2000);

        let initrd_address = PhysicalAddress::new(0x1_1000_0000);

        builder
            .set_command_line_address(command_line_address)
            .unwrap();

        builder.set_initrd(initrd_address, 0x200000).unwrap();

        let params = builder.build();

        assert_eq!(params.command_line_address(), command_line_address,);

        assert_eq!(params.initrd_address(), initrd_address,);

        assert_eq!(params.initrd_size(), 0x200000,);
    }

    #[test]
    fn e820_table_is_serialized() {
        let image = make_kernel();

        let header = SetupHeader::parse(&image).unwrap();

        let mut builder = BootParamsBuilder::new(&image, &header).unwrap();

        let entries = [
            E820Entry::new(0, 0x9F000, E820MemoryType::Ram).unwrap(),
            E820Entry::new(0x9F000, 0x61000, E820MemoryType::Reserved).unwrap(),
        ];

        builder.set_e820_entries(&entries).unwrap();

        let params = builder.build();

        assert_eq!(params.e820_entry_count(), 2,);

        assert_eq!(read_u64(params.as_bytes(), E820_TABLE_OFFSET,), 0,);

        assert_eq!(read_u64(params.as_bytes(), E820_TABLE_OFFSET + 8,), 0x9F000,);
    }

    #[test]
    fn overlapping_e820_entries_are_rejected() {
        let image = make_kernel();

        let header = SetupHeader::parse(&image).unwrap();

        let mut builder = BootParamsBuilder::new(&image, &header).unwrap();

        let entries = [
            E820Entry::new(0x1000, 0x5000, E820MemoryType::Ram).unwrap(),
            E820Entry::new(0x4000, 0x3000, E820MemoryType::Reserved).unwrap(),
        ];

        assert_eq!(
            builder.set_e820_entries(&entries),
            Err(BootParamsError::E820EntriesOverlap),
        );
    }
}
