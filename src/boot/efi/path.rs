use alloc::{string::String, vec::Vec};

use uefi::{
    CString16, Status,
    proto::device_path::{
        DevicePath,
        build::{self, DevicePathBuilder},
    },
};

use crate::boot::storage::DevicePathInfo;

use super::error::{EfiLoadError, EfiLoadResult, EfiLoadStage};

const MAX_FILE_PATH_LEN: usize = 2048;

fn invalid_path() -> EfiLoadError {
    EfiLoadError::new(EfiLoadStage::BuildDevicePath, Status::INVALID_PARAMETER)
}

fn normalize_path(path: &str) -> EfiLoadResult<CString16> {
    if path.is_empty() || path.len() > MAX_FILE_PATH_LEN || path.contains('\0') {
        return Err(invalid_path());
    }

    let mut normalized = String::new();

    normalized
        .try_reserve_exact(path.len() + 1)
        .map_err(|_| EfiLoadError::new(EfiLoadStage::BuildDevicePath, Status::OUT_OF_RESOURCES))?;

    if !path.starts_with('\\') && !path.starts_with('/') {
        normalized.push('\\');
    }

    for character in path.chars() {
        normalized.push(if character == '/' { '\\' } else { character });
    }

    if normalized.ends_with('\\') {
        return Err(invalid_path());
    }

    if normalized
        .split('\\')
        .skip(1)
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(invalid_path());
    }

    CString16::try_from(normalized.as_str()).map_err(|_| invalid_path())
}

pub(crate) fn build_file_device_path<'a>(
    base: &DevicePathInfo,
    file_path: &str,
    buffer: &'a mut Vec<u8>,
) -> EfiLoadResult<&'a DevicePath> {
    let base_path = base
        .as_device_path()
        .ok_or_else(|| EfiLoadError::new(EfiLoadStage::BuildDevicePath, Status::NOT_FOUND))?;

    let file_path = normalize_path(file_path)?;

    let instance_count = base_path.instance_iter().count();

    if instance_count == 0 {
        return Err(invalid_path());
    }

    let file_path_node = build::media::FilePath {
        path_name: &file_path,
    };

    let mut builder = DevicePathBuilder::with_vec(buffer);

    for (index, instance) in base_path.instance_iter().enumerate() {
        for node in instance.node_iter() {
            builder = builder.push(&node).map_err(|_| {
                EfiLoadError::new(EfiLoadStage::BuildDevicePath, Status::OUT_OF_RESOURCES)
            })?;
        }

        builder = builder.push(&file_path_node).map_err(|_| {
            EfiLoadError::new(EfiLoadStage::BuildDevicePath, Status::OUT_OF_RESOURCES)
        })?;

        if index + 1 < instance_count {
            builder = builder.push(&build::end::Instance).map_err(|_| {
                EfiLoadError::new(EfiLoadStage::BuildDevicePath, Status::OUT_OF_RESOURCES)
            })?;
        }
    }

    builder
        .finalize()
        .map_err(|_| EfiLoadError::new(EfiLoadStage::BuildDevicePath, Status::OUT_OF_RESOURCES))
}
