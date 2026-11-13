use alloc::vec::Vec;

use uefi::{
    Handle, Result,
    boot::{self, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol},
    proto::{
        device_path::DevicePath,
        media::{block::BlockIO, fs::SimpleFileSystem},
    },
};

pub(crate) fn find_simple_file_system_handles() -> Result<Vec<Handle>> {
    boot::find_handles::<SimpleFileSystem>()
}

pub(crate) fn find_block_io_handles() -> Result<Vec<Handle>> {
    boot::find_handles::<BlockIO>()
}

pub(crate) fn open_simple_file_system(handle: Handle) -> Result<ScopedProtocol<SimpleFileSystem>> {
    let params = OpenProtocolParams {
        handle,
        agent: boot::image_handle(),
        controller: None,
    };

    unsafe { boot::open_protocol::<SimpleFileSystem>(params, OpenProtocolAttributes::GetProtocol) }
}

pub(crate) fn open_block_io(handle: Handle) -> Result<ScopedProtocol<BlockIO>> {
    let params = OpenProtocolParams {
        handle,
        agent: boot::image_handle(),
        controller: None,
    };

    unsafe { boot::open_protocol::<BlockIO>(params, OpenProtocolAttributes::GetProtocol) }
}

pub(crate) fn open_device_path(handle: Handle) -> Result<ScopedProtocol<DevicePath>> {
    let params = OpenProtocolParams {
        handle,
        agent: boot::image_handle(),
        controller: None,
    };

    unsafe { boot::open_protocol::<DevicePath>(params, OpenProtocolAttributes::GetProtocol) }
}
