use alloc::vec::Vec;

use uefi::{
    Handle,
    proto::device_path::{DevicePath, media::HardDrive},
};

use super::protocols::open_device_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevicePathPartition {
    partition_number: u32,
    start_lba: u64,
    block_count: u64,
}

impl DevicePathPartition {
    pub const fn partition_number(&self) -> u32 {
        self.partition_number
    }

    pub const fn start_lba(&self) -> u64 {
        self.start_lba
    }

    pub const fn block_count(&self) -> u64 {
        self.block_count
    }

    pub fn last_lba(&self) -> Option<u64> {
        self.start_lba.checked_add(self.block_count.checked_sub(1)?)
    }
}

#[derive(Debug, Clone)]
pub struct DevicePathInfo {
    raw_bytes: Vec<u8>,

    partition: Option<DevicePathPartition>,
}

impl DevicePathInfo {
    pub(crate) fn discover(handle: Handle) -> Option<Self> {
        let protocol = open_device_path(handle).ok()?;

        let partition = protocol.node_iter().find_map(|node| {
            let hard_drive = <&HardDrive>::try_from(node).ok()?;

            Some(DevicePathPartition {
                partition_number: hard_drive.partition_number(),

                start_lba: hard_drive.partition_start(),

                block_count: hard_drive.partition_size(),
            })
        });

        let mut raw_bytes = Vec::new();

        raw_bytes
            .try_reserve_exact(protocol.as_bytes().len())
            .ok()?;

        raw_bytes.extend_from_slice(protocol.as_bytes());

        Some(Self {
            raw_bytes,
            partition,
        })
    }

    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw_bytes
    }

    pub fn as_device_path(&self) -> Option<&DevicePath> {
        <&DevicePath>::try_from(self.raw_bytes.as_slice()).ok()
    }

    pub const fn partition(&self) -> Option<&DevicePathPartition> {
        self.partition.as_ref()
    }

    pub const fn has_partition(&self) -> bool {
        self.partition.is_some()
    }
}
