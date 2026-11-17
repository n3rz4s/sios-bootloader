#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    InvalidRange,

    AddressOverflow,

    InvalidMemoryMap,

    OverlappingRegions,

    NoUsableMemory,

    InvalidPageCount,
    InvalidAlignment,
    InvalidAddressWindow,

    NoSuitableMemory,

    OutOfMemory,

    ReservationOverlapsAllocation,

    UnknownAllocation,

    AllocationIdExhausted,
}
