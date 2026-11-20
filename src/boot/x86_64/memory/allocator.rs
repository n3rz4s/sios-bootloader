use alloc::vec::Vec;

use core::sync::atomic::{AtomicU64, Ordering};

use crate::boot::x86_64::PhysicalAddress;

use super::{
    PAGE_SIZE,
    error::MemoryError,
    region::{MemoryRegion, MemoryRegionKind, PhysicalRange},
};

static NEXT_ALLOCATOR_ID: AtomicU64 = AtomicU64::new(1);

fn next_allocator_id() -> Result<u64, MemoryError> {
    loop {
        let current = NEXT_ALLOCATOR_ID.load(Ordering::Relaxed);

        if current == 0 || current == u64::MAX {
            return Err(MemoryError::AllocationIdExhausted);
        }

        if NEXT_ALLOCATOR_ID
            .compare_exchange_weak(current, current + 1, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return Ok(current);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    LowestAddress,

    HighestAddress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationRequest {
    page_count: u64,

    min_address: PhysicalAddress,

    max_address_exclusive: Option<PhysicalAddress>,

    alignment: u64,

    strategy: AllocationStrategy,
}

impl AllocationRequest {
    pub const fn new(page_count: u64) -> Self {
        Self {
            page_count,

            min_address: PhysicalAddress::new(PAGE_SIZE),

            max_address_exclusive: None,

            alignment: PAGE_SIZE,

            strategy: AllocationStrategy::LowestAddress,
        }
    }

    pub const fn with_min_address(mut self, address: PhysicalAddress) -> Self {
        self.min_address = address;
        self
    }

    pub const fn with_max_address_exclusive(mut self, address: PhysicalAddress) -> Self {
        self.max_address_exclusive = Some(address);
        self
    }

    pub const fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    pub const fn with_strategy(mut self, strategy: AllocationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub const fn page_count(&self) -> u64 {
        self.page_count
    }

    pub const fn alignment(&self) -> u64 {
        self.alignment
    }
}

#[derive(Debug)]
pub struct PageAllocation {
    allocator_id: u64,

    allocation_id: u64,

    base: PhysicalAddress,

    page_count: u64,
}

impl PageAllocation {
    pub const fn base_address(&self) -> PhysicalAddress {
        self.base
    }

    pub const fn page_count(&self) -> u64 {
        self.page_count
    }

    pub const fn size_bytes(&self) -> u64 {
        self.page_count * PAGE_SIZE
    }

    pub fn end_exclusive(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.base.as_u64() + self.size_bytes())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FreeRange {
    start: u64,

    end: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AllocationRecord {
    id: u64,

    start: u64,

    end: u64,

    page_count: u64,
}

#[derive(Debug, Clone, Copy)]
struct AllocationCandidate {
    range_index: usize,

    start: u64,

    end: u64,
}

pub struct PhysicalPageAllocator {
    allocator_id: u64,

    next_allocation_id: u64,

    free_ranges: Vec<FreeRange>,

    allocations: Vec<AllocationRecord>,
}

impl PhysicalPageAllocator {
    pub fn new(regions: &[MemoryRegion]) -> Result<Self, MemoryError> {
        if regions.is_empty() {
            return Err(MemoryError::InvalidMemoryMap);
        }

        let mut sorted = Vec::new();

        sorted
            .try_reserve_exact(regions.len())
            .map_err(|_| MemoryError::OutOfMemory)?;

        sorted.extend_from_slice(regions);

        sorted.sort_unstable_by_key(|region| region.start().as_u64());

        for pair in sorted.windows(2) {
            let previous_end = pair[0].end_exclusive().as_u64();

            let current_start = pair[1].start().as_u64();

            if current_start < previous_end {
                return Err(MemoryError::OverlappingRegions);
            }
        }

        let mut free_ranges: Vec<FreeRange> = Vec::new();

        free_ranges
            .try_reserve_exact(sorted.len())
            .map_err(|_| MemoryError::OutOfMemory)?;

        for region in &sorted {
            if region.kind() != MemoryRegionKind::Usable {
                continue;
            }

            let start =
                align_up(region.start().as_u64(), PAGE_SIZE).ok_or(MemoryError::AddressOverflow)?;

            let end = align_down(region.end_exclusive().as_u64(), PAGE_SIZE);

            if start >= end {
                continue;
            }

            if let Some(previous) = free_ranges.last_mut() {
                if previous.end == start {
                    previous.end = end;
                    continue;
                }
            }

            free_ranges.push(FreeRange { start, end });
        }

        if free_ranges.is_empty() {
            return Err(MemoryError::NoUsableMemory);
        }

        Ok(Self {
            allocator_id: next_allocator_id()?,

            next_allocation_id: 1,

            free_ranges,

            allocations: Vec::new(),
        })
    }

    pub fn allocate(&mut self, request: AllocationRequest) -> Result<PageAllocation, MemoryError> {
        if request.page_count == 0 {
            return Err(MemoryError::InvalidPageCount);
        }

        if request.alignment < PAGE_SIZE || !request.alignment.is_power_of_two() {
            return Err(MemoryError::InvalidAlignment);
        }

        if let Some(maximum) = request.max_address_exclusive {
            if maximum.as_u64() <= request.min_address.as_u64() {
                return Err(MemoryError::InvalidAddressWindow);
            }
        }

        let size = request
            .page_count
            .checked_mul(PAGE_SIZE)
            .ok_or(MemoryError::AddressOverflow)?;

        let candidate = match request.strategy {
            AllocationStrategy::LowestAddress => self.find_lowest_fit(&request, size),

            AllocationStrategy::HighestAddress => self.find_highest_fit(&request, size),
        }
        .ok_or(MemoryError::NoSuitableMemory)?;

        let allocation_id = self.next_allocation_id;

        let next_id = allocation_id
            .checked_add(1)
            .ok_or(MemoryError::AllocationIdExhausted)?;

        self.allocations
            .try_reserve(1)
            .map_err(|_| MemoryError::OutOfMemory)?;

        self.free_ranges
            .try_reserve(1)
            .map_err(|_| MemoryError::OutOfMemory)?;

        let original = self.free_ranges[candidate.range_index];

        self.free_ranges.remove(candidate.range_index);

        let mut insert_at = candidate.range_index;

        if original.start < candidate.start {
            self.free_ranges.insert(
                insert_at,
                FreeRange {
                    start: original.start,

                    end: candidate.start,
                },
            );

            insert_at += 1;
        }

        if candidate.end < original.end {
            self.free_ranges.insert(
                insert_at,
                FreeRange {
                    start: candidate.end,

                    end: original.end,
                },
            );
        }

        self.allocations.push(AllocationRecord {
            id: allocation_id,

            start: candidate.start,

            end: candidate.end,

            page_count: request.page_count,
        });

        self.next_allocation_id = next_id;

        Ok(PageAllocation {
            allocator_id: self.allocator_id,

            allocation_id,

            base: PhysicalAddress::new(candidate.start),

            page_count: request.page_count,
        })
    }

    pub fn free(&mut self, allocation: PageAllocation) -> Result<(), MemoryError> {
        if allocation.allocator_id != self.allocator_id {
            return Err(MemoryError::UnknownAllocation);
        }

        let record_index = self
            .allocations
            .iter()
            .position(|record| record.id == allocation.allocation_id)
            .ok_or(MemoryError::UnknownAllocation)?;

        let record = self.allocations[record_index];

        if record.start != allocation.base.as_u64() || record.page_count != allocation.page_count {
            return Err(MemoryError::UnknownAllocation);
        }

        self.free_ranges
            .try_reserve(1)
            .map_err(|_| MemoryError::OutOfMemory)?;

        let insert_at = self
            .free_ranges
            .iter()
            .position(|range| range.start >= record.start)
            .unwrap_or(self.free_ranges.len());

        if insert_at > 0 && self.free_ranges[insert_at - 1].end > record.start {
            return Err(MemoryError::InvalidMemoryMap);
        }

        if insert_at < self.free_ranges.len() && record.end > self.free_ranges[insert_at].start {
            return Err(MemoryError::InvalidMemoryMap);
        }

        self.allocations.remove(record_index);

        self.free_ranges.insert(
            insert_at,
            FreeRange {
                start: record.start,

                end: record.end,
            },
        );

        self.merge_adjacent_ranges();

        Ok(())
    }

    pub fn reserve_range(&mut self, range: PhysicalRange) -> Result<u64, MemoryError> {
        let reserve_start = align_down(range.start().as_u64(), PAGE_SIZE);

        let reserve_end = align_up(range.end_exclusive().as_u64(), PAGE_SIZE)
            .ok_or(MemoryError::AddressOverflow)?;

        for allocation in &self.allocations {
            if ranges_overlap(reserve_start, reserve_end, allocation.start, allocation.end) {
                return Err(MemoryError::ReservationOverlapsAllocation);
            }
        }

        let mut replacement = Vec::new();

        replacement
            .try_reserve(self.free_ranges.len().saturating_add(1))
            .map_err(|_| MemoryError::OutOfMemory)?;

        let mut removed_bytes = 0u64;

        for free in &self.free_ranges {
            if !ranges_overlap(reserve_start, reserve_end, free.start, free.end) {
                replacement.push(*free);

                continue;
            }

            let overlap_start = free.start.max(reserve_start);

            let overlap_end = free.end.min(reserve_end);

            removed_bytes = removed_bytes
                .checked_add(overlap_end - overlap_start)
                .ok_or(MemoryError::AddressOverflow)?;

            if free.start < overlap_start {
                replacement.push(FreeRange {
                    start: free.start,

                    end: overlap_start,
                });
            }

            if overlap_end < free.end {
                replacement.push(FreeRange {
                    start: overlap_end,

                    end: free.end,
                });
            }
        }

        self.free_ranges = replacement;

        Ok(removed_bytes / PAGE_SIZE)
    }

    pub fn free_pages(&self) -> u64 {
        self.free_ranges
            .iter()
            .map(|range| (range.end - range.start) / PAGE_SIZE)
            .sum()
    }

    pub fn allocated_pages(&self) -> u64 {
        self.allocations
            .iter()
            .map(|allocation| allocation.page_count)
            .sum()
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }

    pub fn free_range_count(&self) -> usize {
        self.free_ranges.len()
    }

    pub fn largest_free_run_pages(&self) -> u64 {
        self.free_ranges
            .iter()
            .map(|range| (range.end - range.start) / PAGE_SIZE)
            .max()
            .unwrap_or(0)
    }

    fn find_lowest_fit(
        &self,
        request: &AllocationRequest,
        size: u64,
    ) -> Option<AllocationCandidate> {
        for (range_index, range) in self.free_ranges.iter().enumerate() {
            let start = range.start.max(request.min_address.as_u64());

            let end = request
                .max_address_exclusive
                .map(|maximum| range.end.min(maximum.as_u64()))
                .unwrap_or(range.end);

            if start >= end {
                continue;
            }

            let Some(base) = align_up(start, request.alignment) else {
                continue;
            };

            let Some(allocation_end) = base.checked_add(size) else {
                continue;
            };

            if allocation_end <= end {
                return Some(AllocationCandidate {
                    range_index,

                    start: base,

                    end: allocation_end,
                });
            }
        }

        None
    }

    fn find_highest_fit(
        &self,
        request: &AllocationRequest,
        size: u64,
    ) -> Option<AllocationCandidate> {
        for (range_index, range) in self.free_ranges.iter().enumerate().rev() {
            let start = range.start.max(request.min_address.as_u64());

            let end = request
                .max_address_exclusive
                .map(|maximum| range.end.min(maximum.as_u64()))
                .unwrap_or(range.end);

            if start >= end || end - start < size {
                continue;
            }

            let latest_base = end - size;

            let base = align_down(latest_base, request.alignment);

            if base >= start {
                return Some(AllocationCandidate {
                    range_index,

                    start: base,

                    end: base + size,
                });
            }
        }

        None
    }

    fn merge_adjacent_ranges(&mut self) {
        if self.free_ranges.len() < 2 {
            return;
        }

        let mut index = 0;

        while index + 1 < self.free_ranges.len() {
            if self.free_ranges[index].end == self.free_ranges[index + 1].start {
                let next_end = self.free_ranges[index + 1].end;

                self.free_ranges[index].end = next_end;

                self.free_ranges.remove(index + 1);
            } else {
                index += 1;
            }
        }
    }
}

fn ranges_overlap(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> bool {
    a_start < b_end && b_start < a_end
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    let mask = alignment.checked_sub(1)?;

    value.checked_add(mask).map(|value| value & !mask)
}

fn align_down(value: u64, alignment: u64) -> u64 {
    value & !(alignment - 1)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    use crate::boot::x86_64::{
        PhysicalAddress,
        memory::{MemoryRegion, MemoryRegionKind},
    };

    fn test_regions() -> Vec<MemoryRegion> {
        vec![
            MemoryRegion::new(
                PhysicalAddress::new(0x1000),
                0x9000,
                MemoryRegionKind::Usable,
            )
            .unwrap(),
            MemoryRegion::new(
                PhysicalAddress::new(0xA000),
                0x2000,
                MemoryRegionKind::Reserved,
            )
            .unwrap(),
            MemoryRegion::new(
                PhysicalAddress::new(0xC000),
                0x8000,
                MemoryRegionKind::Usable,
            )
            .unwrap(),
        ]
    }

    fn allocator() -> PhysicalPageAllocator {
        PhysicalPageAllocator::new(&test_regions()).unwrap()
    }

    #[test]
    fn only_usable_regions_become_free_pages() {
        let allocator = allocator();

        assert_eq!(allocator.free_pages(), 17);

        assert_eq!(allocator.largest_free_run_pages(), 9,);

        assert_eq!(allocator.free_range_count(), 2,);
    }

    #[test]
    fn lowest_address_allocation() {
        let mut allocator = allocator();

        let allocation = allocator.allocate(AllocationRequest::new(2)).unwrap();

        assert_eq!(allocation.base_address().as_u64(), 0x1000,);

        assert_eq!(allocation.size_bytes(), 0x2000,);

        assert_eq!(allocator.free_pages(), 15,);

        assert_eq!(allocator.allocated_pages(), 2,);
    }

    #[test]
    fn highest_address_allocation() {
        let mut allocator = allocator();

        let allocation = allocator
            .allocate(AllocationRequest::new(2).with_strategy(AllocationStrategy::HighestAddress))
            .unwrap();

        assert_eq!(allocation.base_address().as_u64(), 0x12000,);
    }

    #[test]
    fn alignment_and_address_window() {
        let regions = vec![
            MemoryRegion::new(
                PhysicalAddress::new(0x1000),
                0xA00000,
                MemoryRegionKind::Usable,
            )
            .unwrap(),
        ];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let allocation = allocator
            .allocate(
                AllocationRequest::new(1)
                    .with_min_address(PhysicalAddress::new(0x1000))
                    .with_max_address_exclusive(PhysicalAddress::new(0x900000))
                    .with_alignment(0x200000),
            )
            .unwrap();

        assert_eq!(allocation.base_address().as_u64(), 0x200000,);

        assert_eq!(allocation.base_address().as_u64() % 0x200000, 0,);

        assert!(allocation.end_exclusive().as_u64() <= 0x900000);
    }

    #[test]
    fn reservation_rounds_outward() {
        let mut allocator = allocator();

        let range = PhysicalRange::new(PhysicalAddress::new(0x3FFF), 2).unwrap();

        assert_eq!(allocator.reserve_range(range).unwrap(), 2,);

        assert_eq!(allocator.free_pages(), 15,);
    }

    #[test]
    fn freeing_allocations_merges_ranges() {
        let mut allocator = allocator();

        let first = allocator.allocate(AllocationRequest::new(2)).unwrap();

        let second = allocator.allocate(AllocationRequest::new(2)).unwrap();

        allocator.free(first).unwrap();

        allocator.free(second).unwrap();

        assert_eq!(allocator.free_pages(), 17,);

        assert_eq!(allocator.free_range_count(), 2,);

        assert_eq!(allocator.allocation_count(), 0,);
    }

    #[test]
    fn allocation_from_another_allocator_is_rejected() {
        let regions = test_regions();

        let mut first = PhysicalPageAllocator::new(&regions).unwrap();

        let mut second = PhysicalPageAllocator::new(&regions).unwrap();

        let allocation = first.allocate(AllocationRequest::new(1)).unwrap();

        assert_eq!(second.free(allocation), Err(MemoryError::UnknownAllocation),);
    }

    #[test]
    fn cannot_reserve_active_allocation() {
        let mut allocator = allocator();

        let allocation = allocator.allocate(AllocationRequest::new(2)).unwrap();

        let range = PhysicalRange::new(allocation.base_address(), allocation.size_bytes()).unwrap();

        assert_eq!(
            allocator.reserve_range(range),
            Err(MemoryError::ReservationOverlapsAllocation),
        );

        allocator.free(allocation).unwrap();
    }

    #[test]
    fn overlapping_memory_regions_are_rejected() {
        let regions = vec![
            MemoryRegion::new(
                PhysicalAddress::new(0x1000),
                0x5000,
                MemoryRegionKind::Usable,
            )
            .unwrap(),
            MemoryRegion::new(
                PhysicalAddress::new(0x4000),
                0x5000,
                MemoryRegionKind::Reserved,
            )
            .unwrap(),
        ];

        assert_eq!(
            PhysicalPageAllocator::new(&regions).err(),
            Some(MemoryError::OverlappingRegions),
        );
    }

    #[test]
    fn invalid_alignment_is_rejected() {
        let mut allocator = allocator();

        assert_eq!(
            allocator
                .allocate(AllocationRequest::new(1).with_alignment(3000),)
                .err(),
            Some(MemoryError::InvalidAlignment),
        );
    }

    #[test]
    fn zero_page_count_is_rejected() {
        let mut allocator = allocator();

        assert_eq!(
            allocator.allocate(AllocationRequest::new(0),).err(),
            Some(MemoryError::InvalidPageCount),
        );
    }

    #[test]
    fn allocation_failure_does_not_modify_allocator_state() {
        let mut allocator = allocator();

        let free_before = allocator.free_pages();

        let allocated_before = allocator.allocated_pages();

        let ranges_before = allocator.free_range_count();

        let result = allocator.allocate(AllocationRequest::new(100));

        assert!(matches!(result, Err(MemoryError::NoSuitableMemory),),);

        assert_eq!(allocator.free_pages(), free_before,);

        assert_eq!(allocator.allocated_pages(), allocated_before,);

        assert_eq!(allocator.free_range_count(), ranges_before,);

        assert_eq!(allocator.allocation_count(), 0,);
    }

    #[test]
    fn repeated_allocations_never_overlap_and_merge_after_free() {
        use alloc::vec::Vec;

        let regions = vec![
            MemoryRegion::new(
                PhysicalAddress::new(PAGE_SIZE),
                PAGE_SIZE * 16,
                MemoryRegionKind::Usable,
            )
            .unwrap(),
        ];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let mut allocations = Vec::new();

        for index in 0..16 {
            let allocation = allocator.allocate(AllocationRequest::new(1)).unwrap();

            assert_eq!(allocation.base_address().as_u64(), PAGE_SIZE * (index + 1),);

            allocations.push(allocation);
        }

        assert_eq!(allocator.free_pages(), 0,);

        assert_eq!(allocator.allocated_pages(), 16,);

        assert_eq!(allocator.allocation_count(), 16,);

        assert!(matches!(
            allocator.allocate(AllocationRequest::new(1)),
            Err(MemoryError::NoSuitableMemory),
        ),);

        for allocation in allocations.into_iter().rev() {
            allocator.free(allocation).unwrap();
        }

        assert_eq!(allocator.free_pages(), 16,);

        assert_eq!(allocator.allocated_pages(), 0,);

        assert_eq!(allocator.allocation_count(), 0,);

        assert_eq!(allocator.free_range_count(), 1,);
    }
}
