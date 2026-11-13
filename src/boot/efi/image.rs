use uefi::{
    CString16, Handle, Status,
    boot::{self, LoadImageSource, OpenProtocolAttributes, OpenProtocolParams},
    proto::{BootPolicy, device_path::DevicePath, loaded_image::LoadedImage},
};

use super::error::{EfiLoadError, EfiLoadResult, EfiLoadStage};

pub(crate) struct LoadedEfiImage {
    handle: Handle,
}

impl LoadedEfiImage {
    pub fn load(device_path: &DevicePath) -> EfiLoadResult<Self> {
        let handle = boot::load_image(
            boot::image_handle(),
            LoadImageSource::FromDevicePath {
                device_path,

                boot_policy: BootPolicy::BootSelection,
            },
        )
        .map_err(|error| EfiLoadError::new(EfiLoadStage::LoadImage, error.status()))?;

        Ok(Self { handle })
    }

    fn set_load_options(&self, options: &CString16) -> EfiLoadResult<()> {
        let size = u32::try_from(options.num_bytes()).map_err(|_| {
            EfiLoadError::new(EfiLoadStage::SetLoadOptions, Status::BAD_BUFFER_SIZE)
        })?;

        let params = OpenProtocolParams {
            handle: self.handle,

            agent: boot::image_handle(),

            controller: None,
        };

        let mut protocol = unsafe {
            boot::open_protocol::<LoadedImage>(params, OpenProtocolAttributes::GetProtocol)
        }
        .map_err(|error| EfiLoadError::new(EfiLoadStage::OpenLoadedImage, error.status()))?;

        let loaded_image = protocol.get_mut().ok_or_else(|| {
            EfiLoadError::new(EfiLoadStage::OpenLoadedImage, Status::DEVICE_ERROR)
        })?;

        unsafe {
            loaded_image.set_load_options(options.as_ptr().cast::<u8>(), size);
        }

        Ok(())
    }

    pub fn start(self, load_options: Option<&str>) -> EfiLoadResult<()> {
        let load_options = match load_options {
            Some(options) => match CString16::try_from(options) {
                Ok(options) => Some(options),

                Err(_) => {
                    let _ = boot::unload_image(self.handle);

                    return Err(EfiLoadError::new(
                        EfiLoadStage::SetLoadOptions,
                        Status::INVALID_PARAMETER,
                    ));
                }
            },

            None => None,
        };

        if let Some(options) = load_options.as_ref() {
            if let Err(error) = self.set_load_options(options) {
                let _ = boot::unload_image(self.handle);

                return Err(error);
            }
        }

        let result = boot::start_image(self.handle)
            .map_err(|error| EfiLoadError::new(EfiLoadStage::StartImage, error.status()));

        let _ = boot::unload_image(self.handle);

        result
    }
}
