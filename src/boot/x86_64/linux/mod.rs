pub mod boot_params;
pub mod command_line;
pub mod error;
pub mod header;
pub mod image;
pub mod kernel_placement;
pub mod protocol;

pub use boot_params::{BootParams, BootParamsBuilder, BootParamsError, E820Entry, EfiBootInfo};

pub use kernel_placement::{KernelPlacement, KernelPlacementError};

pub use command_line::{KernelCommandLine, KernelCommandLineError};

pub use error::KernelImageError;

pub use header::{
    BOOT_FLAG, BootProtocolVersion, LOAD_HIGH, LinuxImageKind, SetupHeader,
    XLF_CAN_BE_LOADED_ABOVE_4G, XLF_EFI_HANDOVER_32, XLF_EFI_HANDOVER_64, XLF_KERNEL_64,
};

pub use image::{KernelPayloadFormat, LinuxKernelImage};

pub use protocol::{E820MemoryType, SecureBootMode};
