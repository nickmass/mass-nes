pub trait TracyExt {
    fn plot_config(
        &self,
        name: &'static std::ffi::CStr,
        step: bool,
        fill: bool,
        color: Option<u32>,
    );
    fn plot_int(&self, name: &'static std::ffi::CStr, value: i64);
    fn emit_frame_image(&self, data: &[u8], width: u16, height: u16, offset: u8, flip: bool);
}

impl TracyExt for tracy_client::Client {
    fn plot_config(
        &self,
        name: &'static std::ffi::CStr,
        step: bool,
        fill: bool,
        color: Option<u32>,
    ) {
        unsafe {
            tracy_client::sys::___tracy_emit_plot_config(
                name.as_ptr(),
                tracy_client::sys::TracyPlotFormatEnum_TracyPlotFormatNumber as i32,
                step as i32,
                fill as i32,
                color.unwrap_or(0),
            );
        }
    }

    fn plot_int(&self, name: &'static std::ffi::CStr, value: i64) {
        unsafe {
            tracy_client::sys::___tracy_emit_plot_int(name.as_ptr(), value);
        }
    }

    fn emit_frame_image(&self, data: &[u8], width: u16, height: u16, offset: u8, flip: bool) {
        unsafe {
            tracy_client::sys::___tracy_emit_frame_image(
                data.as_ptr() as _,
                width,
                height,
                offset,
                flip as i32,
            );
        }
    }
}
