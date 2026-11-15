use super::{
    error::KernelImageError,
    header::{LinuxImageKind, SetupHeader, XLF_KERNEL_64},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelPayloadFormat {
    Gzip,

    Bzip2,

    Lzma,

    Xz,

    Lz4,

    Zstd,

    Elf,

    Unknown,
}

impl KernelPayloadFormat {
    fn detect(data: &[u8]) -> Self {
        if data.starts_with(&[0x1F, 0x8B]) || data.starts_with(&[0x1F, 0x9E]) {
            Self::Gzip
        } else if data.starts_with(&[0x42, 0x5A]) {
            Self::Bzip2
        } else if data.starts_with(&[0x5D, 0x00]) {
            Self::Lzma
        } else if data.starts_with(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]) {
            Self::Xz
        } else if data.starts_with(&[0x02, 0x21]) {
            Self::Lz4
        } else if data.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
            Self::Zstd
        } else if data.starts_with(b"\x7FELF") {
            Self::Elf
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinuxKernelImage {
    file_size: usize,

    header: SetupHeader,

    payload_format: Option<KernelPayloadFormat>,
}

impl LinuxKernelImage {
    pub fn parse(data: &[u8]) -> Result<Self, KernelImageError> {
        let header = SetupHeader::parse(data)?;

        let payload_format = match header.payload_range() {
            Some((offset, length)) => {
                let end = offset
                    .checked_add(length)
                    .ok_or(KernelImageError::InvalidPayload)?;

                let payload = data
                    .get(offset..end)
                    .ok_or(KernelImageError::TruncatedImage)?;

                Some(KernelPayloadFormat::detect(payload))
            }

            None => None,
        };

        Ok(Self {
            file_size: data.len(),
            header,
            payload_format,
        })
    }

    pub fn validate_native_x86_64(&self) -> Result<(), KernelImageError> {
        if self.header.image_kind() != LinuxImageKind::BzImage {
            return Err(KernelImageError::NotBzImage);
        }

        if !self.header.protocol_version().at_least(2, 12) {
            return Err(KernelImageError::UnsupportedProtocol);
        }

        let flags = self
            .header
            .xload_flags()
            .ok_or(KernelImageError::MissingRequiredField)?;

        if flags & XLF_KERNEL_64 == 0 {
            return Err(KernelImageError::Not64BitKernel);
        }

        if self.header.syssize_bytes().is_none() {
            return Err(KernelImageError::MissingRequiredField);
        }

        if self.header.init_size().is_none_or(|size| size == 0) {
            return Err(KernelImageError::MissingRequiredField);
        }

        Ok(())
    }

    pub fn header(&self) -> &SetupHeader {
        &self.header
    }

    pub const fn file_size(&self) -> usize {
        self.file_size
    }

    pub const fn payload_format(&self) -> Option<KernelPayloadFormat> {
        self.payload_format
    }

    pub const fn is_bz_image(&self) -> bool {
        matches!(self.header.image_kind(), LinuxImageKind::BzImage)
    }

    pub fn is_64_bit(&self) -> bool {
        self.header
            .xload_flags()
            .is_some_and(|flags| flags & XLF_KERNEL_64 != 0)
    }

    pub fn supports_above_4g(&self) -> bool {
        self.header.supports_above_4g()
    }
}
