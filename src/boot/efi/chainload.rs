use uefi::Status;

use crate::boot::{EfiTarget, StorageManager};

use super::{
    error::{EfiLoadError, EfiLoadResult, EfiLoadStage},
    image::LoadedEfiImage,
    path::build_file_device_path,
};

pub fn chainload(storage: &mut StorageManager, target: &EfiTarget) -> EfiLoadResult<()> {
    let image_path = target.image_path().as_str();

    let mut matching_volume = None;

    let mut scan_error = None;

    for (index, volume) in storage.volumes_mut().iter_mut().enumerate() {
        match volume.metadata(image_path) {
            Ok(metadata) => {
                if metadata.is_directory() {
                    continue;
                }

                if metadata.size() == 0 {
                    return Err(EfiLoadError::new(
                        EfiLoadStage::ResolveVolume,
                        Status::BAD_BUFFER_SIZE,
                    ));
                }

                if matching_volume.is_some() {
                    return Err(EfiLoadError::new(
                        EfiLoadStage::ResolveVolume,
                        Status::INVALID_PARAMETER,
                    ));
                }

                matching_volume = Some(index);
            }

            Err(error) if error.status() == Status::NOT_FOUND => {
                continue;
            }

            Err(error) => {
                scan_error.get_or_insert(error.status());
            }
        }
    }

    if let Some(status) = scan_error {
        return Err(EfiLoadError::new(EfiLoadStage::ResolveVolume, status));
    }

    let volume_index = matching_volume
        .ok_or_else(|| EfiLoadError::new(EfiLoadStage::ResolveVolume, Status::NOT_FOUND))?;

    let volume = storage
        .volumes()
        .get(volume_index)
        .ok_or_else(|| EfiLoadError::new(EfiLoadStage::ResolveVolume, Status::DEVICE_ERROR))?;

    let device_path = volume
        .device_path()
        .ok_or_else(|| EfiLoadError::new(EfiLoadStage::BuildDevicePath, Status::NOT_FOUND))?;

    let mut path_buffer = alloc::vec::Vec::new();

    let full_device_path = build_file_device_path(device_path, image_path, &mut path_buffer)?;

    let image = LoadedEfiImage::load(full_device_path)?;

    image.start(target.load_options())
}
