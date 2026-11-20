use super::{
    error::KernelImageError,
    header::{LinuxImageKind, SetupHeader},
    protocol::LINUX_64BIT_ENTRY_OFFSET,
};

use crate::boot::x86_64::{
    PhysicalAddress,
    memory::{AllocationRequest, MemoryError, PAGE_SIZE, PageAllocation, PhysicalPageAllocator},
};

const MIN_BZIMAGE_LOAD_ADDRESS: u64 = 0x10_0000;
const FOUR_GIB: u64 = 1u64 << 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelPlacementError {
    KernelImage(KernelImageError),

    HeaderMismatch,

    NotBzImage,

    Not64BitKernel,

    UnsupportedProtocol,

    MissingRequiredField,

    InvalidPreferredAddress,

    InvalidKernelAlignment,

    InvalidKernelSize,

    AddressOverflow,

    AddressAbove4GUnsupported,

    SourceImageTooSmall,

    DestinationBufferTooSmall,

    FixedAddressUnavailable,

    Memory(MemoryError),
}

#[must_use]
#[derive(Debug)]
pub struct KernelPlacement {
    allocation: PageAllocation,

    load_address: PhysicalAddress,

    runtime_start: PhysicalAddress,

    preferred_address: PhysicalAddress,

    source_offset: usize,

    protected_size: usize,

    memory_size: u64,

    alignment: u64,
}

impl KernelPlacement {
    pub fn allocate(
        image: &[u8],
        header: &SetupHeader,
        allocator: &mut PhysicalPageAllocator,
    ) -> Result<Self, KernelPlacementError> {
        let parsed_header = SetupHeader::parse(image).map_err(KernelPlacementError::KernelImage)?;

        if parsed_header != *header {
            return Err(KernelPlacementError::HeaderMismatch);
        }

        if header.image_kind() != LinuxImageKind::BzImage {
            return Err(KernelPlacementError::NotBzImage);
        }

        if !header.protocol_version().at_least(2, 12) {
            return Err(KernelPlacementError::UnsupportedProtocol);
        }

        if !header.supports_64bit() {
            return Err(KernelPlacementError::Not64BitKernel);
        }

        let relocatable = header
            .is_relocatable()
            .ok_or(KernelPlacementError::MissingRequiredField)?;

        let preferred_address = header
            .pref_address()
            .filter(|address| *address != 0)
            .ok_or(KernelPlacementError::MissingRequiredField)?;

        if preferred_address < MIN_BZIMAGE_LOAD_ADDRESS {
            return Err(KernelPlacementError::InvalidPreferredAddress);
        }

        let protected_size = header
            .syssize_bytes()
            .filter(|size| *size != 0)
            .ok_or(KernelPlacementError::MissingRequiredField)?;

        let memory_size = header
            .init_size()
            .filter(|size| *size != 0)
            .ok_or(KernelPlacementError::MissingRequiredField)? as u64;

        if memory_size < protected_size {
            return Err(KernelPlacementError::InvalidKernelSize);
        }

        let source_offset = header.setup_size();

        let protected_size_usize =
            usize::try_from(protected_size).map_err(|_| KernelPlacementError::InvalidKernelSize)?;

        let source_end = source_offset
            .checked_add(protected_size_usize)
            .ok_or(KernelPlacementError::AddressOverflow)?;

        if source_end > image.len() {
            return Err(KernelPlacementError::SourceImageTooSmall);
        }

        let allocation_size =
            align_up(memory_size, PAGE_SIZE).ok_or(KernelPlacementError::AddressOverflow)?;

        let page_count = allocation_size / PAGE_SIZE;

        let supports_above_4g = header.supports_above_4g();

        let address_limit = if supports_above_4g {
            None
        } else {
            Some(FOUR_GIB)
        };

        if !supports_above_4g {
            let minimum_end = preferred_address
                .checked_add(allocation_size)
                .ok_or(KernelPlacementError::AddressOverflow)?;

            if minimum_end > FOUR_GIB {
                return Err(KernelPlacementError::AddressAbove4GUnsupported);
            }
        }

        let preferred = PhysicalAddress::new(preferred_address);

        let (allocation, selected_alignment) = if relocatable {
            Self::allocate_relocatable(
                allocator,
                page_count,
                allocation_size,
                preferred,
                address_limit,
                header,
            )?
        } else {
            if preferred_address % PAGE_SIZE != 0 {
                return Err(KernelPlacementError::InvalidPreferredAddress);
            }

            let allocation = allocate_exact(
                allocator,
                page_count,
                allocation_size,
                preferred,
                PAGE_SIZE,
                address_limit,
            )?
            .ok_or(KernelPlacementError::FixedAddressUnavailable)?;

            (allocation, PAGE_SIZE)
        };

        let load_address = allocation.base_address();

        let runtime_start = if relocatable {
            load_address.as_u64()
        } else {
            preferred_address
        };

        if runtime_start != load_address.as_u64()
            || (relocatable
                && (load_address.as_u64() < preferred_address
                    || load_address.as_u64() % selected_alignment != 0))
        {
            let _ = allocator.free(allocation);
            return Err(KernelPlacementError::InvalidKernelAlignment);
        }

        Ok(Self {
            allocation,
            load_address,
            runtime_start: PhysicalAddress::new(runtime_start),
            preferred_address: preferred,
            source_offset,
            protected_size: protected_size_usize,
            memory_size,
            alignment: selected_alignment,
        })
    }

    fn allocate_relocatable(
        allocator: &mut PhysicalPageAllocator,
        page_count: u64,
        allocation_size: u64,
        preferred: PhysicalAddress,
        address_limit: Option<u64>,
        header: &SetupHeader,
    ) -> Result<(PageAllocation, u64), KernelPlacementError> {
        let preferred_alignment = header
            .kernel_alignment()
            .ok_or(KernelPlacementError::MissingRequiredField)?
            as u64;

        if preferred_alignment == 0 || !preferred_alignment.is_power_of_two() {
            return Err(KernelPlacementError::InvalidKernelAlignment);
        }

        let minimum_exponent = header
            .min_alignment()
            .ok_or(KernelPlacementError::MissingRequiredField)?
            as u32;

        let minimum_alignment = 1u64
            .checked_shl(minimum_exponent)
            .ok_or(KernelPlacementError::InvalidKernelAlignment)?
            .max(PAGE_SIZE);

        let mut alignment = preferred_alignment.max(PAGE_SIZE);

        if minimum_alignment > alignment {
            return Err(KernelPlacementError::InvalidKernelAlignment);
        }

        loop {
            if preferred.as_u64() % alignment == 0 {
                if let Some(allocation) = allocate_exact(
                    allocator,
                    page_count,
                    allocation_size,
                    preferred,
                    alignment,
                    address_limit,
                )? {
                    return Ok((allocation, alignment));
                }
            }

            if let Some(allocation) =
                allocate_at_or_above(allocator, page_count, preferred, alignment, address_limit)?
            {
                return Ok((allocation, alignment));
            }

            if alignment == minimum_alignment {
                break;
            }

            alignment = (alignment / 2).max(minimum_alignment);
        }

        Err(KernelPlacementError::Memory(MemoryError::NoSuitableMemory))
    }

    pub const fn load_address(&self) -> PhysicalAddress {
        self.load_address
    }

    pub const fn runtime_start(&self) -> PhysicalAddress {
        self.runtime_start
    }

    pub const fn preferred_address(&self) -> PhysicalAddress {
        self.preferred_address
    }

    pub const fn protected_size(&self) -> usize {
        self.protected_size
    }

    pub const fn memory_size(&self) -> u64 {
        self.memory_size
    }

    pub const fn alignment(&self) -> u64 {
        self.alignment
    }

    pub const fn page_count(&self) -> u64 {
        self.allocation.page_count()
    }

    pub const fn allocation_size(&self) -> u64 {
        self.allocation.size_bytes()
    }

    pub fn entry_point(&self) -> Result<PhysicalAddress, KernelPlacementError> {
        self.load_address
            .checked_add(LINUX_64BIT_ENTRY_OFFSET)
            .ok_or(KernelPlacementError::AddressOverflow)
    }

    pub fn copy_into(
        &self,
        image: &[u8],
        destination: &mut [u8],
    ) -> Result<(), KernelPlacementError> {
        let allocation_size = usize::try_from(self.allocation.size_bytes())
            .map_err(|_| KernelPlacementError::InvalidKernelSize)?;

        if destination.len() < allocation_size {
            return Err(KernelPlacementError::DestinationBufferTooSmall);
        }

        let source_end = self
            .source_offset
            .checked_add(self.protected_size)
            .ok_or(KernelPlacementError::AddressOverflow)?;

        let source = image
            .get(self.source_offset..source_end)
            .ok_or(KernelPlacementError::SourceImageTooSmall)?;

        destination[..allocation_size].fill(0);
        destination[..self.protected_size].copy_from_slice(source);

        Ok(())
    }

    pub fn release(self, allocator: &mut PhysicalPageAllocator) -> Result<(), MemoryError> {
        allocator.free(self.allocation)
    }
}

fn allocate_exact(
    allocator: &mut PhysicalPageAllocator,
    page_count: u64,
    allocation_size: u64,
    address: PhysicalAddress,
    alignment: u64,
    address_limit: Option<u64>,
) -> Result<Option<PageAllocation>, KernelPlacementError> {
    let end = address
        .as_u64()
        .checked_add(allocation_size)
        .ok_or(KernelPlacementError::AddressOverflow)?;

    if address_limit.is_some_and(|limit| end > limit) {
        return Ok(None);
    }

    let request = AllocationRequest::new(page_count)
        .with_min_address(address)
        .with_max_address_exclusive(PhysicalAddress::new(end))
        .with_alignment(alignment);

    match allocator.allocate(request) {
        Ok(allocation) => Ok(Some(allocation)),
        Err(MemoryError::NoSuitableMemory) => Ok(None),
        Err(error) => Err(KernelPlacementError::Memory(error)),
    }
}

fn allocate_at_or_above(
    allocator: &mut PhysicalPageAllocator,
    page_count: u64,
    minimum_address: PhysicalAddress,
    alignment: u64,
    address_limit: Option<u64>,
) -> Result<Option<PageAllocation>, KernelPlacementError> {
    let mut request = AllocationRequest::new(page_count)
        .with_min_address(minimum_address)
        .with_alignment(alignment);

    if let Some(limit) = address_limit {
        request = request.with_max_address_exclusive(PhysicalAddress::new(limit));
    }

    match allocator.allocate(request) {
        Ok(allocation) => Ok(Some(allocation)),
        Err(MemoryError::NoSuitableMemory) => Ok(None),
        Err(error) => Err(KernelPlacementError::Memory(error)),
    }
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return None;
    }

    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    use crate::boot::x86_64::linux::header::{XLF_CAN_BE_LOADED_ABOVE_4G, XLF_KERNEL_64};
    use crate::boot::x86_64::memory::{MemoryRegion, MemoryRegionKind};

    fn put_u16(image: &mut [u8], offset: usize, value: u16) {
        image[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(image: &mut [u8], offset: usize, value: u32) {
        image[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(image: &mut [u8], offset: usize, value: u64) {
        image[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn make_kernel(
        relocatable: bool,
        supports_above_4g: bool,
        preferred_address: u64,
        init_size: u32,
        min_alignment: u8,
    ) -> Vec<u8> {
        let mut image = vec![0u8; 4096];

        image[0x1F1] = 1;

        put_u32(&mut image, 0x1F4, 64);
        put_u16(&mut image, 0x1FE, 0xAA55);

        image[0x200] = 0xEB;
        image[0x201] = 0x6A;

        image[0x202..0x206].copy_from_slice(b"HdrS");

        put_u16(&mut image, 0x206, 0x020F);

        image[0x211] = 1;

        put_u32(&mut image, 0x22C, u32::MAX);
        put_u32(&mut image, 0x230, 0x200000);

        image[0x234] = u8::from(relocatable);
        image[0x235] = min_alignment;

        let mut flags = XLF_KERNEL_64;

        if supports_above_4g {
            flags |= XLF_CAN_BE_LOADED_ABOVE_4G;
        }

        put_u16(&mut image, 0x236, flags);
        put_u32(&mut image, 0x238, 2047);
        put_u32(&mut image, 0x248, 0);
        put_u32(&mut image, 0x24C, 0);
        put_u64(&mut image, 0x258, preferred_address);
        put_u32(&mut image, 0x260, init_size);

        for (index, byte) in image.iter_mut().enumerate().skip(1024).take(1024) {
            *byte = (index & 0xFF) as u8;
        }

        image
    }

    fn usable_region(start: u64, length: u64) -> MemoryRegion {
        MemoryRegion::new(
            PhysicalAddress::new(start),
            length,
            MemoryRegionKind::Usable,
        )
        .unwrap()
    }

    #[test]
    fn relocatable_kernel_uses_preferred_address_when_available() {
        let image = make_kernel(true, false, 0x2000000, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(0x1000000, 0x7000000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        assert_eq!(placement.load_address().as_u64(), 0x2000000);

        assert_eq!(placement.runtime_start().as_u64(), 0x2000000);

        assert_eq!(placement.entry_point().unwrap().as_u64(), 0x2000200);

        assert_eq!(placement.page_count(), 3);

        assert_eq!(placement.alignment(), 0x200000);
    }

    #[test]
    fn relocatable_kernel_falls_back_to_higher_address() {
        let image = make_kernel(true, false, 0x2000000, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [
            usable_region(0x1000000, 0x1000000),
            MemoryRegion::new(
                PhysicalAddress::new(0x2000000),
                0x200000,
                MemoryRegionKind::Reserved,
            )
            .unwrap(),
            usable_region(0x2200000, 0x3000000),
        ];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        assert_eq!(placement.load_address().as_u64(), 0x2200000);

        assert_eq!(placement.runtime_start(), placement.load_address());
    }

    #[test]
    fn relocatable_kernel_reduces_alignment_to_minimum() {
        let preferred = 0x2000000;

        let image = make_kernel(true, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred + PAGE_SIZE, PAGE_SIZE * 3)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        assert_eq!(placement.load_address().as_u64(), preferred + PAGE_SIZE);

        assert_eq!(placement.alignment(), PAGE_SIZE);
    }

    #[test]
    fn non_relocatable_kernel_uses_exact_preferred_address() {
        let preferred = 0x2000000;

        let image = make_kernel(false, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred, 0x10000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        assert_eq!(placement.load_address().as_u64(), preferred);

        assert_eq!(placement.runtime_start().as_u64(), preferred);
    }

    #[test]
    fn non_relocatable_kernel_does_not_move_when_preferred_address_is_busy() {
        let preferred = 0x2000000;

        let image = make_kernel(false, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(0x3000000, 0x10000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        assert!(matches!(
            KernelPlacement::allocate(&image, &header, &mut allocator),
            Err(KernelPlacementError::FixedAddressUnavailable),
        ));
    }

    #[test]
    fn kernel_above_4g_requires_header_support() {
        let preferred = FOUR_GIB + 0x2000000;

        let image = make_kernel(true, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred, 0x10000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        assert!(matches!(
            KernelPlacement::allocate(&image, &header, &mut allocator),
            Err(KernelPlacementError::AddressAbove4GUnsupported),
        ));
    }

    #[test]
    fn kernel_above_4g_is_allowed_when_header_supports_it() {
        let preferred = FOUR_GIB + 0x2000000;

        let image = make_kernel(true, true, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred, 0x10000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        assert_eq!(placement.load_address().as_u64(), preferred);

        assert_eq!(placement.entry_point().unwrap().as_u64(), preferred + 0x200);
    }

    #[test]
    fn copy_into_clears_window_and_copies_only_protected_image() {
        let preferred = 0x2000000;

        let image = make_kernel(true, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred, 0x10000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        let mut destination = vec![0xAA; placement.allocation_size() as usize];

        placement.copy_into(&image, &mut destination).unwrap();

        assert_eq!(&destination[..1024], &image[1024..2048]);

        assert!(destination[1024..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn init_size_must_cover_protected_kernel_image() {
        let preferred = 0x2000000;

        let image = make_kernel(true, false, preferred, 512, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred, 0x10000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        assert!(matches!(
            KernelPlacement::allocate(&image, &header, &mut allocator),
            Err(KernelPlacementError::InvalidKernelSize),
        ));
    }

    #[test]
    fn release_restores_allocator_state() {
        let preferred = 0x2000000;

        let image = make_kernel(true, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred, 0x100000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let free_before = allocator.free_pages();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        assert_eq!(allocator.allocation_count(), 1,);

        assert_eq!(allocator.free_pages(), free_before - placement.page_count(),);

        placement.release(&mut allocator).unwrap();

        assert_eq!(allocator.allocation_count(), 0,);

        assert_eq!(allocator.free_pages(), free_before,);
    }

    #[test]
    fn copy_into_rejects_small_destination() {
        let preferred = 0x2000000;

        let image = make_kernel(true, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred, 0x100000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        let mut destination = vec![0u8; placement.allocation_size() as usize - 1];

        assert_eq!(
            placement.copy_into(&image, &mut destination,),
            Err(KernelPlacementError::DestinationBufferTooSmall),
        );
    }

    #[test]
    fn copy_into_rejects_truncated_source_image() {
        let preferred = 0x2000000;

        let image = make_kernel(true, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred, 0x100000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        let mut destination = vec![0u8; placement.allocation_size() as usize];

        let truncated_image = &image[..header.setup_size()];

        assert_eq!(
            placement.copy_into(truncated_image, &mut destination,),
            Err(KernelPlacementError::SourceImageTooSmall),
        );
    }

    #[test]
    fn relocatable_kernel_fails_when_no_memory_exists_above_preferred_address() {
        let preferred = 0x2000000;

        let image = make_kernel(true, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(0x1000000, 0x1000000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        assert!(matches!(
            KernelPlacement::allocate(&image, &header, &mut allocator,),
            Err(KernelPlacementError::Memory(MemoryError::NoSuitableMemory,)),
        ),);

        assert_eq!(allocator.allocation_count(), 0,);
    }

    #[test]
    fn unaligned_preferred_address_is_rounded_up_to_kernel_alignment() {
        let preferred = 0x2001000;

        let image = make_kernel(true, false, preferred, 0x3000, 12);

        let header = SetupHeader::parse(&image).unwrap();

        let regions = [usable_region(preferred, 0x500000)];

        let mut allocator = PhysicalPageAllocator::new(&regions).unwrap();

        let placement = KernelPlacement::allocate(&image, &header, &mut allocator).unwrap();

        assert_eq!(placement.load_address().as_u64(), 0x2200000,);

        assert_eq!(placement.alignment(), 0x200000,);

        assert_eq!(placement.load_address.as_u64() % placement.alignment(), 0,);

        assert!(placement.load_address().as_u64() >= preferred,);
    }
}
