pub const MIN_NATIVE_X86_64_PROTOCOL: u16 = 0x020C;

pub const LINUX_64BIT_ENTRY_OFFSET: u64 = 0x200;

pub const BOOT_PARAMS_SIZE: usize = 4096;

pub const SETUP_HEADER_OFFSET: usize = 0x1F1;
pub const SETUP_HEADER_MAX_SIZE: usize = 144;
pub const SETUP_HEADER_SLOT_END: usize = 0x290;

pub const TYPE_OF_LOADER_OFFSET: usize = 0x210;

pub const VIDEO_MODE_OFFSET: usize = 0x1FA;

pub const RAMDISK_IMAGE_OFFSET: usize = 0x218;
pub const RAMDISK_SIZE_OFFSET: usize = 0x21C;

pub const CMD_LINE_PTR_OFFSET: usize = 0x228;

pub const CODE32_START_OFFSET: usize = 0x214;

pub const KERNEL_ALIGNMENT_OFFSET: usize = 0x230;

pub const INITRD_ADDR_MAX_OFFSET: usize = 0x22C;

pub const EXT_RAMDISK_IMAGE_OFFSET: usize = 0x0C0;
pub const EXT_RAMDISK_SIZE_OFFSET: usize = 0x0C4;
pub const EXT_CMD_LINE_PTR_OFFSET: usize = 0x0C8;

pub const SETUP_DATA_OFFSET: usize = 0x250;

pub const ACPI_RSDP_OFFSET: usize = 0x070;

pub const EFI_INFO_OFFSET: usize = 0x1C0;

pub const SECURE_BOOT_OFFSET: usize = 0x1EC;
pub const SENTINEL_OFFSET: usize = 0x1EF;

pub const E820_COUNT_OFFSET: usize = 0x1E8;
pub const E820_TABLE_OFFSET: usize = 0x2D0;
pub const E820_ENTRY_SIZE: usize = 20;
pub const E820_MAX_ENTRIES: usize = 128;

pub const UNDEFINED_BOOTLOADER_ID: u8 = 0xFF;

pub const VGA_MODE_NORMAL: u16 = 0xFFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum E820MemoryType {
    Ram = 1,

    Reserved = 2,

    AcpiReclaimable = 3,

    AcpiNvs = 4,

    Unusable = 5,

    PersistentMemory = 7,

    ReservedKernel = 128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SecureBootMode {
    Unset = 0,

    Unknown = 1,

    Disabled = 2,

    Enabled = 3,
}
