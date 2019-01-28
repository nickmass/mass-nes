//! NES NTSC video filter. Pixel artifacts and color mixing play an important
//! role in NES graphics. Accepts pixels in a native 9-bit format that includes
//! the three color emphasis bits from PPU register $2001.
//! Can also output an RGB palette for use in a regular blitter
//! # Based Upon
//! This library is a very thin wrapper on the original C library, found here:
//! http://slack.net/~ant/libs/ntsc.html
extern crate nes_ntsc_sys as ffi;

/// Image parameters, ranging from -1.0 to 1.0. Actual interal values shown
/// in parenthesis and should remain fairly stage in future versions.
pub struct NesNtscSetup(ffi::nes_ntsc_setup_t);
impl NesNtscSetup {
    /// color bleeding + artifacts
    pub fn composite() -> NesNtscSetup {
        unsafe { NesNtscSetup(ffi::nes_ntsc_composite) }
    }

    /// color bleeding only
    pub fn svideo() -> NesNtscSetup {
        unsafe { NesNtscSetup(ffi::nes_ntsc_svideo) }
    }

    /// crisp image
    pub fn rgb() -> NesNtscSetup {
        unsafe { NesNtscSetup(ffi::nes_ntsc_rgb) }
    }

    /// desaturated + artifacts
    pub fn monochrome() -> NesNtscSetup {
        unsafe { NesNtscSetup(ffi::nes_ntsc_monochrome) }
    }

    /// -1 = -180 degrees, +1 = +180 degrees
    pub fn set_hue(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.hue = val;
        self
    }

    /// -1 = grayscale (0.0), +1 = oversaturated colors (2.0)
    pub fn set_saturation(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.saturation = val;
        self
    }

    /// -1 = dark (0.5), +1 = light (1.5)
    pub fn set_contrast(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.contrast = val;
        self
    }

    /// -1 = dark (0.5), +1 = light (1.5)
    pub fn set_brightness(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.brightness = val;
        self
    }

    /// edge contrast enhancement/blurring
    pub fn set_sharpness(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.sharpness = val;
        self
    }

    /// -1 = dark (0.5), +1 = light (1.5)
    pub fn set_gamma(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.gamma = val;
        self
    }

    /// image resolution
    pub fn set_resolution(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.resolution = val;
        self
    }

    /// artifacts caused by color changes
    pub fn set_artifacts(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.artifacts = val;
        self
    }

    /// color artifacts caused by brightness changes
    pub fn set_fringing(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.fringing = val;
        self
    }

    /// color bleed (color resolution reduction)
    pub fn set_bleed(&mut self, val: f64) -> &mut Self {
        assert!(val >= -1f64 && val <= 1f64);
        self.0.bleed = val;
        self
    }

    /// merges even and odd fields together to reduce flicker
    pub fn merge_fields(&mut self) -> &mut Self {
        self.0.merge_fields = 1;
        self
    }

    /// An RGB palette can be generated for use in a normal blitter. It's 512
    /// colors (1536 bytes).
    pub fn generate_palette(&mut self) -> [u8; 1536] {
        let mut buf = [0; 1536];
        self.0.palette_out = buf.as_mut_ptr();

        unsafe {
            ffi::nes_ntsc_init(
                ::std::ptr::null_mut(),
                &self.0 as *const ffi::nes_ntsc_setup_t,
            );
        }

        buf
    }
}

/// NTSC filter
pub struct NesNtsc(ffi::nes_ntsc_t);

impl NesNtsc {
    /// Intializes and adjust parameters
    pub fn new(setup: NesNtscSetup) -> NesNtsc {
        let mut ntsc = NesNtsc(unsafe { ::std::mem::zeroed() });

        unsafe {
            ffi::nes_ntsc_init(
                &mut ntsc.0 as *mut ffi::nes_ntsc_t,
                &setup.0 as *const ffi::nes_ntsc_setup_t,
            );
        }

        ntsc
    }

    /// Number of output pixels written by blitter for given input width. Width might be rounded
    /// down slightly; use `NesNtsc::in_width()` on result to find rounded value.
    /// Guaranteed not to round 256 down at all.
    pub fn out_width(in_width: u32) -> u32 {
        ((in_width - 1) / 3 + 1) * 7
    }

    /// Number of input pixels that will fit within given output width. Might be rounded down
    /// slightly; use `NesNtsc::out_width()` on result to find rounded value.
    pub fn in_width(out_width: u32) -> u32 {
        (out_width / 7 - 1) * 3 + 1
    }

    /// Filters one or more rows of pixels. Input pixels are 9-bit palette indicies. `out_pitch`
    /// is the number of **bytes** to get to the next output row. Output pixel format is 32-bit RGB.
    /// The `burst_phase` parameter should generally toggle values between frames, i.e. 0 on first
    /// call, 1 on second call, 0 on third call, 1 on fourth, etc. If `merge_fields` is enabled,
    /// you should always pass 0.
    pub fn blit(
        &mut self,
        in_width: u32,
        in_pixels: &[u16],
        burst_phase: u32,
        out_pixels: &mut [u32],
        out_pitch: u32,
    ) {
        let rows = in_pixels.len() as u32 / in_width;
        unsafe {
            ffi::nes_ntsc_blit(
                &self.0 as *const ffi::nes_ntsc_t,
                in_pixels.as_ptr(),
                in_width as ::std::os::raw::c_long,
                burst_phase as ::std::os::raw::c_int,
                in_width as ::std::os::raw::c_int,
                rows as ::std::os::raw::c_int,
                out_pixels.as_mut_ptr() as *mut ::std::os::raw::c_void,
                out_pitch as ::std::os::raw::c_long,
            );
        }
    }
}
