use agx_definitions::Size;
use log::info;
use uefi::{
    boot,
    boot::{OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol},
    println,
    proto::console::gop::GraphicsOutput,
    Result,
};

pub fn set_resolution(desired_resolution: Size) -> Result<ScopedProtocol<GraphicsOutput>> {
    println!("trying to get protos");
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;
    // PT: open_protocol_exclusive just hangs forever, so ask more politely
    let mut gop = unsafe {
        boot::open_protocol::<GraphicsOutput>(
            OpenProtocolParams {
                handle: gop_handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }?;

    let mut switched_to_desired_resolution = false;
    for mode in gop.modes() {
        let res = mode.info().resolution();
        info!("Found supported resolution {:?}", res);
        if res
            == (
                desired_resolution.width as _,
                desired_resolution.height as _,
            )
        {
            gop.set_mode(&mode)
                .expect("Failed to set desired resolution");
            switched_to_desired_resolution = true;
            break;
        }
    }
    if !switched_to_desired_resolution {
        panic!("Failed to switch to the desired resolution");
    }

    Ok(gop)
}
