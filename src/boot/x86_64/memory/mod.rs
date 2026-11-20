pub mod allocator;
pub mod error;
pub mod region;

pub use self::allocator::{
    AllocationRequest, AllocationStrategy, PageAllocation, PhysicalPageAllocator,
};

pub use self::error::MemoryError;

pub use self::region::{MemoryRegion, MemoryRegionKind, PhysicalRange};

pub const PAGE_SIZE: u64 = 4096;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::boot::x86_64::PhysicalAddress;

    fn usable_memory() -> [MemoryRegion; 1] {
        [MemoryRegion::new(
            PhysicalAddress::new(0x1000),
            0x10000,
            MemoryRegionKind::Usable,
        )
        .unwrap()]
    }

    #[test]
    fn public_allocator_api_allocates_and_releases_pages() {
        let regions = usable_memory();

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let initial_free = allocator.free_pages();

        let allocation = allocator
            .allocate(
                AllocationRequest::new(3)
                    .with_min_address(PhysicalAddress::new(0x2000))
                    .with_max_address_exclusive(PhysicalAddress::new(0x9000))
                    .with_alignment(PAGE_SIZE),
            )
            .unwrap();

        assert_eq!(allocation.base_address().as_u64(), 0x2000,);

        assert_eq!(allocation.page_count(), 3,);

        assert_eq!(allocation.size_bytes(), PAGE_SIZE * 3,);

        assert_eq!(allocator.allocated_pages(), 3,);

        assert_eq!(allocator.free_pages(), initial_free - 3,);

        allocator.free(allocation).unwrap();

        assert_eq!(allocator.allocated_pages(), 0,);

        assert_eq!(allocator.free_pages(), initial_free,);
    }

    #[test]
    fn reservation_splits_free_ranges_and_blocks_reserved_pages() {
        let regions = usable_memory();

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let reserved = PhysicalRange::new(PhysicalAddress::new(0x5000), 0x2000).unwrap();

        assert_eq!(allocator.reserve_range(reserved).unwrap(), 2,);

        assert_eq!(allocator.free_pages(), 14,);

        assert_eq!(allocator.free_range_count(), 2,);

        let first = allocator.allocate(AllocationRequest::new(4)).unwrap();

        let second = allocator.allocate(AllocationRequest::new(2)).unwrap();

        assert_eq!(first.base_address().as_u64(), 0x1000,);

        assert_eq!(first.end_exclusive().as_u64(), 0x5000,);

        assert_eq!(second.base_address().as_u64(), 0x7000,);

        allocator.free(first).unwrap();
        allocator.free(second).unwrap();

        assert_eq!(allocator.free_pages(), 14,);

        assert_eq!(allocator.free_range_count(), 2,);
    }
}
