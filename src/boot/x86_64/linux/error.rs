#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelImageError {
    ImageTooSmall,

    MissingHeaderMagic,

    UnsupportedProtocol,

    InvalidHeaderLength,
    InvalidSetupSize,
    InvalidKernelSize,
    InvalidBootSignature,
    InvalidKernelAlignment,
    InvalidPayload,
    InvalidKernelInfo,

    TruncatedImage,

    NotBzImage,
    Not64BitKernel,

    MissingRequiredField,
}
