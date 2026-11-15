use super::error::KernelImageError;

pub const BOOT_FLAG: u16 = 0xAA55;

pub const LOAD_HIGH: u8 = 1 << 0;

pub const XLF_KERNEL_64: u16 = 1 << 0;
pub const XLF_CAN_BE_LOADED_ABOVE_4G: u16 = 1 << 1;
pub const XLF_EFI_HANDOVER_32: u16 = 1 << 2;
pub const XLF_EFI_HANDOVER_64: u16 = 1 << 3;

const SETUP_HEADER_OFFSET: usize = 0x1F1;
const HEADER_OFFSET: usize = 0x202;
const BOOT_FLAG_OFFSET: usize = 0x1FE;
const MINIMUM_IMAGE_SIZE: usize = 0x208;
const SECTOR_SIZE: usize = 512;
const MAX_SETUP_SECTORS: u8 = 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootProtocolVersion {
    raw: u16,
}

impl BootProtocolVersion {
    pub const fn from_raw(raw: u16) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u16 {
        self.raw
    }

    pub const fn major(self) -> u8 {
        (self.raw >> 8) as u8
    }

    pub const fn minor(self) -> u8 {
        self.raw as u8
    }

    pub const fn at_least(self, major: u8, minor: u8) -> bool {
        self.raw >= (((major as u16) << 8) | minor as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxImageKind {
    ZImage,
    BzImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupHeader {
    protocol_version: BootProtocolVersion,

    image_kind: LinuxImageKind,

    setup_sects: u8,

    setup_size: usize,

    header_length: usize,

    syssize_bytes: Option<u64>,

    load_flags: u8,

    code32_start: u32,

    initrd_addr_max: Option<u32>,

    kernel_alignment: Option<u32>,

    relocatable_kernel: Option<bool>,

    min_alignment: Option<u8>,

    xload_flags: Option<u16>,

    cmdline_size: Option<u32>,

    payload_offset: Option<u32>,

    payload_length: Option<u32>,

    pref_address: Option<u64>,

    init_size: Option<u32>,

    handover_offset: Option<u32>,

    kernel_info_offset: Option<u32>,
}

impl SetupHeader {
    pub fn parse(image: &[u8]) -> Result<Self, KernelImageError> {
        if image.len() < MINIMUM_IMAGE_SIZE {
            return Err(KernelImageError::ImageTooSmall);
        }

        if read_u16(image, BOOT_FLAG_OFFSET) != Some(BOOT_FLAG) {
            return Err(KernelImageError::InvalidBootSignature);
        }

        if image.get(HEADER_OFFSET..HEADER_OFFSET + 4) != Some(b"HdrS".as_slice()) {
            return Err(KernelImageError::MissingHeaderMagic);
        }

        let version_raw =
            read_u16(image, HEADER_OFFSET + 4).ok_or(KernelImageError::ImageTooSmall)?;

        let protocol_version = BootProtocolVersion::from_raw(version_raw);

        if !protocol_version.at_least(2, 0) {
            return Err(KernelImageError::UnsupportedProtocol);
        }

        let jump_opcode = *image
            .get(0x200)
            .ok_or(KernelImageError::InvalidHeaderLength)?;

        if jump_opcode != 0xEB {
            return Err(KernelImageError::InvalidHeaderLength);
        }

        let jump_displacement = *image
            .get(0x201)
            .ok_or(KernelImageError::InvalidHeaderLength)? as i8;

        let header_end_signed = HEADER_OFFSET as isize + jump_displacement as isize;

        if header_end_signed < 0 {
            return Err(KernelImageError::InvalidHeaderLength);
        }

        let header_end = header_end_signed as usize;

        let header_length = header_end
            .checked_sub(SETUP_HEADER_OFFSET)
            .ok_or(KernelImageError::InvalidHeaderLength)?;

        if header_end < required_header_end(protocol_version)
            || header_end > image.len()
            || header_length > 144
        {
            return Err(KernelImageError::InvalidHeaderLength);
        }

        let setup_sects_raw = *image
            .get(SETUP_HEADER_OFFSET)
            .ok_or(KernelImageError::ImageTooSmall)?;

        let setup_sects = if setup_sects_raw == 0 {
            4
        } else {
            setup_sects_raw
        };

        if setup_sects > MAX_SETUP_SECTORS {
            return Err(KernelImageError::InvalidSetupSize);
        }

        let setup_size = (setup_sects as usize + 1)
            .checked_mul(SECTOR_SIZE)
            .ok_or(KernelImageError::InvalidSetupSize)?;

        if setup_size > image.len() {
            return Err(KernelImageError::TruncatedImage);
        }

        let syssize_bytes = if protocol_version.at_least(2, 4) {
            let paragraphs = read_u32(image, 0x1F4).ok_or(KernelImageError::ImageTooSmall)?;

            let bytes = (paragraphs as u64)
                .checked_mul(16)
                .ok_or(KernelImageError::InvalidKernelSize)?;

            if bytes == 0 {
                return Err(KernelImageError::InvalidKernelSize);
            }

            let kernel_end = (setup_size as u64)
                .checked_add(bytes)
                .ok_or(KernelImageError::InvalidKernelSize)?;

            if kernel_end > image.len() as u64 {
                return Err(KernelImageError::TruncatedImage);
            }

            Some(bytes)
        } else {
            None
        };

        let load_flags = *image.get(0x211).ok_or(KernelImageError::ImageTooSmall)?;

        let image_kind = if load_flags & LOAD_HIGH != 0 {
            LinuxImageKind::BzImage
        } else {
            LinuxImageKind::ZImage
        };

        let code32_start = read_u32(image, 0x214).ok_or(KernelImageError::ImageTooSmall)?;

        let initrd_addr_max = if protocol_version.at_least(2, 3) {
            Some(read_u32(image, 0x22C).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        let kernel_alignment = if protocol_version.at_least(2, 5) {
            Some(read_u32(image, 0x230).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        let relocatable_kernel = if protocol_version.at_least(2, 5) {
            Some(
                *image
                    .get(0x234)
                    .ok_or(KernelImageError::InvalidHeaderLength)?
                    != 0,
            )
        } else {
            None
        };

        let min_alignment = if protocol_version.at_least(2, 10) {
            Some(
                *image
                    .get(0x235)
                    .ok_or(KernelImageError::InvalidHeaderLength)?,
            )
        } else {
            None
        };

        let xload_flags = if protocol_version.at_least(2, 12) {
            Some(read_u16(image, 0x236).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        let cmdline_size = if protocol_version.at_least(2, 6) {
            Some(read_u32(image, 0x238).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        let payload_offset = if protocol_version.at_least(2, 8) {
            Some(read_u32(image, 0x248).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        let payload_length = if protocol_version.at_least(2, 8) {
            Some(read_u32(image, 0x24C).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        let pref_address = if protocol_version.at_least(2, 10) {
            Some(read_u64(image, 0x258).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        let init_size = if protocol_version.at_least(2, 10) {
            Some(read_u32(image, 0x260).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        let handover_offset = if protocol_version.at_least(2, 11) {
            Some(read_u32(image, 0x264).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        let kernel_info_offset = if protocol_version.at_least(2, 15) {
            Some(read_u32(image, 0x268).ok_or(KernelImageError::InvalidHeaderLength)?)
        } else {
            None
        };

        if relocatable_kernel == Some(true) {
            let alignment = kernel_alignment.ok_or(KernelImageError::MissingRequiredField)?;

            if alignment == 0 || !alignment.is_power_of_two() {
                return Err(KernelImageError::InvalidKernelAlignment);
            }

            if let Some(minimum) = min_alignment {
                if minimum >= 32 {
                    return Err(KernelImageError::InvalidKernelAlignment);
                }

                let minimum_bytes = 1u32 << minimum;

                if alignment < minimum_bytes {
                    return Err(KernelImageError::InvalidKernelAlignment);
                }
            }
        }

        match (payload_offset, payload_length) {
            (Some(0), Some(0)) => {}

            (Some(offset), Some(length)) if offset > 0 && length > 0 => {
                let payload_end = (offset as u64)
                    .checked_add(length as u64)
                    .ok_or(KernelImageError::InvalidPayload)?;

                if let Some(protected_size) = syssize_bytes {
                    if payload_end > protected_size {
                        return Err(KernelImageError::InvalidPayload);
                    }
                }

                let absolute_end = (setup_size as u64)
                    .checked_add(payload_end)
                    .ok_or(KernelImageError::InvalidPayload)?;

                if absolute_end > image.len() as u64 {
                    return Err(KernelImageError::TruncatedImage);
                }
            }

            _ => {
                return Err(KernelImageError::InvalidPayload);
            }
        }

        Ok(Self {
            protocol_version,
            image_kind,
            setup_sects,
            setup_size,
            header_length,
            syssize_bytes,
            load_flags,
            code32_start,
            initrd_addr_max,
            kernel_alignment,
            relocatable_kernel,
            min_alignment,
            xload_flags,
            cmdline_size,
            payload_offset,
            payload_length,
            pref_address,
            init_size,
            handover_offset,
            kernel_info_offset,
        })
    }

    pub const fn protocol_version(&self) -> BootProtocolVersion {
        self.protocol_version
    }

    pub const fn image_kind(&self) -> LinuxImageKind {
        self.image_kind
    }

    pub const fn setup_sects(&self) -> u8 {
        self.setup_sects
    }

    pub const fn setup_size(&self) -> usize {
        self.setup_size
    }

    pub const fn header_length(&self) -> usize {
        self.header_length
    }

    pub const fn syssize_bytes(&self) -> Option<u64> {
        self.syssize_bytes
    }

    pub const fn load_flags(&self) -> u8 {
        self.load_flags
    }

    pub const fn code32_start(&self) -> u32 {
        self.code32_start
    }

    pub const fn initrd_addr_max(&self) -> Option<u32> {
        self.initrd_addr_max
    }

    pub const fn kernel_alignment(&self) -> Option<u32> {
        self.kernel_alignment
    }

    pub const fn is_relocatable(&self) -> Option<bool> {
        self.relocatable_kernel
    }

    pub const fn min_alignment(&self) -> Option<u8> {
        self.min_alignment
    }

    pub const fn xload_flags(&self) -> Option<u16> {
        self.xload_flags
    }

    pub const fn cmdline_size(&self) -> Option<u32> {
        self.cmdline_size
    }

    pub const fn payload_offset(&self) -> Option<u32> {
        self.payload_offset
    }

    pub const fn payload_length(&self) -> Option<u32> {
        self.payload_length
    }

    pub const fn pref_address(&self) -> Option<u64> {
        self.pref_address
    }

    pub const fn init_size(&self) -> Option<u32> {
        self.init_size
    }

    pub const fn handover_offset(&self) -> Option<u32> {
        self.handover_offset
    }

    pub const fn kernel_info_offset(&self) -> Option<u32> {
        self.kernel_info_offset
    }

    pub fn supports_64bit(&self) -> bool {
        self.xload_flags
            .is_some_and(|flags| flags & super::XLF_KERNEL_64 != 0)
    }

    pub fn supports_above_4g(&self) -> bool {
        self.xload_flags
            .is_some_and(|flags| flags & super::XLF_CAN_BE_LOADED_ABOVE_4G != 0)
    }

    pub fn payload_range(&self) -> Option<(usize, usize)> {
        let offset = self.payload_offset? as usize;
        let length = self.payload_length? as usize;

        if offset == 0 || length == 0 {
            return None;
        }

        let start = self.setup_size.checked_add(offset)?;

        Some((start, length))
    }
}

fn required_header_end(version: BootProtocolVersion) -> usize {
    if version.at_least(2, 15) {
        0x26C
    } else if version.at_least(2, 11) {
        0x268
    } else if version.at_least(2, 10) {
        0x264
    } else if version.at_least(2, 9) {
        0x258
    } else if version.at_least(2, 8) {
        0x250
    } else if version.at_least(2, 7) {
        0x248
    } else if version.at_least(2, 6) {
        0x23C
    } else if version.at_least(2, 5) {
        0x235
    } else if version.at_least(2, 3) {
        0x230
    } else if version.at_least(2, 2) {
        0x22C
    } else {
        0x218
    }
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        data.get(offset..offset.checked_add(8)?)?.try_into().ok()?,
    ))
}
