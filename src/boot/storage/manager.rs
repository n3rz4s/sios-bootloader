use alloc::vec::Vec;

use uefi::{Handle, Status};

use super::{
    device::BlockDevice,
    device_path::DevicePathPartition,
    error::{StorageFailure, StorageResult, StorageStage},
    gpt::{GptError, GptPartition, GptTable},
    protocols::{find_block_io_handles, find_simple_file_system_handles},
    volume::StorageVolume,
};

#[derive(Debug, Clone)]
pub struct GptDisk {
    handle: Handle,

    table: GptTable,
}

impl GptDisk {
    pub const fn handle(&self) -> Handle {
        self.handle
    }

    pub fn table(&self) -> &GptTable {
        &self.table
    }

    pub fn partitions(&self) -> &[GptPartition] {
        self.table.partitions()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GptScanFailure {
    device_handle: Handle,

    error: GptError,
}

impl GptScanFailure {
    pub const fn device_handle(&self) -> Handle {
        self.device_handle
    }

    pub const fn error(&self) -> GptError {
        self.error
    }
}

pub struct StorageManager {
    volumes: Vec<StorageVolume>,

    block_devices: Vec<BlockDevice>,

    gpt_disks: Vec<GptDisk>,

    failures: Vec<StorageFailure>,

    gpt_failures: Vec<GptScanFailure>,
}

impl StorageManager {
    pub fn discover() -> StorageResult<Self> {
        let filesystem_handles = match find_simple_file_system_handles() {
            Ok(handles) => handles,

            Err(error) if error.status() == Status::NOT_FOUND => Vec::new(),

            Err(error) => {
                return Err(StorageFailure::new(
                    None,
                    StorageStage::EnumerateFileSystems,
                    error.status(),
                ));
            }
        };

        let block_io_handles = match find_block_io_handles() {
            Ok(handles) => handles,

            Err(error) if error.status() == Status::NOT_FOUND => Vec::new(),

            Err(error) => {
                return Err(StorageFailure::new(
                    None,
                    StorageStage::EnumerateBlockDevices,
                    error.status(),
                ));
            }
        };

        let mut volumes = Vec::new();

        let mut failures = Vec::new();

        for handle in filesystem_handles {
            match StorageVolume::open(handle) {
                Ok(volume) => {
                    volumes.push(volume);
                }

                Err(failure) => {
                    failures.push(failure);
                }
            }
        }

        let mut block_devices = Vec::new();

        for handle in block_io_handles {
            match BlockDevice::open(handle) {
                Ok(device) => {
                    block_devices.push(device);
                }

                Err(failure) => {
                    failures.push(failure);
                }
            }
        }

        let mut gpt_disks = Vec::new();

        let mut gpt_failures = Vec::new();

        for device in &block_devices {
            if device.is_logical_partition() || !device.is_media_present() {
                continue;
            }

            match GptTable::read(device) {
                Ok(Some(table)) => {
                    gpt_disks.push(GptDisk {
                        handle: device.handle(),
                        table,
                    });
                }

                Ok(None) => {}

                Err(error) => {
                    gpt_failures.push(GptScanFailure {
                        device_handle: device.handle(),
                        error,
                    });
                }
            }
        }

        Ok(Self {
            volumes,
            block_devices,
            gpt_disks,
            failures,
            gpt_failures,
        })
    }

    pub fn volumes(&self) -> &[StorageVolume] {
        &self.volumes
    }

    pub fn volumes_mut(&mut self) -> &mut [StorageVolume] {
        &mut self.volumes
    }

    pub fn volume_mut(&mut self, index: usize) -> Option<&mut StorageVolume> {
        self.volumes.get_mut(index)
    }

    pub fn block_devices(&self) -> &[BlockDevice] {
        &self.block_devices
    }

    pub fn gpt_disks(&self) -> &[GptDisk] {
        &self.gpt_disks
    }

    pub fn failures(&self) -> &[StorageFailure] {
        &self.failures
    }

    pub fn gpt_failures(&self) -> &[GptScanFailure] {
        &self.gpt_failures
    }

    pub fn volume_count(&self) -> usize {
        self.volumes.len()
    }

    pub fn block_device_count(&self) -> usize {
        self.block_devices.len()
    }

    pub fn gpt_disk_count(&self) -> usize {
        self.gpt_disks.len()
    }

    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub fn gpt_failure_count(&self) -> usize {
        self.gpt_failures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.volumes.is_empty() && self.block_devices.is_empty()
    }

    pub fn partition_for_volume(&self, volume_index: usize) -> Option<(&GptDisk, &GptPartition)> {
        let volume = self.volumes.get(volume_index)?;

        let device_path = volume.device_path()?;

        let partition_path: &DevicePathPartition = device_path.partition()?;

        let mut matched = None;

        for disk in &self.gpt_disks {
            for partition in disk.partitions() {
                let matches = partition.index() == partition_path.partition_number()
                    && partition.first_lba() == partition_path.start_lba()
                    && partition.block_count() == Some(partition_path.block_count());

                if !matches {
                    continue;
                }

                if matched.is_some() {
                    return None;
                }

                matched = Some((disk, partition));
            }
        }

        matched
    }
}
