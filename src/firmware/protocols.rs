use uefi::{
    Result,
    boot::{self, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol},
    proto::console::gop::GraphicsOutput,
};

pub fn open_graphics_output() -> Result<ScopedProtocol<GraphicsOutput>> {
    let handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;

    let params = OpenProtocolParams {
        handle,
        agent: boot::image_handle(),
        controller: None,
    };

    unsafe { boot::open_protocol::<GraphicsOutput>(params, OpenProtocolAttributes::GetProtocol) }
}
