use alloc::vec::Vec;

use uefi::{
    Handle, Status,
    boot::ScopedProtocol,
    proto::media::block::{BlockIO, Lba},
};

use super::{
    device_path::DevicePathInfo,
    error::{StorageFailure, StorageResult, StorageStage},
    protocols::open_block_io,
};

pub struct BlockDevice {
    handle: Handle,

    protocol: ScopedProtocol<BlockIO>,

    device_path: Option<DevicePathInfo>,
}

impl BlockDevice {
    pub fn open(handle: Handle) -> StorageResult<Self> {
        let protocol = open_block_io(handle).map_err(|error| {
            StorageFailure::new(Some(handle), StorageStage::OpenBlockIo, error.status())
        })?;

        let device_path = DevicePathInfo::discover(handle);

        Ok(Self {
            handle,
            protocol,
            device_path,
        })
    }

    pub const fn handle(&self) -> Handle {
        self.handle
    }

    pub fn device_path(&self) -> Option<&DevicePathInfo> {
        self.device_path.as_ref()
    }

    pub fn media_id(&self) -> u32 {
        self.protocol.media().media_id()
    }

    pub fn block_size(&self) -> u32 {
        self.protocol.media().block_size()
    }

    pub fn io_align(&self) -> u32 {
        self.protocol.media().io_align()
    }

    pub fn last_block(&self) -> u64 {
        self.protocol.media().last_block()
    }

    pub fn is_media_present(&self) -> bool {
        self.protocol.media().is_media_present()
    }

    pub fn is_removable_media(&self) -> bool {
        self.protocol.media().is_removable_media()
    }

    pub fn is_logical_partition(&self) -> bool {
        self.protocol.media().is_logical_partition()
    }

    pub fn is_read_only(&self) -> bool {
        self.protocol.media().is_read_only()
    }

    pub fn block_count(&self) -> Option<u64> {
        self.last_block().checked_add(1)
    }

    pub fn capacity_bytes(&self) -> Option<u64> {
        self.block_count()?.checked_mul(self.block_size() as u64)
    }

    pub fn read_blocks(&self, starting_lba: u64, block_count: usize) -> StorageResult<Vec<u8>> {
        let handle = Some(self.handle);

        if block_count == 0 {
            return Ok(Vec::new());
        }

        let (media_id, block_size, io_align, last_block, media_present) = {
            let media = self.protocol.media();

            (
                media.media_id(),
                media.block_size(),
                media.io_align(),
                media.last_block(),
                media.is_media_present(),
            )
        };

        if !media_present {
            return Err(StorageFailure::new(
                handle,
                StorageStage::ReadBlocks,
                Status::NO_MEDIA,
            ));
        }

        if block_size == 0 {
            return Err(StorageFailure::new(
                handle,
                StorageStage::ReadBlocks,
                Status::DEVICE_ERROR,
            ));
        }

        let count_u64 = u64::try_from(block_count).map_err(|_| {
            StorageFailure::new(handle, StorageStage::ReadBlocks, Status::BAD_BUFFER_SIZE)
        })?;

        let final_lba = starting_lba.checked_add(count_u64 - 1).ok_or_else(|| {
            StorageFailure::new(handle, StorageStage::ReadBlocks, Status::INVALID_PARAMETER)
        })?;

        if final_lba > last_block {
            return Err(StorageFailure::new(
                handle,
                StorageStage::ReadBlocks,
                Status::INVALID_PARAMETER,
            ));
        }

        let byte_count = block_count
            .checked_mul(block_size as usize)
            .ok_or_else(|| {
                StorageFailure::new(handle, StorageStage::ReadBlocks, Status::BAD_BUFFER_SIZE)
            })?;

        let alignment = (io_align as usize).max(1);

        let allocation_size = byte_count.checked_add(alignment - 1).ok_or_else(|| {
            StorageFailure::new(handle, StorageStage::ReadBlocks, Status::OUT_OF_RESOURCES)
        })?;

        let mut allocation = Vec::new();

        allocation.try_reserve_exact(allocation_size).map_err(|_| {
            StorageFailure::new(handle, StorageStage::ReadBlocks, Status::OUT_OF_RESOURCES)
        })?;

        allocation.resize(allocation_size, 0);

        let base_address = allocation.as_ptr() as usize;

        let misalignment = base_address % alignment;

        let offset = if misalignment == 0 {
            0
        } else {
            alignment - misalignment
        };

        let aligned_buffer = &mut allocation[offset..offset + byte_count];

        self.protocol
            .read_blocks(media_id, starting_lba as Lba, aligned_buffer)
            .map_err(|error| {
                StorageFailure::new(handle, StorageStage::ReadBlocks, error.status())
            })?;

        let mut result = Vec::new();

        result.try_reserve_exact(byte_count).map_err(|_| {
            StorageFailure::new(handle, StorageStage::ReadBlocks, Status::OUT_OF_RESOURCES)
        })?;

        result.extend_from_slice(aligned_buffer);

        Ok(result)
    }
}
