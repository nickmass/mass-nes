// Derived from nes_ntsc 0.2.2
/* nes_ntsc 0.2.2. http://www.slack.net/~ant/ */

/* Copyright (C) 2006-2007 Shay Green. This module is free software; you
can redistribute it and/or modify it under the terms of the GNU Lesser
General Public License as published by the Free Software Foundation; either
version 2.1 of the License, or (at your option) any later version. This
module is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License for more
details. You should have received a copy of the GNU Lesser General Public
License along with this module; if not, write to the Free Software Foundation,
Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA */

pub struct NesNtscSetup {
    /// -1 = -180 degrees, +1 = +180 degrees
    hue: f64,
    /// -1 = grayscale (0.0), +1 = oversaturated colors (2.0)
    saturation: f64,
    /// -1 = dark (0.5), +1 = light (1.5)
    contrast: f64,
    /// -1 = dark (0.5), +1 = light (1.5)
    brightness: f64,
    /// edge contrast enhancement/blurring
    sharpness: f64,

    /// -1 = dark (1.5), +1 = light (0.5)
    gamma: f64,
    /// image resolution
    resolution: f64,
    /// artifacts casused by color changes
    artifacts: f64,
    /// color artifacts caused by brightness changes
    fringing: f64,
    /// color bleed (color resolution reduction)
    bleed: f64,
    /// if set, merges even and odd fields together to reduce flicker
    merge_fields: bool,
    /// optional RGB decoder matrix
    decoder_matrix: Option<[f32; 6]>,

    /* You can replace the standard NES color generation with an RGB palette. The
    first replaces all color generation, while the second replaces only the core
    64-color generation and does standard color emphasis calculations on it. */
    /// optional 512-entry RGB palette in, 3 bytes per color
    palette: Option<[u8; 64 * 8 * 3]>,
    /// optional 64-entry RGB palette in, 3 bytes per color
    base_palette: Option<[u8; 64 * 3]>,
}

impl NesNtscSetup {
    pub fn composite() -> Self {
        NesNtscSetup {
            hue: 0.0,
            saturation: 0.0,
            contrast: 0.0,
            brightness: 0.0,
            sharpness: 0.0,
            gamma: 0.0,
            resolution: 0.0,
            artifacts: 0.0,
            fringing: 0.0,
            bleed: 0.0,
            merge_fields: true,
            decoder_matrix: None,
            palette: None,
            base_palette: None,
        }
    }

    pub fn monochrome() -> Self {
        NesNtscSetup {
            hue: 0.0,
            saturation: -1.0,
            contrast: 0.0,
            brightness: 0.0,
            sharpness: 0.2,
            gamma: 0.0,
            resolution: 0.2,
            artifacts: -0.2,
            fringing: -0.2,
            bleed: -1.0,
            merge_fields: true,
            decoder_matrix: None,
            palette: None,
            base_palette: None,
        }
    }

    pub fn svideo() -> Self {
        NesNtscSetup {
            hue: 0.0,
            saturation: 0.0,
            contrast: 0.0,
            brightness: 0.0,
            sharpness: 0.2,
            gamma: 0.0,
            resolution: 0.2,
            artifacts: -1.0,
            fringing: -1.0,
            bleed: 0.0,
            merge_fields: true,
            decoder_matrix: None,
            palette: None,
            base_palette: None,
        }
    }

    pub fn rgb() -> Self {
        NesNtscSetup {
            hue: 0.0,
            saturation: 0.0,
            contrast: 0.0,
            brightness: 0.0,
            sharpness: 0.2,
            gamma: 0.0,
            resolution: 0.7,
            artifacts: -1.0,
            fringing: -1.0,
            bleed: -1.0,
            merge_fields: true,
            decoder_matrix: None,
            palette: None,
            base_palette: None,
        }
    }

    /// An RGB palette can be generated for use in a normal blitter. It's 512
    /// colors (1536 bytes).
    pub fn generate_palette(&self) -> [u8; 1536] {
        let mut buf = [0; 1536];
        NesNtsc::new_with_palette(self, Some(&mut buf));

        buf
    }
}

const NES_NTSC_EMPHASIS: bool = true;
const NES_NTSC_PALETTE_SIZE: u32 = if NES_NTSC_EMPHASIS { 64 * 8 } else { 64 };
const NES_NTSC_ENTRY_SIZE: u32 = 128;

pub type NesNtscRgb = u64;

pub struct NesNtsc {
    table: Box<[Box<[NesNtscRgb]>]>,
}

const ALIGNMENT_COUNT: u32 = 3;
const BURST_COUNT: u32 = 3;
const RESCALE_IN: u32 = 8;
const RESCALE_OUT: u32 = 7;

const ARTIFACTS_MID: f32 = 1.0;
const ARTIFACTS_MAX: f32 = ARTIFACTS_MID * 1.5;
const FRINGING_MID: f32 = 1.0;
const FRINGING_MAX: f32 = FRINGING_MID * 2.0;
const STD_DECODER_HUE: i32 = -15;

const LUMA_CUTOFF: f32 = 0.2;
const GAMMA_SIZE: u32 = 1;
const RGB_BITS: u32 = 8;
const EXT_DECODER_HUE: i32 = STD_DECODER_HUE + 15;
const RGB_UNIT: u32 = 1 << RGB_BITS;
const RGB_OFFSET: f32 = (RGB_UNIT * 2) as f32 + 0.5;

const BURST_SIZE: u32 = NES_NTSC_ENTRY_SIZE / BURST_COUNT;
const KERNEL_HALF: i32 = 16;
const KERNEL_SIZE: i32 = KERNEL_HALF * 2 + 1;

const DEFAULT_PALETTE_CONTRAST: Option<f32> = None;

fn std_hue_condition(setup: &NesNtscSetup) -> bool {
    !(setup.base_palette.is_some() || setup.palette.is_some())
}

const DEFAULT_DECODER: [f32; 6] = [0.956, 0.621, -0.272, -0.647, -1.105, 1.702];

struct Init {
    to_rgb: [f32; BURST_COUNT as usize * 6],
    to_float: [f32; GAMMA_SIZE as usize],
    contrast: f32,
    brightness: f32,
    artifacts: f32,
    fringing: f32,
    kernel: [f32; (RESCALE_OUT * KERNEL_SIZE as u32 * 2) as usize],
}

impl Init {
    fn new(setup: &NesNtscSetup) -> Self {
        let mut init = Init {
            to_rgb: [0.0; 18],
            to_float: [0.0; 1],
            contrast: 0.0,
            brightness: 0.0,
            artifacts: 0.0,
            fringing: 0.0,
            kernel: [0.0; 462],
        };

        init.brightness = setup.brightness as f32 * (0.5 * RGB_UNIT as f32) + RGB_OFFSET;
        init.contrast = setup.contrast as f32 * (0.5 * RGB_UNIT as f32) + RGB_UNIT as f32;

        if let Some(contrast) = DEFAULT_PALETTE_CONTRAST {
            if setup.palette.is_none() {
                init.contrast *= contrast;
            }
        }

        init.artifacts = setup.artifacts as f32;
        if init.artifacts > 0.0 {
            init.artifacts *= ARTIFACTS_MAX - ARTIFACTS_MID;
        }
        init.artifacts = init.artifacts * ARTIFACTS_MID + ARTIFACTS_MID;

        init.fringing = setup.fringing as f32;
        if init.fringing > 0.0 {
            init.fringing *= FRINGING_MAX - FRINGING_MID;
        }
        init.fringing = init.fringing * FRINGING_MID + FRINGING_MID;

        init_filters(&mut init, setup);

        if GAMMA_SIZE > 1 {
            let to_float = 1.0 / (GAMMA_SIZE - (GAMMA_SIZE > 1) as u32) as f32;
            let gamma = 1.1333 - setup.gamma as f32 * 0.5;

            for (idx, f) in init.to_float.iter_mut().enumerate() {
                *f = (idx as f32 * to_float).powf(gamma) * init.contrast + init.brightness;
            }
        }

        const PI: f32 = std::f32::consts::PI;

        let mut hue = setup.hue as f32 * PI + PI / 180.0 * EXT_DECODER_HUE as f32;
        let sat = setup.saturation as f32 + 1.0;
        let decoder = if let Some(decoder) = setup.decoder_matrix.as_ref() {
            decoder
        } else {
            if std_hue_condition(setup) {
                hue += PI / 180.0 * (STD_DECODER_HUE as f32 - EXT_DECODER_HUE as f32);
            }
            &DEFAULT_DECODER
        };

        let mut s = hue.sin() * sat;
        let mut c = hue.cos() * sat;
        let mut out_idx = 0;
        for _ in 0..BURST_COUNT {
            let mut in_idx = 0;
            for _ in 0..3 {
                let i = decoder[in_idx];
                in_idx += 1;
                let q = decoder[in_idx];
                in_idx += 1;

                init.to_rgb[out_idx] = i * c - q * s;
                out_idx += 1;
                init.to_rgb[out_idx] = i * s + q * c;
                out_idx += 1;
            }
            if BURST_COUNT <= 1 {
                break;
            }
            (s, c) = rotate_iq(s, c, 0.866025, -0.5);
        }

        init
    }
}

fn init_filters(init: &mut Init, setup: &NesNtscSetup) {
    const PI: f32 = std::f32::consts::PI;
    // rescale_out is defined at 7 for NES
    let mut kernels = [0.0; KERNEL_SIZE as usize * 2];

    let rolloff = 1.0 + setup.sharpness as f32 * 0.032;
    let maxh = 32.0;
    let pow_a_n = rolloff.powf(maxh);

    let mut to_angle = setup.resolution as f32 + 1.0;
    to_angle = PI / maxh * LUMA_CUTOFF * (to_angle * to_angle + 1.0);
    kernels[KERNEL_SIZE as usize * 3 / 2] = maxh;
    for i in 0..KERNEL_HALF * 2 + 1 {
        let x = i - KERNEL_HALF;
        let angle = x as f32 * to_angle;

        if x != 0 || pow_a_n > 1.056 || pow_a_n < 0.981 {
            let rolloff_cos_a = rolloff * angle.cos();
            let num = 1.0 - rolloff_cos_a - pow_a_n * (maxh * angle).cos()
                + pow_a_n * rolloff * ((maxh - 1.0) * angle).cos();
            let den = 1.0 - rolloff_cos_a - rolloff_cos_a + rolloff * rolloff;
            let dsf = num / den;
            kernels[(KERNEL_SIZE * 3 / 2 - KERNEL_HALF + i) as usize] = dsf - 0.5;
        }
    }

    let mut sum = 0.0;
    for i in 0..KERNEL_HALF * 2 + 1 {
        let x = PI * 2.0 / (KERNEL_HALF * 2) as f32 * i as f32;
        let blackman = 0.42 - 0.5 * x.cos() + 0.08 * (x * 2.0).cos();
        let idx = (KERNEL_SIZE * 3 / 2 - KERNEL_HALF + i) as usize;
        kernels[idx] *= blackman;
        sum += kernels[idx];
    }

    sum = 1.0 / sum;
    for i in 0..KERNEL_HALF * 2 + 1 {
        let x = (KERNEL_SIZE * 3 / 2 - KERNEL_HALF + i) as usize;
        kernels[x] *= sum;
        assert!(!kernels[x].is_nan());
    }

    let cutoff_factor = -0.03125;
    let mut cutoff = setup.bleed as f32;

    if cutoff < 0.0 {
        cutoff *= cutoff;
        cutoff *= cutoff;
        cutoff *= cutoff;
        cutoff *= -30.0 / 0.65;
    }
    cutoff = cutoff_factor - 0.65 * cutoff_factor * cutoff;

    for i in -KERNEL_HALF..=KERNEL_HALF {
        let idx = (KERNEL_SIZE / 2 + i) as usize;
        let i = i as f32;
        kernels[idx] = (i * i * cutoff).exp();
    }

    for i in 0..2 {
        let mut sum = 0.0;
        for x in (i..KERNEL_SIZE).step_by(2) {
            sum += kernels[x as usize];
        }

        sum = 1.0 / sum;
        for x in (i..KERNEL_SIZE).step_by(2) {
            kernels[x as usize] *= sum;
            assert!(!kernels[x as usize].is_nan());
        }
    }

    let mut weight = 1.0;
    let mut out_idx = 0;
    for _ in 0..RESCALE_OUT {
        let mut remain = 0.0;
        weight -= 1.0 / RESCALE_IN as f32;
        for i in 0..KERNEL_SIZE * 2 {
            let cur = kernels[i as usize];
            let m = cur * weight;
            init.kernel[out_idx] = m + remain;
            out_idx += 1;
            remain = cur - m;
        }
    }
}

fn rotate_iq(i: f32, mut q: f32, sin_b: f32, cos_b: f32) -> (f32, f32) {
    let t = i * cos_b - q * sin_b;
    q = i * sin_b + q * cos_b;
    (t, q)
}

impl NesNtsc {
    /// Initializes and adjusts parameters. Can be called multiple times on the same
    /// nes_ntsc_t object. Can pass NULL for either parameter. */
    pub fn new(setup: &NesNtscSetup) -> Self {
        Self::new_with_palette(setup, None)
    }

    /// Number of output pixels written by blitter for given input width. Width might
    /// be rounded down slightly; use NES_NTSC_IN_WIDTH() on result to find rounded
    /// value. Guaranteed not to round 256 down at all. */
    pub const fn out_width(in_width: u32) -> u32 {
        ((in_width - 1) / NES_NTSC_IN_CHUNK + 1) * NES_NTSC_OUT_CHUNK
    }

    /// Number of input pixels that will fit within given output width. Might be
    /// rounded down slightly; use NES_NTSC_OUT_WIDTH() on result to find rounded
    /// value.
    pub const fn in_width(out_width: u32) -> u32 {
        (out_width / NES_NTSC_OUT_CHUNK - 1) * NES_NTSC_IN_CHUNK + 1
    }

    fn new_with_palette<'a>(setup: &NesNtscSetup, mut palette_out: Option<&'a mut [u8]>) -> Self {
        let mut ntsc = NesNtsc {
            table: vec![
                vec![0; NES_NTSC_ENTRY_SIZE as usize].into_boxed_slice();
                NES_NTSC_PALETTE_SIZE as usize
            ]
            .into_boxed_slice(),
        };
        let init = Init::new(setup);

        let mut gamma = setup.gamma as f32 * -0.5;
        if std_hue_condition(setup) {
            gamma += 0.1333;
        }

        let mut gamma_factor = gamma.abs().powf(0.73);
        if gamma < 0.0 {
            gamma_factor = -gamma_factor;
        }

        let merge_fields = if setup.artifacts <= -1.0 && setup.fringing <= -1.0 {
            true
        } else {
            setup.merge_fields
        };

        for entry in 0..NES_NTSC_PALETTE_SIZE {
            const LO_LEVELS: [f32; 4] = [-0.12, 0.0, 0.31, 0.72];
            const HI_LEVEL: [f32; 4] = [0.4, 0.68, 1.0, 1.0];
            let level = entry >> 4 & 0x03;
            let mut lo = LO_LEVELS[level as usize];
            let mut hi = HI_LEVEL[level as usize];

            let color = (entry & 0x0f) as u8;
            if color == 0 {
                lo = hi;
            } else if color == 0x0d {
                hi = lo;
            } else if color > 0x0d {
                hi = 0.0;
                lo = 0.0;
            }

            const PHASES: [f32; 0x10 + 3] = [
                -1.0, -0.866025, -0.5, 0.0, 0.5, 0.866025, 1.0, 0.866025, 0.5, 0.0, -0.5,
                -0.866025, -1.0, -0.866025, -0.5, 0.0, 0.5, 0.866025, 1.0,
            ];

            let to_angle_sin = |color| PHASES[color as usize];

            let to_angle_cos = |color| PHASES[color as usize + 3];

            let sat = (hi - lo) * 0.5;
            let mut i = to_angle_sin(color) * sat;
            let mut q = to_angle_cos(color) * sat;
            let mut y = (hi + lo) * 0.5;

            if NES_NTSC_EMPHASIS {
                let tint = entry >> 6 & 7;
                if tint != 0 && color <= 0x0d {
                    const ATTEN_MUL: f32 = 0.79399;
                    const ATTEN_SUB: f32 = 0.0782838;

                    if tint == 7 {
                        y = y * (ATTEN_MUL * 1.13) - (ATTEN_SUB * 1.13);
                    } else {
                        const TINTS: [u8; 8] = [0, 6, 10, 8, 2, 4, 0, 0];
                        let tint_color = TINTS[tint as usize];
                        let mut sat = hi * (0.5 - ATTEN_MUL * 0.5) + ATTEN_SUB * 0.5;
                        y -= sat * 0.5;
                        if tint >= 3 && tint != 4 {
                            sat *= 0.6;
                            y -= sat;
                        }
                        i += to_angle_sin(tint_color) * sat;
                        q += to_angle_cos(tint_color) * sat;
                    }
                }
            }

            if let Some(palette) = setup.palette.as_ref() {
                let in_idx = entry as usize * 3;
                const TO_FLOAT: f32 = 1.0 / 0xff as f32;
                let r = palette[in_idx] as f32 * TO_FLOAT;
                let g = palette[in_idx + 1] as f32 * TO_FLOAT;
                let b = palette[in_idx + 2] as f32 * TO_FLOAT;

                (y, i, q) = rgb_to_yiq(r, g, b);
            }

            y *= setup.contrast as f32 * 0.5 + 1.0;
            y += setup.brightness as f32 * 0.5 - 0.5 / 256.0;
            let (mut r, mut g, mut b) = yiq_to_rgb::<f32>(y, i, q, DEFAULT_DECODER.as_slice());
            r = (r * gamma_factor - gamma_factor) * r + r;
            g = (g * gamma_factor - gamma_factor) * g + g;
            b = (b * gamma_factor - gamma_factor) * b + b;

            (y, i, q) = rgb_to_yiq(r, g, b);

            i *= RGB_UNIT as f32;
            q *= RGB_UNIT as f32;
            y *= RGB_UNIT as f32;
            y += RGB_OFFSET;

            let (r, g, b) = yiq_to_rgb::<i32>(y, i, q, init.to_rgb.as_slice());

            let rgb = pack_rgb(r, g, b.min(0x3e0));

            if let Some(palette_out) = palette_out.as_mut() {
                rgb_palette_out(rgb, &mut palette_out[entry as usize * 3..]);
            }

            let kernel = ntsc.table[entry as usize].as_mut();
            gen_kernel(&init, y, i, q, kernel);
            if merge_fields {
                merge_kernel_fields(kernel);
            }
            correct_errors(rgb, kernel);
        }

        ntsc
    }

    /// Filters one or more rows of pixels. Input pixels are 6/9-bit palette indicies.
    /// In_row_width is the number of pixels to get to the next input row. Out_pitch
    /// is the number of *bytes* to get to the next output row. Output pixel format
    /// is set by NES_NTSC_OUT_DEPTH (defaults to 16-bit RGB). */
    pub fn blit(
        &self,
        nes_input: &[u16],
        rgb_output: &mut [u32],
        in_width: usize,
        in_height: usize,
        burst_phase: i32,
    ) {
        let mut blitter = Blitter::new(
            self,
            nes_input,
            rgb_output,
            in_width,
            in_height,
            burst_phase,
        );
        blitter.blit();
    }
}

struct PixelInfo {
    offset: i32,
    negate: f32,
    kernel: [f32; 4],
}

const NES_NTSC_PIXELS: [PixelInfo; ALIGNMENT_COUNT as usize] = [
    ntsc_pixel_info(-4, -9, [1.0, 1.0, 0.6667, 0.0]),
    ntsc_pixel_info(-2, -7, [0.3333, 1.0, 1.0, 0.3333]),
    ntsc_pixel_info(0, -5, [0.0, 0.6667, 1.0, 1.0]),
];

const fn pixel_offset(ntsc: i32, scaled: i32) -> (i32, f32) {
    let a = if RESCALE_IN > 1 {
        let rescale_in = RESCALE_IN as i32;
        let rescale_out = RESCALE_OUT as i32;
        let ntsc = ntsc - scaled / rescale_out * rescale_in;
        let scaled = (scaled + rescale_out * 10) % rescale_out;

        KERNEL_SIZE / 2
            + ntsc
            + (scaled != 0) as i32
            + (rescale_out - scaled) % rescale_out
            + (KERNEL_SIZE * 2 * scaled)
    } else {
        KERNEL_SIZE / 2 + ntsc - scaled
    };

    let b = 1 - ((ntsc + 100) & 2);

    (a, b as f32)
}

const fn ntsc_pixel_info(ntsc: i32, scaled: i32, kernel: [f32; 4]) -> PixelInfo {
    let (offset, negate) = pixel_offset(ntsc, scaled);
    PixelInfo {
        offset,
        negate,
        kernel,
    }
}

fn merge_kernel_fields(io: &mut [NesNtscRgb]) {
    let mut io_idx = 0;
    let burst_size = BURST_SIZE as usize;
    let mut n = BURST_SIZE;

    loop {
        let io = &mut io[io_idx..];
        let p0 = io[burst_size * 0].wrapping_add(RGB_BIAS);
        let p1 = io[burst_size * 1].wrapping_add(RGB_BIAS);
        let p2 = io[burst_size * 2].wrapping_add(RGB_BIAS);

        io[burst_size * 0] = ((p0.wrapping_add(p1) - ((p0 ^ p1) & NES_NTSC_RGB_BUILDER)) >> 1)
            .wrapping_sub(RGB_BIAS);
        io[burst_size * 1] = ((p1.wrapping_add(p2) - ((p1 ^ p2) & NES_NTSC_RGB_BUILDER)) >> 1)
            .wrapping_sub(RGB_BIAS);
        io[burst_size * 2] = ((p2.wrapping_add(p0) - ((p2 ^ p0) & NES_NTSC_RGB_BUILDER)) >> 1)
            .wrapping_sub(RGB_BIAS);

        io_idx += 1;

        n -= 1;
        if n == 0 {
            break;
        }
    }
}

fn correct_errors(color: NesNtscRgb, out: &mut [NesNtscRgb]) {
    let mut out_idx = 0;
    let mut n = BURST_COUNT;

    loop {
        let out = &mut out[out_idx..];

        for i in 0..(RGB_KERNEL_SIZE / 2) as usize {
            //
            let error = color
                .wrapping_sub(out[i])
                .wrapping_sub(out[(i + 12) % 14 + 14])
                .wrapping_sub(out[(i + 10) % 14 + 28])
                .wrapping_sub(out[i + 7])
                .wrapping_sub(out[i + 5 + 14])
                .wrapping_sub(out[i + 3 + 28]);

            distribute_error(i + 3 + 28, i + 5 + 14, i + 7, error, i, out);
        }

        out_idx += (ALIGNMENT_COUNT * RGB_KERNEL_SIZE) as usize;

        n -= 1;
        if n == 0 {
            break;
        }
    }
}

fn distribute_error(
    a: usize,
    b: usize,
    c: usize,
    error: NesNtscRgb,
    i: usize,
    out: &mut [NesNtscRgb],
) {
    let mut fourth = (error + 2 * NES_NTSC_RGB_BUILDER) >> 2;
    fourth &= (RGB_BIAS >> 1) - NES_NTSC_RGB_BUILDER;
    fourth = fourth.wrapping_sub(RGB_BIAS >> 2);
    out[a] = out[a].wrapping_add(fourth);
    out[b] = out[b].wrapping_add(fourth);
    out[c] = out[c].wrapping_add(fourth);
    out[i] = out[i].wrapping_add(error.wrapping_sub(fourth.wrapping_mul(3)));
}

const RGB_KERNEL_SIZE: u32 = BURST_SIZE / ALIGNMENT_COUNT;
const RGB_BIAS: u64 = RGB_UNIT as u64 * 2 * NES_NTSC_RGB_BUILDER;

fn gen_kernel(init: &Init, mut y: f32, mut i: f32, mut q: f32, out: &mut [NesNtscRgb]) {
    let mut out_idx = 0;
    let to_rgb = init.to_rgb.as_slice();
    let mut to_rgb_idx = 0;

    y -= RGB_OFFSET;
    for _ in 0..BURST_COUNT {
        let mut pixel_idx = 0;
        for _ in 0..ALIGNMENT_COUNT {
            let pixel = &NES_NTSC_PIXELS[pixel_idx];
            let yy = y * init.fringing * pixel.negate;
            let ic0 = (i + yy) * pixel.kernel[0];
            let qc1 = (q + yy) * pixel.kernel[1];
            let ic2 = (i - yy) * pixel.kernel[2];
            let qc3 = (q - yy) * pixel.kernel[3];

            let factor = init.artifacts * pixel.negate;
            let ii = i * factor;
            let yc0 = (y + ii) * pixel.kernel[0];
            let yc2 = (y - ii) * pixel.kernel[2];

            let qq = q * factor;
            let yc1 = (y + qq) * pixel.kernel[1];
            let yc3 = (y - qq) * pixel.kernel[3];

            let mut k_idx = pixel.offset as usize;
            pixel_idx += 1;
            for _ in 0..RGB_KERNEL_SIZE {
                let k = &init.kernel[k_idx..];
                let i = k[0] * ic0 + k[2] * ic2;
                let q = k[1] * qc1 + k[3] * qc3;
                let kernel_size = KERNEL_SIZE as usize;
                let y = k[kernel_size] * yc0
                    + k[kernel_size + 1] * yc1
                    + k[kernel_size + 2] * yc2
                    + k[kernel_size + 3] * yc3
                    + RGB_OFFSET;

                if RESCALE_OUT <= 1 {
                    k_idx -= 1;
                } else if k_idx < kernel_size * 2 * (RESCALE_OUT as usize - 1) {
                    k_idx += kernel_size * 2 - 1;
                } else {
                    k_idx -= kernel_size * 2 * (RESCALE_OUT as usize - 1) + 2;
                }

                let (r, g, b) = yiq_to_rgb::<i32>(y, i, q, &to_rgb[to_rgb_idx..]);

                out[out_idx] = pack_rgb(r, g, b).wrapping_sub(RGB_BIAS);
                out_idx += 1;
            }
        }

        if BURST_COUNT <= 1 {
            break;
        }

        to_rgb_idx += 6;

        (i, q) = rotate_iq(i, q, -0.866025, -0.5);
    }
}

fn rgb_to_yiq(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let y = r * 0.299 + g * 0.587 + b * 0.114;
    let i = r * 0.596 - g * 0.275 - b * 0.321;
    let q = r * 0.212 - g * 0.523 + b * 0.311;

    (y, i, q)
}

fn yiq_to_rgb<T: FromFloat>(y: f32, i: f32, q: f32, to_rgb: &[f32]) -> (T, T, T) {
    let r = y + to_rgb[0] * i + to_rgb[1] * q;
    let g = y + to_rgb[2] * i + to_rgb[3] * q;
    let b = y + to_rgb[4] * i + to_rgb[5] * q;

    (T::from_f32(r), T::from_f32(g), T::from_f32(b))
}

trait FromFloat {
    fn from_f32(value: f32) -> Self;
}

impl FromFloat for i32 {
    fn from_f32(value: f32) -> Self {
        value as i32
    }
}

impl FromFloat for f32 {
    fn from_f32(value: f32) -> Self {
        value
    }
}

fn pack_rgb(r: i32, g: i32, b: i32) -> NesNtscRgb {
    let r = r as NesNtscRgb;
    let g = g as NesNtscRgb;
    let b = b as NesNtscRgb;

    r << 21 | g << 11 | b << 1
}

fn rgb_palette_out(rgb: NesNtscRgb, out: &mut [u8]) {
    let rgb = nes_ntsc_clamp(rgb, 8 - RGB_BITS);
    out[0] = (rgb >> 21) as u8;
    out[1] = (rgb >> 11) as u8;
    out[2] = (rgb >> 1) as u8;
}

const NES_NTSC_RGB_BUILDER: NesNtscRgb = 1 << 21 | 1 << 11 | 1 << 1;
const NES_NTSC_CLAMP_MASK: NesNtscRgb = NES_NTSC_RGB_BUILDER * 3 / 2;
const NES_NTSC_CLAMP_ADD: NesNtscRgb = NES_NTSC_RGB_BUILDER * 0x101;

fn nes_ntsc_clamp(mut io: NesNtscRgb, bits: u32) -> NesNtscRgb {
    let sub = io >> (9 - bits) & NES_NTSC_CLAMP_MASK;
    let mut clamp = NES_NTSC_CLAMP_ADD - sub;
    io |= clamp;
    clamp -= sub;
    io &= clamp;

    io
}

const NES_NTSC_IN_CHUNK: u32 = 3;
const NES_NTSC_OUT_CHUNK: u32 = 7;
const NES_NTSC_BLACK: u16 = 15;
const NES_NTSC_BURST_COUNT: u32 = 3;
const NES_NTSC_BURST_SIZE: u32 = NES_NTSC_ENTRY_SIZE / NES_NTSC_BURST_COUNT;

pub struct Blitter<'a> {
    ntsc: &'a NesNtsc,
    input: &'a [u16],
    output: &'a mut [u32],
    in_width: usize,
    in_height: usize,
    burst_phase: i32,
    kernel: [usize; 3],
    kernelx: [usize; 3],
    burst_offset: usize,
}

impl<'a> Blitter<'a> {
    pub fn new(
        ntsc: &'a NesNtsc,
        input: &'a [u16],
        output: &'a mut [u32],
        in_width: usize,
        in_height: usize,
        burst_phase: i32,
    ) -> Self {
        Self {
            ntsc,
            input,
            in_width,
            in_height,
            burst_phase,
            output,
            kernel: [0; 3],
            kernelx: [0; 3],
            burst_offset: 0,
        }
    }

    pub fn blit(&'a mut self) {
        let chunk_count = (self.in_width - 1) / NES_NTSC_IN_CHUNK as usize;
        for _ in 0..self.in_height {
            self.begin_row(NES_NTSC_BLACK, NES_NTSC_BLACK, self.input[0]);
            self.input = &self.input[1..];

            for _ in 0..chunk_count {
                self.color_in(0, self.input[0]);
                self.rgb_out(0);
                self.rgb_out(1);

                self.color_in(1, self.input[1]);
                self.rgb_out(2);
                self.rgb_out(3);

                self.color_in(2, self.input[2]);
                self.rgb_out(4);
                self.rgb_out(5);
                self.rgb_out(6);

                self.input = &self.input[3..];
                self.output = &mut self.output[7..];
            }

            self.color_in(0, NES_NTSC_BLACK);
            self.rgb_out(0);
            self.rgb_out(1);

            self.color_in(1, NES_NTSC_BLACK);
            self.rgb_out(2);
            self.rgb_out(3);

            self.color_in(2, NES_NTSC_BLACK);
            self.rgb_out(4);
            self.rgb_out(5);
            self.rgb_out(6);

            self.output = &mut self.output[7..];
            self.burst_phase = (self.burst_phase + 1) % NES_NTSC_BURST_COUNT as i32;
        }
    }

    fn begin_row(&mut self, pixel0: u16, pixel1: u16, pixel2: u16) {
        self.burst_offset = self.burst_phase as usize * NES_NTSC_BURST_SIZE as usize;
        self.kernel[0] = pixel0 as usize;
        self.kernel[1] = pixel1 as usize;
        self.kernel[2] = pixel2 as usize;

        self.kernelx[0] = 0;
        self.kernelx[1] = self.kernel[0];
        self.kernelx[2] = self.kernel[0];
    }

    fn color_in(&mut self, index: usize, color: u16) {
        self.kernelx[index] = self.kernel[index];
        self.kernel[index] = color as usize;
    }

    fn rgb_out(&mut self, index: usize) {
        let k = |entry: usize, x: usize| self.ntsc.table[entry][x + self.burst_offset];
        let mut raw = k(self.kernel[0], index)
            + k(self.kernel[1], (index + 12) % 7 + 14)
            + k(self.kernel[2], (index + 10) % 7 + 28)
            + k(self.kernelx[0], (index + 7) % 14)
            + k(self.kernelx[1], (index + 5) % 7 + 21)
            + k(self.kernelx[2], (index + 3) % 7 + 35);

        let sub = raw >> 9 & NES_NTSC_CLAMP_MASK;
        let mut clamp = NES_NTSC_CLAMP_ADD - sub;

        raw |= clamp;
        clamp -= sub;
        raw &= clamp;

        self.output[index] =
            ((raw >> 5 & 0xff0000) | (raw >> 3 & 0xff00) | (raw >> 1 & 0xff)) as u32;
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
    #[test]
    fn compare_impls() {
        let mut control_palette = vec![0; 1536];
        let ntsc = unsafe {
            let mut setup = nes_ntsc_sys::nes_ntsc_composite;
            setup.palette_out = control_palette.as_mut_ptr();
            let mut ntsc = std::mem::zeroed::<nes_ntsc_sys::nes_ntsc_t>();
            nes_ntsc_sys::nes_ntsc_init((&mut ntsc) as *mut _, &setup as *const _);
            ntsc
        };

        let mut new_setup = super::NesNtscSetup::composite();
        let palette_out = new_setup.generate_palette();
        let new_ntsc = super::NesNtsc::new(&mut new_setup);

        for p in control_palette.iter().zip(&palette_out) {
            assert_eq!(p.0, p.1);
        }

        let mut same = 0;
        let mut diff = 0;

        let mut len_a = 0;
        let mut len_b = 0;

        for _ in ntsc.table.as_flattened() {
            len_a += 1;
        }

        for _ in new_ntsc.table.iter().flatten() {
            len_b += 1;
        }

        assert_eq!(len_a, len_b);

        for (k, kk) in ntsc
            .table
            .as_flattened()
            .iter()
            .copied()
            .zip(new_ntsc.table.iter().flatten().copied())
        {
            if k == kk {
                same += 1;
            } else {
                diff += 1;
            }
        }

        eprintln!("same: {same}  diff: {diff}");

        assert!(diff == 0);
    }
}
