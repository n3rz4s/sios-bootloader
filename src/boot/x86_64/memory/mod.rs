pub mod allocator;
pub mod error;
pub mod region;

pub use self::allocator::{
    AllocationRequest, AllocationStrategy, PageAllocation, PhysicalPageAllocator,
};

pub use self::error::MemoryError;

pub use self::region::{MemoryRegion, MemoryRegionKind, PhysicalRange};

pub const PAGE_SIZE: u64 = 4096;
