pub mod error;
pub mod header;
pub mod image;

pub use error::KernelImageError;

pub use header::{
    BOOT_FLAG, BootProtocolVersion, LOAD_HIGH, LinuxImageKind, SetupHeader,
    XLF_CAN_BE_LOADED_ABOVE_4G, XLF_EFI_HANDOVER_32, XLF_EFI_HANDOVER_64, XLF_KERNEL_64,
};

pub use image::{KernelPayloadFormat, LinuxKernelImage};
