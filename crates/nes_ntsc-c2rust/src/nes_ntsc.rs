#![allow(dead_code, mutable_transmutes, non_camel_case_types, non_snake_case, non_upper_case_globals, unused_assignments, unused_mut, path_statements, static_mut_refs, unused_labels)]

use std::ffi as libc;
fn cos(x: libc::c_double) -> libc::c_double {
    x.cos()
}

fn sin(x: libc::c_double) -> libc::c_double {
    x.sin()
}

fn exp(x: libc::c_double) -> libc::c_double {
    x.exp()
}

fn pow(x: libc::c_double, n: libc::c_double) -> libc::c_double {
    x.powf(n)
}

fn fabs(x: libc::c_double) -> libc::c_double {
    x.abs()
}

fn __assert_fail(
    __assertion: *const libc::c_char,
    __file: *const libc::c_char,
    __line: libc::c_uint,
    __function: *const libc::c_char,
) -> ! {
    panic!()
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct nes_ntsc_setup_t {
    pub hue: libc::c_double,
    pub saturation: libc::c_double,
    pub contrast: libc::c_double,
    pub brightness: libc::c_double,
    pub sharpness: libc::c_double,
    pub gamma: libc::c_double,
    pub resolution: libc::c_double,
    pub artifacts: libc::c_double,
    pub fringing: libc::c_double,
    pub bleed: libc::c_double,
    pub merge_fields: libc::c_int,
    pub decoder_matrix: *const libc::c_float,
    pub palette_out: *mut libc::c_uchar,
    pub palette: *const libc::c_uchar,
    pub base_palette: *const libc::c_uchar,
}
pub type C2RustUnnamed = libc::c_uint;
pub const nes_ntsc_palette_size: C2RustUnnamed = 512;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct nes_ntsc_t {
    pub table: [[nes_ntsc_rgb_t; 128]; 512],
}
pub type nes_ntsc_rgb_t = libc::c_ulong;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct init_t {
    pub to_rgb: [libc::c_float; 18],
    pub to_float: [libc::c_float; 1],
    pub contrast: libc::c_float,
    pub brightness: libc::c_float,
    pub artifacts: libc::c_float,
    pub fringing: libc::c_float,
    pub kernel: [libc::c_float; 462],
}
pub const rgb_kernel_size: C2RustUnnamed_8 = 14;
pub const rgb_bias: C2RustUnnamed_9 = 1074791424;
pub const burst_size: C2RustUnnamed_5 = 42;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pixel_info_t {
    pub offset: libc::c_int,
    pub negate: libc::c_float,
    pub kernel: [libc::c_float; 4],
}
pub const kernel_size: C2RustUnnamed_7 = 33;
pub const kernel_half: C2RustUnnamed_6 = 16;
pub const nes_ntsc_burst_count: C2RustUnnamed_2 = 3;
pub type nes_ntsc_out_t = libc::c_uint;
pub const nes_ntsc_entry_size: C2RustUnnamed_3 = 128;
pub const nes_ntsc_black: C2RustUnnamed_1 = 15;
pub const nes_ntsc_burst_size: C2RustUnnamed_4 = 42;
pub const nes_ntsc_in_chunk: C2RustUnnamed_0 = 3;
pub type C2RustUnnamed_0 = libc::c_uint;
pub type C2RustUnnamed_1 = libc::c_uint;
pub type C2RustUnnamed_2 = libc::c_uint;
pub type C2RustUnnamed_3 = libc::c_uint;
pub type C2RustUnnamed_4 = libc::c_uint;
pub type C2RustUnnamed_5 = libc::c_uint;
pub type C2RustUnnamed_6 = libc::c_uint;
pub type C2RustUnnamed_7 = libc::c_uint;
pub type C2RustUnnamed_8 = libc::c_uint;
pub type C2RustUnnamed_9 = libc::c_uint;
#[no_mangle]
pub static mut nes_ntsc_monochrome: nes_ntsc_setup_t = {
    let mut init = nes_ntsc_setup_t {
        hue: 0 as libc::c_int as libc::c_double,
        saturation: -(1 as libc::c_int) as libc::c_double,
        contrast: 0 as libc::c_int as libc::c_double,
        brightness: 0 as libc::c_int as libc::c_double,
        sharpness: 0.2f64,
        gamma: 0 as libc::c_int as libc::c_double,
        resolution: 0.2f64,
        artifacts: -0.2f64,
        fringing: -0.2f64,
        bleed: -(1 as libc::c_int) as libc::c_double,
        merge_fields: 1 as libc::c_int,
        decoder_matrix: 0 as *const libc::c_float,
        palette_out: 0 as *const libc::c_uchar as *mut libc::c_uchar,
        palette: 0 as *const libc::c_uchar,
        base_palette: 0 as *const libc::c_uchar,
    };
    init
};
#[no_mangle]
pub static mut nes_ntsc_composite: nes_ntsc_setup_t = {
    let mut init = nes_ntsc_setup_t {
        hue: 0 as libc::c_int as libc::c_double,
        saturation: 0 as libc::c_int as libc::c_double,
        contrast: 0 as libc::c_int as libc::c_double,
        brightness: 0 as libc::c_int as libc::c_double,
        sharpness: 0 as libc::c_int as libc::c_double,
        gamma: 0 as libc::c_int as libc::c_double,
        resolution: 0 as libc::c_int as libc::c_double,
        artifacts: 0 as libc::c_int as libc::c_double,
        fringing: 0 as libc::c_int as libc::c_double,
        bleed: 0 as libc::c_int as libc::c_double,
        merge_fields: 1 as libc::c_int,
        decoder_matrix: 0 as *const libc::c_float,
        palette_out: 0 as *const libc::c_uchar as *mut libc::c_uchar,
        palette: 0 as *const libc::c_uchar,
        base_palette: 0 as *const libc::c_uchar,
    };
    init
};
#[no_mangle]
pub static mut nes_ntsc_svideo: nes_ntsc_setup_t = {
    let mut init = nes_ntsc_setup_t {
        hue: 0 as libc::c_int as libc::c_double,
        saturation: 0 as libc::c_int as libc::c_double,
        contrast: 0 as libc::c_int as libc::c_double,
        brightness: 0 as libc::c_int as libc::c_double,
        sharpness: 0.2f64,
        gamma: 0 as libc::c_int as libc::c_double,
        resolution: 0.2f64,
        artifacts: -(1 as libc::c_int) as libc::c_double,
        fringing: -(1 as libc::c_int) as libc::c_double,
        bleed: 0 as libc::c_int as libc::c_double,
        merge_fields: 1 as libc::c_int,
        decoder_matrix: 0 as *const libc::c_float,
        palette_out: 0 as *const libc::c_uchar as *mut libc::c_uchar,
        palette: 0 as *const libc::c_uchar,
        base_palette: 0 as *const libc::c_uchar,
    };
    init
};
#[no_mangle]
pub static mut nes_ntsc_rgb: nes_ntsc_setup_t = {
    let mut init = nes_ntsc_setup_t {
        hue: 0 as libc::c_int as libc::c_double,
        saturation: 0 as libc::c_int as libc::c_double,
        contrast: 0 as libc::c_int as libc::c_double,
        brightness: 0 as libc::c_int as libc::c_double,
        sharpness: 0.2f64,
        gamma: 0 as libc::c_int as libc::c_double,
        resolution: 0.7f64,
        artifacts: -(1 as libc::c_int) as libc::c_double,
        fringing: -(1 as libc::c_int) as libc::c_double,
        bleed: -(1 as libc::c_int) as libc::c_double,
        merge_fields: 1 as libc::c_int,
        decoder_matrix: 0 as *const libc::c_float,
        palette_out: 0 as *const libc::c_uchar as *mut libc::c_uchar,
        palette: 0 as *const libc::c_uchar,
        base_palette: 0 as *const libc::c_uchar,
    };
    init
};
static mut default_decoder: [libc::c_float; 6] = [
    0.956f32,
    0.621f32,
    -0.272f32,
    -0.647f32,
    -1.105f32,
    1.702f32,
];
unsafe extern "C" fn init_filters(
    mut impl_0: *mut init_t,
    mut setup: *const nes_ntsc_setup_t,
) {
    let mut kernels: [libc::c_float; 66] = [0.; 66];
    let rolloff: libc::c_float = 1 as libc::c_int as libc::c_float
        + (*setup).sharpness as libc::c_float * 0.032f64 as libc::c_float;
    let maxh: libc::c_float = 32 as libc::c_int as libc::c_float;
    let pow_a_n: libc::c_float = pow(rolloff as libc::c_double, maxh as libc::c_double)
        as libc::c_float;
    let mut sum: libc::c_float = 0.;
    let mut i: libc::c_int = 0;
    let mut to_angle: libc::c_float = (*setup).resolution as libc::c_float
        + 1 as libc::c_int as libc::c_float;
    to_angle = 3.14159265358979323846f32 / maxh * 0.20f64 as libc::c_float
        * (to_angle * to_angle + 1 as libc::c_int as libc::c_float);
    kernels[(kernel_size as libc::c_int * 3 as libc::c_int / 2 as libc::c_int)
        as usize] = maxh;
    i = 0 as libc::c_int;
    while i < kernel_half as libc::c_int * 2 as libc::c_int + 1 as libc::c_int {
        let mut x: libc::c_int = i - kernel_half as libc::c_int;
        let mut angle: libc::c_float = x as libc::c_float * to_angle;
        if x != 0 || pow_a_n > 1.056f64 as libc::c_float
            || pow_a_n < 0.981f64 as libc::c_float
        {
            let mut rolloff_cos_a: libc::c_float = rolloff
                * cos(angle as libc::c_double) as libc::c_float;
            let mut num: libc::c_float = 1 as libc::c_int as libc::c_float
                - rolloff_cos_a
                - pow_a_n * cos((maxh * angle) as libc::c_double) as libc::c_float
                + pow_a_n * rolloff
                    * cos(
                        ((maxh - 1 as libc::c_int as libc::c_float) * angle)
                            as libc::c_double,
                    ) as libc::c_float;
            let mut den: libc::c_float = 1 as libc::c_int as libc::c_float
                - rolloff_cos_a - rolloff_cos_a + rolloff * rolloff;
            let mut dsf: libc::c_float = num / den;
            kernels[(kernel_size as libc::c_int * 3 as libc::c_int / 2 as libc::c_int
                - kernel_half as libc::c_int + i)
                as usize] = dsf - 0.5f64 as libc::c_float;
        }
        i += 1;
        i;
    }
    sum = 0 as libc::c_int as libc::c_float;
    i = 0 as libc::c_int;
    while i < kernel_half as libc::c_int * 2 as libc::c_int + 1 as libc::c_int {
        let mut x_0: libc::c_float = 3.14159265358979323846f32
            * 2 as libc::c_int as libc::c_float
            / (kernel_half as libc::c_int * 2 as libc::c_int) as libc::c_float
            * i as libc::c_float;
        let mut blackman: libc::c_float = 0.42f32
            - 0.5f32 * cos(x_0 as libc::c_double) as libc::c_float
            + 0.08f32
                * cos((x_0 * 2 as libc::c_int as libc::c_float) as libc::c_double)
                    as libc::c_float;
        kernels[(kernel_size as libc::c_int * 3 as libc::c_int / 2 as libc::c_int
            - kernel_half as libc::c_int + i) as usize] *= blackman;
        sum
            += kernels[(kernel_size as libc::c_int * 3 as libc::c_int / 2 as libc::c_int
                - kernel_half as libc::c_int + i) as usize];
        i += 1;
        i;
    }
    sum = 1.0f32 / sum;
    i = 0 as libc::c_int;
    while i < kernel_half as libc::c_int * 2 as libc::c_int + 1 as libc::c_int {
        let mut x_1: libc::c_int = kernel_size as libc::c_int * 3 as libc::c_int
            / 2 as libc::c_int - kernel_half as libc::c_int + i;
        kernels[x_1 as usize] *= sum;
        if kernels[x_1 as usize] == kernels[x_1 as usize] {} else {
            __assert_fail(
                b"kernels [x] == kernels [x]\0" as *const u8 as *const libc::c_char,
                b"/home/nickmass/rust/mass-nes/crates/nes_ntsc-sys/nes_ntsc/nes_ntsc_impl.h\0"
                    as *const u8 as *const libc::c_char,
                122 as libc::c_int as libc::c_uint,
                (*::core::mem::transmute::<
                    &[u8; 54],
                    &[libc::c_char; 54],
                >(b"void init_filters(init_t *, const nes_ntsc_setup_t *)\0"))
                    .as_ptr(),
            );
        }
        'c_3427: {
            if kernels[x_1 as usize] == kernels[x_1 as usize] {} else {
                __assert_fail(
                    b"kernels [x] == kernels [x]\0" as *const u8 as *const libc::c_char,
                    b"/home/nickmass/rust/mass-nes/crates/nes_ntsc-sys/nes_ntsc/nes_ntsc_impl.h\0"
                        as *const u8 as *const libc::c_char,
                    122 as libc::c_int as libc::c_uint,
                    (*::core::mem::transmute::<
                        &[u8; 54],
                        &[libc::c_char; 54],
                    >(b"void init_filters(init_t *, const nes_ntsc_setup_t *)\0"))
                        .as_ptr(),
                );
            }
        };
        i += 1;
        i;
    }
    let cutoff_factor: libc::c_float = -0.03125f32;
    let mut cutoff: libc::c_float = (*setup).bleed as libc::c_float;
    let mut i_0: libc::c_int = 0;
    if cutoff < 0 as libc::c_int as libc::c_float {
        cutoff *= cutoff;
        cutoff *= cutoff;
        cutoff *= cutoff;
        cutoff *= -30.0f32 / 0.65f32;
    }
    cutoff = cutoff_factor - 0.65f32 * cutoff_factor * cutoff;
    i_0 = -(kernel_half as libc::c_int);
    while i_0 <= kernel_half as libc::c_int {
        kernels[(kernel_size as libc::c_int / 2 as libc::c_int + i_0)
            as usize] = exp(((i_0 * i_0) as libc::c_float * cutoff) as libc::c_double)
            as libc::c_float;
        i_0 += 1;
        i_0;
    }
    i_0 = 0 as libc::c_int;
    while i_0 < 2 as libc::c_int {
        let mut sum_0: libc::c_float = 0 as libc::c_int as libc::c_float;
        let mut x_2: libc::c_int = 0;
        x_2 = i_0;
        while x_2 < kernel_size as libc::c_int {
            sum_0 += kernels[x_2 as usize];
            x_2 += 2 as libc::c_int;
        }
        sum_0 = 1.0f32 / sum_0;
        x_2 = i_0;
        while x_2 < kernel_size as libc::c_int {
            kernels[x_2 as usize] *= sum_0;
            if kernels[x_2 as usize] == kernels[x_2 as usize] {} else {
                __assert_fail(
                    b"kernels [x] == kernels [x]\0" as *const u8 as *const libc::c_char,
                    b"/home/nickmass/rust/mass-nes/crates/nes_ntsc-sys/nes_ntsc/nes_ntsc_impl.h\0"
                        as *const u8 as *const libc::c_char,
                    157 as libc::c_int as libc::c_uint,
                    (*::core::mem::transmute::<
                        &[u8; 54],
                        &[libc::c_char; 54],
                    >(b"void init_filters(init_t *, const nes_ntsc_setup_t *)\0"))
                        .as_ptr(),
                );
            }
            'c_3198: {
                if kernels[x_2 as usize] == kernels[x_2 as usize] {} else {
                    __assert_fail(
                        b"kernels [x] == kernels [x]\0" as *const u8
                            as *const libc::c_char,
                        b"/home/nickmass/rust/mass-nes/crates/nes_ntsc-sys/nes_ntsc/nes_ntsc_impl.h\0"
                            as *const u8 as *const libc::c_char,
                        157 as libc::c_int as libc::c_uint,
                        (*::core::mem::transmute::<
                            &[u8; 54],
                            &[libc::c_char; 54],
                        >(b"void init_filters(init_t *, const nes_ntsc_setup_t *)\0"))
                            .as_ptr(),
                    );
                }
            };
            x_2 += 2 as libc::c_int;
        }
        i_0 += 1;
        i_0;
    }
    let mut weight: libc::c_float = 1.0f32;
    let mut out: *mut libc::c_float = ((*impl_0).kernel).as_mut_ptr();
    let mut n: libc::c_int = 7 as libc::c_int;
    loop {
        let mut remain: libc::c_float = 0 as libc::c_int as libc::c_float;
        let mut i_1: libc::c_int = 0;
        weight -= 1.0f32 / 8 as libc::c_int as libc::c_float;
        i_1 = 0 as libc::c_int;
        while i_1 < kernel_size as libc::c_int * 2 as libc::c_int {
            let mut cur: libc::c_float = kernels[i_1 as usize];
            let mut m: libc::c_float = cur * weight;
            let fresh0 = out;
            out = out.offset(1);
            *fresh0 = m + remain;
            remain = cur - m;
            i_1 += 1;
            i_1;
        }
        n -= 1;
        if !(n != 0) {
            break;
        }
    };
}
unsafe extern "C" fn init(mut impl_0: *mut init_t, mut setup: *const nes_ntsc_setup_t) {
    (*impl_0)
        .brightness = (*setup).brightness as libc::c_float
        * (0.5f32 * ((1 as libc::c_int) << 8 as libc::c_int) as libc::c_float)
        + ((((1 as libc::c_int) << 8 as libc::c_int) * 2 as libc::c_int) as libc::c_float
            + 0.5f32);
    (*impl_0)
        .contrast = (*setup).contrast as libc::c_float
        * (0.5f32 * ((1 as libc::c_int) << 8 as libc::c_int) as libc::c_float)
        + ((1 as libc::c_int) << 8 as libc::c_int) as libc::c_float;
    (*impl_0).artifacts = (*setup).artifacts as libc::c_float;
    if (*impl_0).artifacts > 0 as libc::c_int as libc::c_float {
        (*impl_0).artifacts *= 1.0f32 * 1.5f32 - 1.0f32;
    }
    (*impl_0).artifacts = (*impl_0).artifacts * 1.0f32 + 1.0f32;
    (*impl_0).fringing = (*setup).fringing as libc::c_float;
    if (*impl_0).fringing > 0 as libc::c_int as libc::c_float {
        (*impl_0).fringing *= 1.0f32 * 2 as libc::c_int as libc::c_float - 1.0f32;
    }
    (*impl_0).fringing = (*impl_0).fringing * 1.0f32 + 1.0f32;
    init_filters(impl_0, setup);
    if 1 as libc::c_int > 1 as libc::c_int {
        let to_float: libc::c_float = 1.0f32
            / (1 as libc::c_int - (1 as libc::c_int > 1 as libc::c_int) as libc::c_int)
                as libc::c_float;
        let gamma: libc::c_float = 1.1333f32 - (*setup).gamma as libc::c_float * 0.5f32;
        let mut i: libc::c_int = 0;
        i = 0 as libc::c_int;
        while i < 1 as libc::c_int {
            (*impl_0)
                .to_float[i
                as usize] = pow(
                (i as libc::c_float * to_float) as libc::c_double,
                gamma as libc::c_double,
            ) as libc::c_float * (*impl_0).contrast + (*impl_0).brightness;
            i += 1;
            i;
        }
    }
    let mut hue: libc::c_float = (*setup).hue as libc::c_float
        * 3.14159265358979323846f32
        + 3.14159265358979323846f32 / 180 as libc::c_int as libc::c_float
            * (-(15 as libc::c_int) + 15 as libc::c_int) as libc::c_float;
    let mut sat: libc::c_float = (*setup).saturation as libc::c_float
        + 1 as libc::c_int as libc::c_float;
    let mut decoder: *const libc::c_float = (*setup).decoder_matrix;
    if decoder.is_null() {
        decoder = default_decoder.as_ptr();
        if !(!((*setup).base_palette).is_null() || !((*setup).palette).is_null()) {
            hue
                += 3.14159265358979323846f32 / 180 as libc::c_int as libc::c_float
                    * (-(15 as libc::c_int) - (-(15 as libc::c_int) + 15 as libc::c_int))
                        as libc::c_float;
        }
    }
    let mut s: libc::c_float = sin(hue as libc::c_double) as libc::c_float * sat;
    let mut c: libc::c_float = cos(hue as libc::c_double) as libc::c_float * sat;
    let mut out: *mut libc::c_float = ((*impl_0).to_rgb).as_mut_ptr();
    let mut n: libc::c_int = 0;
    n = 3 as libc::c_int;
    loop {
        let mut in_0: *const libc::c_float = decoder;
        let mut n_0: libc::c_int = 3 as libc::c_int;
        loop {
            let fresh1 = in_0;
            in_0 = in_0.offset(1);
            let mut i_0: libc::c_float = *fresh1;
            let fresh2 = in_0;
            in_0 = in_0.offset(1);
            let mut q: libc::c_float = *fresh2;
            let fresh3 = out;
            out = out.offset(1);
            *fresh3 = i_0 * c - q * s;
            let fresh4 = out;
            out = out.offset(1);
            *fresh4 = i_0 * s + q * c;
            n_0 -= 1;
            if !(n_0 != 0) {
                break;
            }
        }
        if 3 as libc::c_int <= 1 as libc::c_int {
            break;
        }
        let mut t: libc::c_float = 0.;
        t = s * -0.5f32 - c * 0.866025f32;
        c = s * 0.866025f32 + c * -0.5f32;
        s = t;
        n -= 1;
        if !(n != 0) {
            break;
        }
    };
}
unsafe extern "C" fn gen_kernel(
    mut impl_0: *mut init_t,
    mut y: libc::c_float,
    mut i: libc::c_float,
    mut q: libc::c_float,
    mut out: *mut nes_ntsc_rgb_t,
) {
    let mut to_rgb: *const libc::c_float = ((*impl_0).to_rgb).as_mut_ptr();
    let mut burst_remain: libc::c_int = 3 as libc::c_int;
    y
        -= (((1 as libc::c_int) << 8 as libc::c_int) * 2 as libc::c_int) as libc::c_float
            + 0.5f32;
    loop {
        let mut pixel: *const pixel_info_t = nes_ntsc_pixels.as_ptr();
        let mut alignment_remain: libc::c_int = 3 as libc::c_int;
        loop {
            let yy: libc::c_float = y * (*impl_0).fringing * (*pixel).negate;
            let ic0: libc::c_float = (i + yy)
                * (*pixel).kernel[0 as libc::c_int as usize];
            let qc1: libc::c_float = (q + yy)
                * (*pixel).kernel[1 as libc::c_int as usize];
            let ic2: libc::c_float = (i - yy)
                * (*pixel).kernel[2 as libc::c_int as usize];
            let qc3: libc::c_float = (q - yy)
                * (*pixel).kernel[3 as libc::c_int as usize];
            let factor: libc::c_float = (*impl_0).artifacts * (*pixel).negate;
            let ii: libc::c_float = i * factor;
            let yc0: libc::c_float = (y + ii)
                * (*pixel).kernel[0 as libc::c_int as usize];
            let yc2: libc::c_float = (y - ii)
                * (*pixel).kernel[2 as libc::c_int as usize];
            let qq: libc::c_float = q * factor;
            let yc1: libc::c_float = (y + qq)
                * (*pixel).kernel[1 as libc::c_int as usize];
            let yc3: libc::c_float = (y - qq)
                * (*pixel).kernel[3 as libc::c_int as usize];
            let mut k: *const libc::c_float = &mut *((*impl_0).kernel)
                .as_mut_ptr()
                .offset((*pixel).offset as isize) as *mut libc::c_float;
            let mut n: libc::c_int = 0;
            pixel = pixel.offset(1);
            pixel;
            n = rgb_kernel_size as libc::c_int;
            while n != 0 {
                let mut i_0: libc::c_float = *k.offset(0 as libc::c_int as isize) * ic0
                    + *k.offset(2 as libc::c_int as isize) * ic2;
                let mut q_0: libc::c_float = *k.offset(1 as libc::c_int as isize) * qc1
                    + *k.offset(3 as libc::c_int as isize) * qc3;
                let mut y_0: libc::c_float = *k
                    .offset((kernel_size as libc::c_int + 0 as libc::c_int) as isize)
                    * yc0
                    + *k.offset((kernel_size as libc::c_int + 1 as libc::c_int) as isize)
                        * yc1
                    + *k.offset((kernel_size as libc::c_int + 2 as libc::c_int) as isize)
                        * yc2
                    + *k.offset((kernel_size as libc::c_int + 3 as libc::c_int) as isize)
                        * yc3
                    + ((((1 as libc::c_int) << 8 as libc::c_int) * 2 as libc::c_int)
                        as libc::c_float + 0.5f32);
                if 7 as libc::c_int <= 1 as libc::c_int {
                    k = k.offset(-1);
                    k;
                } else if k
                    < &mut *((*impl_0).kernel)
                        .as_mut_ptr()
                        .offset(
                            (kernel_size as libc::c_int * 2 as libc::c_int
                                * (7 as libc::c_int - 1 as libc::c_int)) as isize,
                        ) as *mut libc::c_float as *const libc::c_float
                {
                    k = k
                        .offset(
                            (kernel_size as libc::c_int * 2 as libc::c_int
                                - 1 as libc::c_int) as isize,
                        );
                } else {
                    k = k
                        .offset(
                            -((kernel_size as libc::c_int * 2 as libc::c_int
                                * (7 as libc::c_int - 1 as libc::c_int) + 2 as libc::c_int)
                                as isize),
                        );
                }
                let mut r: libc::c_int = 0;
                let mut g: libc::c_int = 0;
                r = (y_0 + *to_rgb.offset(0 as libc::c_int as isize) * i_0
                    + *to_rgb.offset(1 as libc::c_int as isize) * q_0) as libc::c_int;
                g = (y_0 + *to_rgb.offset(2 as libc::c_int as isize) * i_0
                    + *to_rgb.offset(3 as libc::c_int as isize) * q_0) as libc::c_int;
                let mut b: libc::c_int = (y_0
                    + *to_rgb.offset(4 as libc::c_int as isize) * i_0
                    + *to_rgb.offset(5 as libc::c_int as isize) * q_0) as libc::c_int;
                let fresh5 = out;
                out = out.offset(1);
                *fresh5 = ((r << 21 as libc::c_int | g << 11 as libc::c_int
                    | b << 1 as libc::c_int) - rgb_bias as libc::c_int)
                    as nes_ntsc_rgb_t;
                n -= 1;
                n;
            }
            if !(3 as libc::c_int > 1 as libc::c_int
                && {
                    alignment_remain -= 1;
                    alignment_remain != 0
                })
            {
                break;
            }
        }
        if 3 as libc::c_int <= 1 as libc::c_int {
            break;
        }
        to_rgb = to_rgb.offset(6 as libc::c_int as isize);
        let mut t: libc::c_float = 0.;
        t = i * -0.5f32 - q * -0.866025f32;
        q = i * -0.866025f32 + q * -0.5f32;
        i = t;
        burst_remain -= 1;
        if !(burst_remain != 0) {
            break;
        }
    };
}
#[no_mangle]
pub static mut nes_ntsc_pixels: [pixel_info_t; 3] = [
    {
        let mut init = pixel_info_t {
            offset: kernel_size as libc::c_int / 2 as libc::c_int
                + (-(4 as libc::c_int)
                    - -(9 as libc::c_int) / 7 as libc::c_int * 8 as libc::c_int)
                + ((-(9 as libc::c_int) + 7 as libc::c_int * 10 as libc::c_int)
                    % 7 as libc::c_int != 0 as libc::c_int) as libc::c_int
                + (7 as libc::c_int
                    - (-(9 as libc::c_int) + 7 as libc::c_int * 10 as libc::c_int)
                        % 7 as libc::c_int) % 7 as libc::c_int
                + kernel_size as libc::c_int * 2 as libc::c_int
                    * ((-(9 as libc::c_int) + 7 as libc::c_int * 10 as libc::c_int)
                        % 7 as libc::c_int),
            negate: 1.0f32
                - (-(4 as libc::c_int) + 100 as libc::c_int & 2 as libc::c_int)
                    as libc::c_float,
            kernel: [
                1 as libc::c_int as libc::c_float,
                1 as libc::c_int as libc::c_float,
                0.6667f32,
                0 as libc::c_int as libc::c_float,
            ],
        };
        init
    },
    {
        let mut init = pixel_info_t {
            offset: kernel_size as libc::c_int / 2 as libc::c_int
                + (-(2 as libc::c_int)
                    - -(7 as libc::c_int) / 7 as libc::c_int * 8 as libc::c_int)
                + ((-(7 as libc::c_int) + 7 as libc::c_int * 10 as libc::c_int)
                    % 7 as libc::c_int != 0 as libc::c_int) as libc::c_int
                + (7 as libc::c_int
                    - (-(7 as libc::c_int) + 7 as libc::c_int * 10 as libc::c_int)
                        % 7 as libc::c_int) % 7 as libc::c_int
                + kernel_size as libc::c_int * 2 as libc::c_int
                    * ((-(7 as libc::c_int) + 7 as libc::c_int * 10 as libc::c_int)
                        % 7 as libc::c_int),
            negate: 1.0f32
                - (-(2 as libc::c_int) + 100 as libc::c_int & 2 as libc::c_int)
                    as libc::c_float,
            kernel: [
                0.3333f32,
                1 as libc::c_int as libc::c_float,
                1 as libc::c_int as libc::c_float,
                0.3333f32,
            ],
        };
        init
    },
    {
        let mut init = pixel_info_t {
            offset: kernel_size as libc::c_int / 2 as libc::c_int
                + (0 as libc::c_int
                    - -(5 as libc::c_int) / 7 as libc::c_int * 8 as libc::c_int)
                + ((-(5 as libc::c_int) + 7 as libc::c_int * 10 as libc::c_int)
                    % 7 as libc::c_int != 0 as libc::c_int) as libc::c_int
                + (7 as libc::c_int
                    - (-(5 as libc::c_int) + 7 as libc::c_int * 10 as libc::c_int)
                        % 7 as libc::c_int) % 7 as libc::c_int
                + kernel_size as libc::c_int * 2 as libc::c_int
                    * ((-(5 as libc::c_int) + 7 as libc::c_int * 10 as libc::c_int)
                        % 7 as libc::c_int),
            negate: 1.0f32
                - (0 as libc::c_int + 100 as libc::c_int & 2 as libc::c_int)
                    as libc::c_float,
            kernel: [
                0 as libc::c_int as libc::c_float,
                0.6667f32,
                1 as libc::c_int as libc::c_float,
                1 as libc::c_int as libc::c_float,
            ],
        };
        init
    },
];
unsafe extern "C" fn merge_kernel_fields(mut io: *mut nes_ntsc_rgb_t) {
    let mut n: libc::c_int = 0;
    n = burst_size as libc::c_int;
    while n != 0 {
        let mut p0: nes_ntsc_rgb_t = (*io
            .offset((burst_size as libc::c_int * 0 as libc::c_int) as isize))
            .wrapping_add(rgb_bias as libc::c_int as nes_ntsc_rgb_t);
        let mut p1: nes_ntsc_rgb_t = (*io
            .offset((burst_size as libc::c_int * 1 as libc::c_int) as isize))
            .wrapping_add(rgb_bias as libc::c_int as nes_ntsc_rgb_t);
        let mut p2: nes_ntsc_rgb_t = (*io
            .offset((burst_size as libc::c_int * 2 as libc::c_int) as isize))
            .wrapping_add(rgb_bias as libc::c_int as nes_ntsc_rgb_t);
        *io
            .offset(
                (burst_size as libc::c_int * 0 as libc::c_int) as isize,
            ) = (p0
            .wrapping_add(p1)
            .wrapping_sub(
                (p0 ^ p1)
                    & ((1 as libc::c_long) << 21 as libc::c_int
                        | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                        | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                        as nes_ntsc_rgb_t,
            ) >> 1 as libc::c_int)
            .wrapping_sub(rgb_bias as libc::c_int as nes_ntsc_rgb_t);
        *io
            .offset(
                (burst_size as libc::c_int * 1 as libc::c_int) as isize,
            ) = (p1
            .wrapping_add(p2)
            .wrapping_sub(
                (p1 ^ p2)
                    & ((1 as libc::c_long) << 21 as libc::c_int
                        | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                        | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                        as nes_ntsc_rgb_t,
            ) >> 1 as libc::c_int)
            .wrapping_sub(rgb_bias as libc::c_int as nes_ntsc_rgb_t);
        *io
            .offset(
                (burst_size as libc::c_int * 2 as libc::c_int) as isize,
            ) = (p2
            .wrapping_add(p0)
            .wrapping_sub(
                (p2 ^ p0)
                    & ((1 as libc::c_long) << 21 as libc::c_int
                        | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                        | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                        as nes_ntsc_rgb_t,
            ) >> 1 as libc::c_int)
            .wrapping_sub(rgb_bias as libc::c_int as nes_ntsc_rgb_t);
        io = io.offset(1);
        io;
        n -= 1;
        n;
    }
}
unsafe extern "C" fn correct_errors(
    mut color: nes_ntsc_rgb_t,
    mut out: *mut nes_ntsc_rgb_t,
) {
    let mut n: libc::c_int = 0;
    n = 3 as libc::c_int;
    while n != 0 {
        let mut i: libc::c_uint = 0;
        i = 0 as libc::c_int as libc::c_uint;
        while i < (rgb_kernel_size as libc::c_int / 2 as libc::c_int) as libc::c_uint {
            let mut error: nes_ntsc_rgb_t = color
                .wrapping_sub(*out.offset(i as isize))
                .wrapping_sub(
                    *out
                        .offset(
                            i
                                .wrapping_add(12 as libc::c_int as libc::c_uint)
                                .wrapping_rem(14 as libc::c_int as libc::c_uint)
                                .wrapping_add(14 as libc::c_int as libc::c_uint) as isize,
                        ),
                )
                .wrapping_sub(
                    *out
                        .offset(
                            i
                                .wrapping_add(10 as libc::c_int as libc::c_uint)
                                .wrapping_rem(14 as libc::c_int as libc::c_uint)
                                .wrapping_add(28 as libc::c_int as libc::c_uint) as isize,
                        ),
                )
                .wrapping_sub(
                    *out
                        .offset(
                            i.wrapping_add(7 as libc::c_int as libc::c_uint) as isize,
                        ),
                )
                .wrapping_sub(
                    *out
                        .offset(
                            i
                                .wrapping_add(5 as libc::c_int as libc::c_uint)
                                .wrapping_add(14 as libc::c_int as libc::c_uint) as isize,
                        ),
                )
                .wrapping_sub(
                    *out
                        .offset(
                            i
                                .wrapping_add(3 as libc::c_int as libc::c_uint)
                                .wrapping_add(28 as libc::c_int as libc::c_uint) as isize,
                        ),
                );
            let mut fourth: nes_ntsc_rgb_t = error
                .wrapping_add(
                    (2 as libc::c_int as libc::c_long
                        * ((1 as libc::c_long) << 21 as libc::c_int
                            | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                            | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long))
                        as nes_ntsc_rgb_t,
                ) >> 2 as libc::c_int;
            fourth
                &= ((rgb_bias as libc::c_int >> 1 as libc::c_int) as libc::c_long
                    - ((1 as libc::c_long) << 21 as libc::c_int
                        | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                        | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long))
                    as nes_ntsc_rgb_t;
            fourth = fourth
                .wrapping_sub(
                    (rgb_bias as libc::c_int >> 2 as libc::c_int) as nes_ntsc_rgb_t,
                );
            let ref mut fresh6 = *out
                .offset(
                    i
                        .wrapping_add(3 as libc::c_int as libc::c_uint)
                        .wrapping_add(28 as libc::c_int as libc::c_uint) as isize,
                );
            *fresh6 = (*fresh6).wrapping_add(fourth);
            let ref mut fresh7 = *out
                .offset(
                    i
                        .wrapping_add(5 as libc::c_int as libc::c_uint)
                        .wrapping_add(14 as libc::c_int as libc::c_uint) as isize,
                );
            *fresh7 = (*fresh7).wrapping_add(fourth);
            let ref mut fresh8 = *out
                .offset(i.wrapping_add(7 as libc::c_int as libc::c_uint) as isize);
            *fresh8 = (*fresh8).wrapping_add(fourth);
            let ref mut fresh9 = *out.offset(i as isize);
            *fresh9 = (*fresh9)
                .wrapping_add(
                    error.wrapping_sub(fourth * 3 as libc::c_int as nes_ntsc_rgb_t),
                );
            i = i.wrapping_add(1);
            i;
        }
        out = out.offset((3 as libc::c_int * rgb_kernel_size as libc::c_int) as isize);
        n -= 1;
        n;
    }
}
#[no_mangle]
pub unsafe extern "C" fn nes_ntsc_init(
    mut ntsc: *mut nes_ntsc_t,
    mut setup: *const nes_ntsc_setup_t,
) {
    let mut merge_fields: libc::c_int = 0;
    let mut entry: libc::c_int = 0;
    let mut impl_0: init_t = init_t {
        to_rgb: [0.; 18],
        to_float: [0.; 1],
        contrast: 0.,
        brightness: 0.,
        artifacts: 0.,
        fringing: 0.,
        kernel: [0.; 462],
    };
    let mut gamma_factor: libc::c_float = 0.;
    if setup.is_null() {
        setup = &nes_ntsc_composite;
    }
    init(&mut impl_0, setup);
    let mut gamma: libc::c_float = (*setup).gamma as libc::c_float * -0.5f32;
    if !(!((*setup).base_palette).is_null() || !((*setup).palette).is_null()) {
        gamma += 0.1333f32;
    }
    gamma_factor = pow(
        fabs(gamma as libc::c_double) as libc::c_float as libc::c_double,
        0.73f32 as libc::c_double,
    ) as libc::c_float;
    if gamma < 0 as libc::c_int as libc::c_float {
        gamma_factor = -gamma_factor;
    }
    merge_fields = (*setup).merge_fields;
    if (*setup).artifacts <= -(1 as libc::c_int) as libc::c_double
        && (*setup).fringing <= -(1 as libc::c_int) as libc::c_double
    {
        merge_fields = 1 as libc::c_int;
    }
    entry = 0 as libc::c_int;
    while entry < nes_ntsc_palette_size as libc::c_int {
        static mut lo_levels: [libc::c_float; 4] = [-0.12f32, 0.00f32, 0.31f32, 0.72f32];
        static mut hi_levels: [libc::c_float; 4] = [0.40f32, 0.68f32, 1.00f32, 1.00f32];
        let mut level: libc::c_int = entry >> 4 as libc::c_int & 0x3 as libc::c_int;
        let mut lo: libc::c_float = lo_levels[level as usize];
        let mut hi: libc::c_float = hi_levels[level as usize];
        let mut color: libc::c_int = entry & 0xf as libc::c_int;
        if color == 0 as libc::c_int {
            lo = hi;
        }
        if color == 0xd as libc::c_int {
            hi = lo;
        }
        if color > 0xd as libc::c_int {
            lo = 0.0f32;
            hi = lo;
        }
        static mut phases: [libc::c_float; 19] = [
            -1.0f32,
            -0.866025f32,
            -0.5f32,
            0.0f32,
            0.5f32,
            0.866025f32,
            1.0f32,
            0.866025f32,
            0.5f32,
            0.0f32,
            -0.5f32,
            -0.866025f32,
            -1.0f32,
            -0.866025f32,
            -0.5f32,
            0.0f32,
            0.5f32,
            0.866025f32,
            1.0f32,
        ];
        let mut sat: libc::c_float = (hi - lo) * 0.5f32;
        let mut i: libc::c_float = phases[color as usize] * sat;
        let mut q: libc::c_float = phases[(color + 3 as libc::c_int) as usize] * sat;
        let mut y: libc::c_float = (hi + lo) * 0.5f32;
        if !((*setup).base_palette).is_null() {
            let mut in_0: *const libc::c_uchar = &*((*setup).base_palette)
                .offset(((entry & 0x3f as libc::c_int) * 3 as libc::c_int) as isize)
                as *const libc::c_uchar;
            static mut to_float: libc::c_float = 1.0f32
                / 0xff as libc::c_int as libc::c_float;
            let mut r: libc::c_float = to_float
                * *in_0.offset(0 as libc::c_int as isize) as libc::c_int
                    as libc::c_float;
            let mut g: libc::c_float = to_float
                * *in_0.offset(1 as libc::c_int as isize) as libc::c_int
                    as libc::c_float;
            let mut b: libc::c_float = to_float
                * *in_0.offset(2 as libc::c_int as isize) as libc::c_int
                    as libc::c_float;
            y = r * 0.299f32 + g * 0.587f32 + b * 0.114f32;
            i = r * 0.596f32 - g * 0.275f32 - b * 0.321f32;
            q = r * 0.212f32 - g * 0.523f32 + b * 0.311f32;
        }
        let mut tint: libc::c_int = entry >> 6 as libc::c_int & 7 as libc::c_int;
        if tint != 0 && color <= 0xd as libc::c_int {
            static mut atten_mul: libc::c_float = 0.79399f32;
            static mut atten_sub: libc::c_float = 0.0782838f32;
            if tint == 7 as libc::c_int {
                y = y * (atten_mul * 1.13f32) - atten_sub * 1.13f32;
            } else {
                static mut tints: [libc::c_uchar; 8] = [
                    0 as libc::c_int as libc::c_uchar,
                    6 as libc::c_int as libc::c_uchar,
                    10 as libc::c_int as libc::c_uchar,
                    8 as libc::c_int as libc::c_uchar,
                    2 as libc::c_int as libc::c_uchar,
                    4 as libc::c_int as libc::c_uchar,
                    0 as libc::c_int as libc::c_uchar,
                    0 as libc::c_int as libc::c_uchar,
                ];
                let tint_color: libc::c_int = tints[tint as usize] as libc::c_int;
                let mut sat_0: libc::c_float = hi * (0.5f32 - atten_mul * 0.5f32)
                    + atten_sub * 0.5f32;
                y -= sat_0 * 0.5f32;
                if tint >= 3 as libc::c_int && tint != 4 as libc::c_int {
                    sat_0 *= 0.6f32;
                    y -= sat_0;
                }
                i += phases[tint_color as usize] * sat_0;
                q += phases[(tint_color + 3 as libc::c_int) as usize] * sat_0;
            }
        }
        if !((*setup).palette).is_null() {
            let mut in_1: *const libc::c_uchar = &*((*setup).palette)
                .offset((entry * 3 as libc::c_int) as isize) as *const libc::c_uchar;
            static mut to_float_0: libc::c_float = 1.0f32
                / 0xff as libc::c_int as libc::c_float;
            let mut r_0: libc::c_float = to_float_0
                * *in_1.offset(0 as libc::c_int as isize) as libc::c_int
                    as libc::c_float;
            let mut g_0: libc::c_float = to_float_0
                * *in_1.offset(1 as libc::c_int as isize) as libc::c_int
                    as libc::c_float;
            let mut b_0: libc::c_float = to_float_0
                * *in_1.offset(2 as libc::c_int as isize) as libc::c_int
                    as libc::c_float;
            y = r_0 * 0.299f32 + g_0 * 0.587f32 + b_0 * 0.114f32;
            i = r_0 * 0.596f32 - g_0 * 0.275f32 - b_0 * 0.321f32;
            q = r_0 * 0.212f32 - g_0 * 0.523f32 + b_0 * 0.311f32;
        }
        y
            *= (*setup).contrast as libc::c_float * 0.5f32
                + 1 as libc::c_int as libc::c_float;
        y
            += (*setup).brightness as libc::c_float * 0.5f32
                - 0.5f32 / 256 as libc::c_int as libc::c_float;
        let mut r_1: libc::c_float = 0.;
        let mut g_1: libc::c_float = 0.;
        r_1 = y + default_decoder[0 as libc::c_int as usize] * i
            + default_decoder[1 as libc::c_int as usize] * q;
        g_1 = y + default_decoder[2 as libc::c_int as usize] * i
            + default_decoder[3 as libc::c_int as usize] * q;
        let mut b_1: libc::c_float = y + default_decoder[4 as libc::c_int as usize] * i
            + default_decoder[5 as libc::c_int as usize] * q;
        r_1 = (r_1 * gamma_factor - gamma_factor) * r_1 + r_1;
        g_1 = (g_1 * gamma_factor - gamma_factor) * g_1 + g_1;
        b_1 = (b_1 * gamma_factor - gamma_factor) * b_1 + b_1;
        y = r_1 * 0.299f32 + g_1 * 0.587f32 + b_1 * 0.114f32;
        i = r_1 * 0.596f32 - g_1 * 0.275f32 - b_1 * 0.321f32;
        q = r_1 * 0.212f32 - g_1 * 0.523f32 + b_1 * 0.311f32;
        i *= ((1 as libc::c_int) << 8 as libc::c_int) as libc::c_float;
        q *= ((1 as libc::c_int) << 8 as libc::c_int) as libc::c_float;
        y *= ((1 as libc::c_int) << 8 as libc::c_int) as libc::c_float;
        y
            += (((1 as libc::c_int) << 8 as libc::c_int) * 2 as libc::c_int)
                as libc::c_float + 0.5f32;
        let mut r_2: libc::c_int = 0;
        let mut g_2: libc::c_int = 0;
        r_2 = (y + impl_0.to_rgb[0 as libc::c_int as usize] * i
            + impl_0.to_rgb[1 as libc::c_int as usize] * q) as libc::c_int;
        g_2 = (y + impl_0.to_rgb[2 as libc::c_int as usize] * i
            + impl_0.to_rgb[3 as libc::c_int as usize] * q) as libc::c_int;
        let mut b_2: libc::c_int = (y + impl_0.to_rgb[4 as libc::c_int as usize] * i
            + impl_0.to_rgb[5 as libc::c_int as usize] * q) as libc::c_int;
        let mut rgb: nes_ntsc_rgb_t = (r_2 << 21 as libc::c_int
            | g_2 << 11 as libc::c_int
            | (if b_2 < 0x3e0 as libc::c_int { b_2 } else { 0x3e0 as libc::c_int })
                << 1 as libc::c_int) as nes_ntsc_rgb_t;
        if !((*setup).palette_out).is_null() {
            let mut out: *mut libc::c_uchar = &mut *((*setup).palette_out)
                .offset((entry * 3 as libc::c_int) as isize) as *mut libc::c_uchar;
            let mut clamped: nes_ntsc_rgb_t = rgb;
            let mut sub: nes_ntsc_rgb_t = clamped
                >> 9 as libc::c_int - (8 as libc::c_int - 8 as libc::c_int)
                & (((1 as libc::c_long) << 21 as libc::c_int
                    | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                    | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                    * 3 as libc::c_int as libc::c_long
                    / 2 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t;
            let mut clamp: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
                .wrapping_sub(sub);
            clamped |= clamp;
            clamp = clamp.wrapping_sub(sub);
            clamped &= clamp;
            *out
                .offset(
                    0 as libc::c_int as isize,
                ) = (clamped >> 21 as libc::c_int) as libc::c_uchar;
            *out
                .offset(
                    1 as libc::c_int as isize,
                ) = (clamped >> 11 as libc::c_int) as libc::c_uchar;
            *out
                .offset(
                    2 as libc::c_int as isize,
                ) = (clamped >> 1 as libc::c_int) as libc::c_uchar;
        }
        if !ntsc.is_null() {
            let mut kernel: *mut nes_ntsc_rgb_t = ((*ntsc).table[entry as usize])
                .as_mut_ptr();
            gen_kernel(&mut impl_0, y, i, q, kernel);
            if merge_fields != 0 {
                merge_kernel_fields(kernel);
            }
            correct_errors(rgb, kernel);
        }
        entry += 1;
        entry;
    }
}
#[no_mangle]
pub unsafe extern "C" fn nes_ntsc_blit(
    mut ntsc: *const nes_ntsc_t,
    mut input: *const libc::c_ushort,
    mut in_row_width: libc::c_long,
    mut burst_phase: libc::c_int,
    mut in_width: libc::c_int,
    mut in_height: libc::c_int,
    mut rgb_out: *mut libc::c_void,
    mut out_pitch: libc::c_long,
) {
    let mut chunk_count: libc::c_int = (in_width - 1 as libc::c_int)
        / nes_ntsc_in_chunk as libc::c_int;
    while in_height != 0 {
        let mut line_in: *const libc::c_ushort = input;
        let ktable: *const libc::c_char = (((*ntsc).table[0 as libc::c_int as usize])
            .as_ptr() as *const libc::c_char)
            .offset(
                (burst_phase as libc::c_ulong)
                    .wrapping_mul(
                        (nes_ntsc_burst_size as libc::c_int as libc::c_ulong)
                            .wrapping_mul(
                                ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                            ),
                    ) as isize,
            );
        let nes_ntsc_pixel0_: libc::c_uint = nes_ntsc_black as libc::c_int
            as libc::c_uint;
        let mut kernel0: *const nes_ntsc_rgb_t = ktable
            .offset(
                (nes_ntsc_pixel0_ as libc::c_ulong)
                    .wrapping_mul(
                        (nes_ntsc_entry_size as libc::c_int as libc::c_ulong)
                            .wrapping_mul(
                                ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                            ),
                    ) as isize,
            ) as *const nes_ntsc_rgb_t;
        let nes_ntsc_pixel1_: libc::c_uint = nes_ntsc_black as libc::c_int
            as libc::c_uint;
        let mut kernel1: *const nes_ntsc_rgb_t = ktable
            .offset(
                (nes_ntsc_pixel1_ as libc::c_ulong)
                    .wrapping_mul(
                        (nes_ntsc_entry_size as libc::c_int as libc::c_ulong)
                            .wrapping_mul(
                                ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                            ),
                    ) as isize,
            ) as *const nes_ntsc_rgb_t;
        let nes_ntsc_pixel2_: libc::c_uint = *line_in as libc::c_uint;
        let mut kernel2: *const nes_ntsc_rgb_t = ktable
            .offset(
                (nes_ntsc_pixel2_ as libc::c_ulong)
                    .wrapping_mul(
                        (nes_ntsc_entry_size as libc::c_int as libc::c_ulong)
                            .wrapping_mul(
                                ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                            ),
                    ) as isize,
            ) as *const nes_ntsc_rgb_t;
        let mut kernelx0: *const nes_ntsc_rgb_t = 0 as *const nes_ntsc_rgb_t;
        let mut kernelx1: *const nes_ntsc_rgb_t = kernel0;
        let mut kernelx2: *const nes_ntsc_rgb_t = kernel0;
        let mut line_out: *mut nes_ntsc_out_t = rgb_out as *mut nes_ntsc_out_t;
        let mut n: libc::c_int = 0;
        line_in = line_in.offset(1);
        line_in;
        n = chunk_count;
        while n != 0 {
            let mut color_: libc::c_uint = 0;
            kernelx0 = kernel0;
            color_ = *line_in.offset(0 as libc::c_int as isize) as libc::c_uint;
            kernel0 = ktable
                .offset(
                    (color_ as libc::c_ulong)
                        .wrapping_mul(
                            (nes_ntsc_entry_size as libc::c_int as libc::c_ulong)
                                .wrapping_mul(
                                    ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                                ),
                        ) as isize,
                ) as *const nes_ntsc_rgb_t;
            let mut raw_: nes_ntsc_rgb_t = (*kernel0.offset(0 as libc::c_int as isize))
                .wrapping_add(
                    *kernel1
                        .offset(
                            ((0 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                                + 14 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernel2
                        .offset(
                            ((0 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                                + 28 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx0
                        .offset(
                            ((0 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                                as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx1
                        .offset(
                            ((0 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                                + 21 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx2
                        .offset(
                            ((0 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                                + 35 as libc::c_int) as isize,
                        ),
                );
            let mut sub: nes_ntsc_rgb_t = raw_ >> 9 as libc::c_int - 0 as libc::c_int
                & (((1 as libc::c_long) << 21 as libc::c_int
                    | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                    | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                    * 3 as libc::c_int as libc::c_long
                    / 2 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t;
            let mut clamp: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
                .wrapping_sub(sub);
            raw_ |= clamp;
            clamp = clamp.wrapping_sub(sub);
            raw_ &= clamp;
            if 32 as libc::c_int == 16 as libc::c_int {
                *line_out
                    .offset(
                        0 as libc::c_int as isize,
                    ) = (raw_ >> 13 as libc::c_int - 0 as libc::c_int
                    & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                    | raw_ >> 8 as libc::c_int - 0 as libc::c_int
                        & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw_ >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 24 as libc::c_int
                || 32 as libc::c_int == 32 as libc::c_int
            {
                *line_out
                    .offset(
                        0 as libc::c_int as isize,
                    ) = (raw_ >> 5 as libc::c_int - 0 as libc::c_int
                    & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                    | raw_ >> 3 as libc::c_int - 0 as libc::c_int
                        & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                    | raw_ >> 1 as libc::c_int - 0 as libc::c_int
                        & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 15 as libc::c_int {
                *line_out
                    .offset(
                        0 as libc::c_int as isize,
                    ) = (raw_ >> 14 as libc::c_int - 0 as libc::c_int
                    & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                    | raw_ >> 9 as libc::c_int - 0 as libc::c_int
                        & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw_ >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 0 as libc::c_int {
                *line_out
                    .offset(
                        0 as libc::c_int as isize,
                    ) = (raw_ << 0 as libc::c_int) as nes_ntsc_out_t;
            }
            let mut raw__0: nes_ntsc_rgb_t = (*kernel0.offset(1 as libc::c_int as isize))
                .wrapping_add(
                    *kernel1
                        .offset(
                            ((1 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                                + 14 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernel2
                        .offset(
                            ((1 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                                + 28 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx0
                        .offset(
                            ((1 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                                as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx1
                        .offset(
                            ((1 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                                + 21 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx2
                        .offset(
                            ((1 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                                + 35 as libc::c_int) as isize,
                        ),
                );
            let mut sub_0: nes_ntsc_rgb_t = raw__0 >> 9 as libc::c_int - 0 as libc::c_int
                & (((1 as libc::c_long) << 21 as libc::c_int
                    | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                    | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                    * 3 as libc::c_int as libc::c_long
                    / 2 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t;
            let mut clamp_0: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
                .wrapping_sub(sub_0);
            raw__0 |= clamp_0;
            clamp_0 = clamp_0.wrapping_sub(sub_0);
            raw__0 &= clamp_0;
            if 32 as libc::c_int == 16 as libc::c_int {
                *line_out
                    .offset(
                        1 as libc::c_int as isize,
                    ) = (raw__0 >> 13 as libc::c_int - 0 as libc::c_int
                    & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                    | raw__0 >> 8 as libc::c_int - 0 as libc::c_int
                        & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__0 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 24 as libc::c_int
                || 32 as libc::c_int == 32 as libc::c_int
            {
                *line_out
                    .offset(
                        1 as libc::c_int as isize,
                    ) = (raw__0 >> 5 as libc::c_int - 0 as libc::c_int
                    & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                    | raw__0 >> 3 as libc::c_int - 0 as libc::c_int
                        & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__0 >> 1 as libc::c_int - 0 as libc::c_int
                        & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 15 as libc::c_int {
                *line_out
                    .offset(
                        1 as libc::c_int as isize,
                    ) = (raw__0 >> 14 as libc::c_int - 0 as libc::c_int
                    & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__0 >> 9 as libc::c_int - 0 as libc::c_int
                        & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__0 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 0 as libc::c_int {
                *line_out
                    .offset(
                        1 as libc::c_int as isize,
                    ) = (raw__0 << 0 as libc::c_int) as nes_ntsc_out_t;
            }
            let mut color__0: libc::c_uint = 0;
            kernelx1 = kernel1;
            color__0 = *line_in.offset(1 as libc::c_int as isize) as libc::c_uint;
            kernel1 = ktable
                .offset(
                    (color__0 as libc::c_ulong)
                        .wrapping_mul(
                            (nes_ntsc_entry_size as libc::c_int as libc::c_ulong)
                                .wrapping_mul(
                                    ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                                ),
                        ) as isize,
                ) as *const nes_ntsc_rgb_t;
            let mut raw__1: nes_ntsc_rgb_t = (*kernel0.offset(2 as libc::c_int as isize))
                .wrapping_add(
                    *kernel1
                        .offset(
                            ((2 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                                + 14 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernel2
                        .offset(
                            ((2 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                                + 28 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx0
                        .offset(
                            ((2 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                                as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx1
                        .offset(
                            ((2 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                                + 21 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx2
                        .offset(
                            ((2 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                                + 35 as libc::c_int) as isize,
                        ),
                );
            let mut sub_1: nes_ntsc_rgb_t = raw__1 >> 9 as libc::c_int - 0 as libc::c_int
                & (((1 as libc::c_long) << 21 as libc::c_int
                    | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                    | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                    * 3 as libc::c_int as libc::c_long
                    / 2 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t;
            let mut clamp_1: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
                .wrapping_sub(sub_1);
            raw__1 |= clamp_1;
            clamp_1 = clamp_1.wrapping_sub(sub_1);
            raw__1 &= clamp_1;
            if 32 as libc::c_int == 16 as libc::c_int {
                *line_out
                    .offset(
                        2 as libc::c_int as isize,
                    ) = (raw__1 >> 13 as libc::c_int - 0 as libc::c_int
                    & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                    | raw__1 >> 8 as libc::c_int - 0 as libc::c_int
                        & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__1 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 24 as libc::c_int
                || 32 as libc::c_int == 32 as libc::c_int
            {
                *line_out
                    .offset(
                        2 as libc::c_int as isize,
                    ) = (raw__1 >> 5 as libc::c_int - 0 as libc::c_int
                    & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                    | raw__1 >> 3 as libc::c_int - 0 as libc::c_int
                        & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__1 >> 1 as libc::c_int - 0 as libc::c_int
                        & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 15 as libc::c_int {
                *line_out
                    .offset(
                        2 as libc::c_int as isize,
                    ) = (raw__1 >> 14 as libc::c_int - 0 as libc::c_int
                    & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__1 >> 9 as libc::c_int - 0 as libc::c_int
                        & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__1 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 0 as libc::c_int {
                *line_out
                    .offset(
                        2 as libc::c_int as isize,
                    ) = (raw__1 << 0 as libc::c_int) as nes_ntsc_out_t;
            }
            let mut raw__2: nes_ntsc_rgb_t = (*kernel0.offset(3 as libc::c_int as isize))
                .wrapping_add(
                    *kernel1
                        .offset(
                            ((3 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                                + 14 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernel2
                        .offset(
                            ((3 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                                + 28 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx0
                        .offset(
                            ((3 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                                as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx1
                        .offset(
                            ((3 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                                + 21 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx2
                        .offset(
                            ((3 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                                + 35 as libc::c_int) as isize,
                        ),
                );
            let mut sub_2: nes_ntsc_rgb_t = raw__2 >> 9 as libc::c_int - 0 as libc::c_int
                & (((1 as libc::c_long) << 21 as libc::c_int
                    | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                    | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                    * 3 as libc::c_int as libc::c_long
                    / 2 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t;
            let mut clamp_2: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
                .wrapping_sub(sub_2);
            raw__2 |= clamp_2;
            clamp_2 = clamp_2.wrapping_sub(sub_2);
            raw__2 &= clamp_2;
            if 32 as libc::c_int == 16 as libc::c_int {
                *line_out
                    .offset(
                        3 as libc::c_int as isize,
                    ) = (raw__2 >> 13 as libc::c_int - 0 as libc::c_int
                    & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                    | raw__2 >> 8 as libc::c_int - 0 as libc::c_int
                        & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__2 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 24 as libc::c_int
                || 32 as libc::c_int == 32 as libc::c_int
            {
                *line_out
                    .offset(
                        3 as libc::c_int as isize,
                    ) = (raw__2 >> 5 as libc::c_int - 0 as libc::c_int
                    & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                    | raw__2 >> 3 as libc::c_int - 0 as libc::c_int
                        & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__2 >> 1 as libc::c_int - 0 as libc::c_int
                        & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 15 as libc::c_int {
                *line_out
                    .offset(
                        3 as libc::c_int as isize,
                    ) = (raw__2 >> 14 as libc::c_int - 0 as libc::c_int
                    & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__2 >> 9 as libc::c_int - 0 as libc::c_int
                        & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__2 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 0 as libc::c_int {
                *line_out
                    .offset(
                        3 as libc::c_int as isize,
                    ) = (raw__2 << 0 as libc::c_int) as nes_ntsc_out_t;
            }
            let mut color__1: libc::c_uint = 0;
            kernelx2 = kernel2;
            color__1 = *line_in.offset(2 as libc::c_int as isize) as libc::c_uint;
            kernel2 = ktable
                .offset(
                    (color__1 as libc::c_ulong)
                        .wrapping_mul(
                            (nes_ntsc_entry_size as libc::c_int as libc::c_ulong)
                                .wrapping_mul(
                                    ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                                ),
                        ) as isize,
                ) as *const nes_ntsc_rgb_t;
            let mut raw__3: nes_ntsc_rgb_t = (*kernel0.offset(4 as libc::c_int as isize))
                .wrapping_add(
                    *kernel1
                        .offset(
                            ((4 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                                + 14 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernel2
                        .offset(
                            ((4 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                                + 28 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx0
                        .offset(
                            ((4 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                                as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx1
                        .offset(
                            ((4 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                                + 21 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx2
                        .offset(
                            ((4 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                                + 35 as libc::c_int) as isize,
                        ),
                );
            let mut sub_3: nes_ntsc_rgb_t = raw__3 >> 9 as libc::c_int - 0 as libc::c_int
                & (((1 as libc::c_long) << 21 as libc::c_int
                    | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                    | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                    * 3 as libc::c_int as libc::c_long
                    / 2 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t;
            let mut clamp_3: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
                .wrapping_sub(sub_3);
            raw__3 |= clamp_3;
            clamp_3 = clamp_3.wrapping_sub(sub_3);
            raw__3 &= clamp_3;
            if 32 as libc::c_int == 16 as libc::c_int {
                *line_out
                    .offset(
                        4 as libc::c_int as isize,
                    ) = (raw__3 >> 13 as libc::c_int - 0 as libc::c_int
                    & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                    | raw__3 >> 8 as libc::c_int - 0 as libc::c_int
                        & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__3 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 24 as libc::c_int
                || 32 as libc::c_int == 32 as libc::c_int
            {
                *line_out
                    .offset(
                        4 as libc::c_int as isize,
                    ) = (raw__3 >> 5 as libc::c_int - 0 as libc::c_int
                    & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                    | raw__3 >> 3 as libc::c_int - 0 as libc::c_int
                        & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__3 >> 1 as libc::c_int - 0 as libc::c_int
                        & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 15 as libc::c_int {
                *line_out
                    .offset(
                        4 as libc::c_int as isize,
                    ) = (raw__3 >> 14 as libc::c_int - 0 as libc::c_int
                    & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__3 >> 9 as libc::c_int - 0 as libc::c_int
                        & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__3 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 0 as libc::c_int {
                *line_out
                    .offset(
                        4 as libc::c_int as isize,
                    ) = (raw__3 << 0 as libc::c_int) as nes_ntsc_out_t;
            }
            let mut raw__4: nes_ntsc_rgb_t = (*kernel0.offset(5 as libc::c_int as isize))
                .wrapping_add(
                    *kernel1
                        .offset(
                            ((5 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                                + 14 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernel2
                        .offset(
                            ((5 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                                + 28 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx0
                        .offset(
                            ((5 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                                as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx1
                        .offset(
                            ((5 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                                + 21 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx2
                        .offset(
                            ((5 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                                + 35 as libc::c_int) as isize,
                        ),
                );
            let mut sub_4: nes_ntsc_rgb_t = raw__4 >> 9 as libc::c_int - 0 as libc::c_int
                & (((1 as libc::c_long) << 21 as libc::c_int
                    | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                    | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                    * 3 as libc::c_int as libc::c_long
                    / 2 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t;
            let mut clamp_4: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
                .wrapping_sub(sub_4);
            raw__4 |= clamp_4;
            clamp_4 = clamp_4.wrapping_sub(sub_4);
            raw__4 &= clamp_4;
            if 32 as libc::c_int == 16 as libc::c_int {
                *line_out
                    .offset(
                        5 as libc::c_int as isize,
                    ) = (raw__4 >> 13 as libc::c_int - 0 as libc::c_int
                    & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                    | raw__4 >> 8 as libc::c_int - 0 as libc::c_int
                        & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__4 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 24 as libc::c_int
                || 32 as libc::c_int == 32 as libc::c_int
            {
                *line_out
                    .offset(
                        5 as libc::c_int as isize,
                    ) = (raw__4 >> 5 as libc::c_int - 0 as libc::c_int
                    & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                    | raw__4 >> 3 as libc::c_int - 0 as libc::c_int
                        & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__4 >> 1 as libc::c_int - 0 as libc::c_int
                        & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 15 as libc::c_int {
                *line_out
                    .offset(
                        5 as libc::c_int as isize,
                    ) = (raw__4 >> 14 as libc::c_int - 0 as libc::c_int
                    & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__4 >> 9 as libc::c_int - 0 as libc::c_int
                        & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__4 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 0 as libc::c_int {
                *line_out
                    .offset(
                        5 as libc::c_int as isize,
                    ) = (raw__4 << 0 as libc::c_int) as nes_ntsc_out_t;
            }
            let mut raw__5: nes_ntsc_rgb_t = (*kernel0.offset(6 as libc::c_int as isize))
                .wrapping_add(
                    *kernel1
                        .offset(
                            ((6 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                                + 14 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernel2
                        .offset(
                            ((6 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                                + 28 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx0
                        .offset(
                            ((6 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                                as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx1
                        .offset(
                            ((6 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                                + 21 as libc::c_int) as isize,
                        ),
                )
                .wrapping_add(
                    *kernelx2
                        .offset(
                            ((6 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                                + 35 as libc::c_int) as isize,
                        ),
                );
            let mut sub_5: nes_ntsc_rgb_t = raw__5 >> 9 as libc::c_int - 0 as libc::c_int
                & (((1 as libc::c_long) << 21 as libc::c_int
                    | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                    | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                    * 3 as libc::c_int as libc::c_long
                    / 2 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t;
            let mut clamp_5: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
                .wrapping_sub(sub_5);
            raw__5 |= clamp_5;
            clamp_5 = clamp_5.wrapping_sub(sub_5);
            raw__5 &= clamp_5;
            if 32 as libc::c_int == 16 as libc::c_int {
                *line_out
                    .offset(
                        6 as libc::c_int as isize,
                    ) = (raw__5 >> 13 as libc::c_int - 0 as libc::c_int
                    & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                    | raw__5 >> 8 as libc::c_int - 0 as libc::c_int
                        & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__5 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 24 as libc::c_int
                || 32 as libc::c_int == 32 as libc::c_int
            {
                *line_out
                    .offset(
                        6 as libc::c_int as isize,
                    ) = (raw__5 >> 5 as libc::c_int - 0 as libc::c_int
                    & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                    | raw__5 >> 3 as libc::c_int - 0 as libc::c_int
                        & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__5 >> 1 as libc::c_int - 0 as libc::c_int
                        & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 15 as libc::c_int {
                *line_out
                    .offset(
                        6 as libc::c_int as isize,
                    ) = (raw__5 >> 14 as libc::c_int - 0 as libc::c_int
                    & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                    | raw__5 >> 9 as libc::c_int - 0 as libc::c_int
                        & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                    | raw__5 >> 4 as libc::c_int - 0 as libc::c_int
                        & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
            }
            if 32 as libc::c_int == 0 as libc::c_int {
                *line_out
                    .offset(
                        6 as libc::c_int as isize,
                    ) = (raw__5 << 0 as libc::c_int) as nes_ntsc_out_t;
            }
            line_in = line_in.offset(3 as libc::c_int as isize);
            line_out = line_out.offset(7 as libc::c_int as isize);
            n -= 1;
            n;
        }
        let mut color__2: libc::c_uint = 0;
        kernelx0 = kernel0;
        color__2 = nes_ntsc_black as libc::c_int as libc::c_uint;
        kernel0 = ktable
            .offset(
                (color__2 as libc::c_ulong)
                    .wrapping_mul(
                        (nes_ntsc_entry_size as libc::c_int as libc::c_ulong)
                            .wrapping_mul(
                                ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                            ),
                    ) as isize,
            ) as *const nes_ntsc_rgb_t;
        let mut raw__6: nes_ntsc_rgb_t = (*kernel0.offset(0 as libc::c_int as isize))
            .wrapping_add(
                *kernel1
                    .offset(
                        ((0 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                            + 14 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernel2
                    .offset(
                        ((0 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                            + 28 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx0
                    .offset(
                        ((0 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                            as isize,
                    ),
            )
            .wrapping_add(
                *kernelx1
                    .offset(
                        ((0 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                            + 21 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx2
                    .offset(
                        ((0 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                            + 35 as libc::c_int) as isize,
                    ),
            );
        let mut sub_6: nes_ntsc_rgb_t = raw__6 >> 9 as libc::c_int - 0 as libc::c_int
            & (((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 3 as libc::c_int as libc::c_long / 2 as libc::c_int as libc::c_long)
                as nes_ntsc_rgb_t;
        let mut clamp_6: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
            | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
            | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
            * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
            .wrapping_sub(sub_6);
        raw__6 |= clamp_6;
        clamp_6 = clamp_6.wrapping_sub(sub_6);
        raw__6 &= clamp_6;
        if 32 as libc::c_int == 16 as libc::c_int {
            *line_out
                .offset(
                    0 as libc::c_int as isize,
                ) = (raw__6 >> 13 as libc::c_int - 0 as libc::c_int
                & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                | raw__6 >> 8 as libc::c_int - 0 as libc::c_int
                    & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__6 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 24 as libc::c_int
            || 32 as libc::c_int == 32 as libc::c_int
        {
            *line_out
                .offset(
                    0 as libc::c_int as isize,
                ) = (raw__6 >> 5 as libc::c_int - 0 as libc::c_int
                & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                | raw__6 >> 3 as libc::c_int - 0 as libc::c_int
                    & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                | raw__6 >> 1 as libc::c_int - 0 as libc::c_int
                    & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 15 as libc::c_int {
            *line_out
                .offset(
                    0 as libc::c_int as isize,
                ) = (raw__6 >> 14 as libc::c_int - 0 as libc::c_int
                & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                | raw__6 >> 9 as libc::c_int - 0 as libc::c_int
                    & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__6 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 0 as libc::c_int {
            *line_out
                .offset(
                    0 as libc::c_int as isize,
                ) = (raw__6 << 0 as libc::c_int) as nes_ntsc_out_t;
        }
        let mut raw__7: nes_ntsc_rgb_t = (*kernel0.offset(1 as libc::c_int as isize))
            .wrapping_add(
                *kernel1
                    .offset(
                        ((1 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                            + 14 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernel2
                    .offset(
                        ((1 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                            + 28 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx0
                    .offset(
                        ((1 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                            as isize,
                    ),
            )
            .wrapping_add(
                *kernelx1
                    .offset(
                        ((1 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                            + 21 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx2
                    .offset(
                        ((1 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                            + 35 as libc::c_int) as isize,
                    ),
            );
        let mut sub_7: nes_ntsc_rgb_t = raw__7 >> 9 as libc::c_int - 0 as libc::c_int
            & (((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 3 as libc::c_int as libc::c_long / 2 as libc::c_int as libc::c_long)
                as nes_ntsc_rgb_t;
        let mut clamp_7: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
            | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
            | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
            * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
            .wrapping_sub(sub_7);
        raw__7 |= clamp_7;
        clamp_7 = clamp_7.wrapping_sub(sub_7);
        raw__7 &= clamp_7;
        if 32 as libc::c_int == 16 as libc::c_int {
            *line_out
                .offset(
                    1 as libc::c_int as isize,
                ) = (raw__7 >> 13 as libc::c_int - 0 as libc::c_int
                & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                | raw__7 >> 8 as libc::c_int - 0 as libc::c_int
                    & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__7 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 24 as libc::c_int
            || 32 as libc::c_int == 32 as libc::c_int
        {
            *line_out
                .offset(
                    1 as libc::c_int as isize,
                ) = (raw__7 >> 5 as libc::c_int - 0 as libc::c_int
                & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                | raw__7 >> 3 as libc::c_int - 0 as libc::c_int
                    & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                | raw__7 >> 1 as libc::c_int - 0 as libc::c_int
                    & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 15 as libc::c_int {
            *line_out
                .offset(
                    1 as libc::c_int as isize,
                ) = (raw__7 >> 14 as libc::c_int - 0 as libc::c_int
                & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                | raw__7 >> 9 as libc::c_int - 0 as libc::c_int
                    & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__7 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 0 as libc::c_int {
            *line_out
                .offset(
                    1 as libc::c_int as isize,
                ) = (raw__7 << 0 as libc::c_int) as nes_ntsc_out_t;
        }
        let mut color__3: libc::c_uint = 0;
        kernelx1 = kernel1;
        color__3 = nes_ntsc_black as libc::c_int as libc::c_uint;
        kernel1 = ktable
            .offset(
                (color__3 as libc::c_ulong)
                    .wrapping_mul(
                        (nes_ntsc_entry_size as libc::c_int as libc::c_ulong)
                            .wrapping_mul(
                                ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                            ),
                    ) as isize,
            ) as *const nes_ntsc_rgb_t;
        let mut raw__8: nes_ntsc_rgb_t = (*kernel0.offset(2 as libc::c_int as isize))
            .wrapping_add(
                *kernel1
                    .offset(
                        ((2 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                            + 14 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernel2
                    .offset(
                        ((2 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                            + 28 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx0
                    .offset(
                        ((2 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                            as isize,
                    ),
            )
            .wrapping_add(
                *kernelx1
                    .offset(
                        ((2 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                            + 21 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx2
                    .offset(
                        ((2 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                            + 35 as libc::c_int) as isize,
                    ),
            );
        let mut sub_8: nes_ntsc_rgb_t = raw__8 >> 9 as libc::c_int - 0 as libc::c_int
            & (((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 3 as libc::c_int as libc::c_long / 2 as libc::c_int as libc::c_long)
                as nes_ntsc_rgb_t;
        let mut clamp_8: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
            | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
            | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
            * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
            .wrapping_sub(sub_8);
        raw__8 |= clamp_8;
        clamp_8 = clamp_8.wrapping_sub(sub_8);
        raw__8 &= clamp_8;
        if 32 as libc::c_int == 16 as libc::c_int {
            *line_out
                .offset(
                    2 as libc::c_int as isize,
                ) = (raw__8 >> 13 as libc::c_int - 0 as libc::c_int
                & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                | raw__8 >> 8 as libc::c_int - 0 as libc::c_int
                    & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__8 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 24 as libc::c_int
            || 32 as libc::c_int == 32 as libc::c_int
        {
            *line_out
                .offset(
                    2 as libc::c_int as isize,
                ) = (raw__8 >> 5 as libc::c_int - 0 as libc::c_int
                & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                | raw__8 >> 3 as libc::c_int - 0 as libc::c_int
                    & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                | raw__8 >> 1 as libc::c_int - 0 as libc::c_int
                    & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 15 as libc::c_int {
            *line_out
                .offset(
                    2 as libc::c_int as isize,
                ) = (raw__8 >> 14 as libc::c_int - 0 as libc::c_int
                & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                | raw__8 >> 9 as libc::c_int - 0 as libc::c_int
                    & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__8 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 0 as libc::c_int {
            *line_out
                .offset(
                    2 as libc::c_int as isize,
                ) = (raw__8 << 0 as libc::c_int) as nes_ntsc_out_t;
        }
        let mut raw__9: nes_ntsc_rgb_t = (*kernel0.offset(3 as libc::c_int as isize))
            .wrapping_add(
                *kernel1
                    .offset(
                        ((3 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                            + 14 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernel2
                    .offset(
                        ((3 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                            + 28 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx0
                    .offset(
                        ((3 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                            as isize,
                    ),
            )
            .wrapping_add(
                *kernelx1
                    .offset(
                        ((3 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                            + 21 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx2
                    .offset(
                        ((3 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                            + 35 as libc::c_int) as isize,
                    ),
            );
        let mut sub_9: nes_ntsc_rgb_t = raw__9 >> 9 as libc::c_int - 0 as libc::c_int
            & (((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 3 as libc::c_int as libc::c_long / 2 as libc::c_int as libc::c_long)
                as nes_ntsc_rgb_t;
        let mut clamp_9: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
            | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
            | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
            * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
            .wrapping_sub(sub_9);
        raw__9 |= clamp_9;
        clamp_9 = clamp_9.wrapping_sub(sub_9);
        raw__9 &= clamp_9;
        if 32 as libc::c_int == 16 as libc::c_int {
            *line_out
                .offset(
                    3 as libc::c_int as isize,
                ) = (raw__9 >> 13 as libc::c_int - 0 as libc::c_int
                & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                | raw__9 >> 8 as libc::c_int - 0 as libc::c_int
                    & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__9 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 24 as libc::c_int
            || 32 as libc::c_int == 32 as libc::c_int
        {
            *line_out
                .offset(
                    3 as libc::c_int as isize,
                ) = (raw__9 >> 5 as libc::c_int - 0 as libc::c_int
                & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                | raw__9 >> 3 as libc::c_int - 0 as libc::c_int
                    & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                | raw__9 >> 1 as libc::c_int - 0 as libc::c_int
                    & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 15 as libc::c_int {
            *line_out
                .offset(
                    3 as libc::c_int as isize,
                ) = (raw__9 >> 14 as libc::c_int - 0 as libc::c_int
                & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                | raw__9 >> 9 as libc::c_int - 0 as libc::c_int
                    & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__9 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 0 as libc::c_int {
            *line_out
                .offset(
                    3 as libc::c_int as isize,
                ) = (raw__9 << 0 as libc::c_int) as nes_ntsc_out_t;
        }
        let mut color__4: libc::c_uint = 0;
        kernelx2 = kernel2;
        color__4 = nes_ntsc_black as libc::c_int as libc::c_uint;
        kernel2 = ktable
            .offset(
                (color__4 as libc::c_ulong)
                    .wrapping_mul(
                        (nes_ntsc_entry_size as libc::c_int as libc::c_ulong)
                            .wrapping_mul(
                                ::core::mem::size_of::<nes_ntsc_rgb_t>() as libc::c_ulong,
                            ),
                    ) as isize,
            ) as *const nes_ntsc_rgb_t;
        let mut raw__10: nes_ntsc_rgb_t = (*kernel0.offset(4 as libc::c_int as isize))
            .wrapping_add(
                *kernel1
                    .offset(
                        ((4 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                            + 14 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernel2
                    .offset(
                        ((4 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                            + 28 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx0
                    .offset(
                        ((4 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                            as isize,
                    ),
            )
            .wrapping_add(
                *kernelx1
                    .offset(
                        ((4 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                            + 21 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx2
                    .offset(
                        ((4 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                            + 35 as libc::c_int) as isize,
                    ),
            );
        let mut sub_10: nes_ntsc_rgb_t = raw__10 >> 9 as libc::c_int - 0 as libc::c_int
            & (((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 3 as libc::c_int as libc::c_long / 2 as libc::c_int as libc::c_long)
                as nes_ntsc_rgb_t;
        let mut clamp_10: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
            | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
            | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
            * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
            .wrapping_sub(sub_10);
        raw__10 |= clamp_10;
        clamp_10 = clamp_10.wrapping_sub(sub_10);
        raw__10 &= clamp_10;
        if 32 as libc::c_int == 16 as libc::c_int {
            *line_out
                .offset(
                    4 as libc::c_int as isize,
                ) = (raw__10 >> 13 as libc::c_int - 0 as libc::c_int
                & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                | raw__10 >> 8 as libc::c_int - 0 as libc::c_int
                    & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__10 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 24 as libc::c_int
            || 32 as libc::c_int == 32 as libc::c_int
        {
            *line_out
                .offset(
                    4 as libc::c_int as isize,
                ) = (raw__10 >> 5 as libc::c_int - 0 as libc::c_int
                & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                | raw__10 >> 3 as libc::c_int - 0 as libc::c_int
                    & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                | raw__10 >> 1 as libc::c_int - 0 as libc::c_int
                    & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 15 as libc::c_int {
            *line_out
                .offset(
                    4 as libc::c_int as isize,
                ) = (raw__10 >> 14 as libc::c_int - 0 as libc::c_int
                & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                | raw__10 >> 9 as libc::c_int - 0 as libc::c_int
                    & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__10 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 0 as libc::c_int {
            *line_out
                .offset(
                    4 as libc::c_int as isize,
                ) = (raw__10 << 0 as libc::c_int) as nes_ntsc_out_t;
        }
        let mut raw__11: nes_ntsc_rgb_t = (*kernel0.offset(5 as libc::c_int as isize))
            .wrapping_add(
                *kernel1
                    .offset(
                        ((5 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                            + 14 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernel2
                    .offset(
                        ((5 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                            + 28 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx0
                    .offset(
                        ((5 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                            as isize,
                    ),
            )
            .wrapping_add(
                *kernelx1
                    .offset(
                        ((5 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                            + 21 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx2
                    .offset(
                        ((5 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                            + 35 as libc::c_int) as isize,
                    ),
            );
        let mut sub_11: nes_ntsc_rgb_t = raw__11 >> 9 as libc::c_int - 0 as libc::c_int
            & (((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 3 as libc::c_int as libc::c_long / 2 as libc::c_int as libc::c_long)
                as nes_ntsc_rgb_t;
        let mut clamp_11: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
            | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
            | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
            * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
            .wrapping_sub(sub_11);
        raw__11 |= clamp_11;
        clamp_11 = clamp_11.wrapping_sub(sub_11);
        raw__11 &= clamp_11;
        if 32 as libc::c_int == 16 as libc::c_int {
            *line_out
                .offset(
                    5 as libc::c_int as isize,
                ) = (raw__11 >> 13 as libc::c_int - 0 as libc::c_int
                & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                | raw__11 >> 8 as libc::c_int - 0 as libc::c_int
                    & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__11 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 24 as libc::c_int
            || 32 as libc::c_int == 32 as libc::c_int
        {
            *line_out
                .offset(
                    5 as libc::c_int as isize,
                ) = (raw__11 >> 5 as libc::c_int - 0 as libc::c_int
                & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                | raw__11 >> 3 as libc::c_int - 0 as libc::c_int
                    & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                | raw__11 >> 1 as libc::c_int - 0 as libc::c_int
                    & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 15 as libc::c_int {
            *line_out
                .offset(
                    5 as libc::c_int as isize,
                ) = (raw__11 >> 14 as libc::c_int - 0 as libc::c_int
                & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                | raw__11 >> 9 as libc::c_int - 0 as libc::c_int
                    & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__11 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 0 as libc::c_int {
            *line_out
                .offset(
                    5 as libc::c_int as isize,
                ) = (raw__11 << 0 as libc::c_int) as nes_ntsc_out_t;
        }
        let mut raw__12: nes_ntsc_rgb_t = (*kernel0.offset(6 as libc::c_int as isize))
            .wrapping_add(
                *kernel1
                    .offset(
                        ((6 as libc::c_int + 12 as libc::c_int) % 7 as libc::c_int
                            + 14 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernel2
                    .offset(
                        ((6 as libc::c_int + 10 as libc::c_int) % 7 as libc::c_int
                            + 28 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx0
                    .offset(
                        ((6 as libc::c_int + 7 as libc::c_int) % 14 as libc::c_int)
                            as isize,
                    ),
            )
            .wrapping_add(
                *kernelx1
                    .offset(
                        ((6 as libc::c_int + 5 as libc::c_int) % 7 as libc::c_int
                            + 21 as libc::c_int) as isize,
                    ),
            )
            .wrapping_add(
                *kernelx2
                    .offset(
                        ((6 as libc::c_int + 3 as libc::c_int) % 7 as libc::c_int
                            + 35 as libc::c_int) as isize,
                    ),
            );
        let mut sub_12: nes_ntsc_rgb_t = raw__12 >> 9 as libc::c_int - 0 as libc::c_int
            & (((1 as libc::c_long) << 21 as libc::c_int
                | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
                | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
                * 3 as libc::c_int as libc::c_long / 2 as libc::c_int as libc::c_long)
                as nes_ntsc_rgb_t;
        let mut clamp_12: nes_ntsc_rgb_t = ((((1 as libc::c_long) << 21 as libc::c_int
            | ((1 as libc::c_int) << 11 as libc::c_int) as libc::c_long
            | ((1 as libc::c_int) << 1 as libc::c_int) as libc::c_long)
            * 0x101 as libc::c_int as libc::c_long) as nes_ntsc_rgb_t)
            .wrapping_sub(sub_12);
        raw__12 |= clamp_12;
        clamp_12 = clamp_12.wrapping_sub(sub_12);
        raw__12 &= clamp_12;
        if 32 as libc::c_int == 16 as libc::c_int {
            *line_out
                .offset(
                    6 as libc::c_int as isize,
                ) = (raw__12 >> 13 as libc::c_int - 0 as libc::c_int
                & 0xf800 as libc::c_int as nes_ntsc_rgb_t
                | raw__12 >> 8 as libc::c_int - 0 as libc::c_int
                    & 0x7e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__12 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 24 as libc::c_int
            || 32 as libc::c_int == 32 as libc::c_int
        {
            *line_out
                .offset(
                    6 as libc::c_int as isize,
                ) = (raw__12 >> 5 as libc::c_int - 0 as libc::c_int
                & 0xff0000 as libc::c_int as nes_ntsc_rgb_t
                | raw__12 >> 3 as libc::c_int - 0 as libc::c_int
                    & 0xff00 as libc::c_int as nes_ntsc_rgb_t
                | raw__12 >> 1 as libc::c_int - 0 as libc::c_int
                    & 0xff as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 15 as libc::c_int {
            *line_out
                .offset(
                    6 as libc::c_int as isize,
                ) = (raw__12 >> 14 as libc::c_int - 0 as libc::c_int
                & 0x7c00 as libc::c_int as nes_ntsc_rgb_t
                | raw__12 >> 9 as libc::c_int - 0 as libc::c_int
                    & 0x3e0 as libc::c_int as nes_ntsc_rgb_t
                | raw__12 >> 4 as libc::c_int - 0 as libc::c_int
                    & 0x1f as libc::c_int as nes_ntsc_rgb_t) as nes_ntsc_out_t;
        }
        if 32 as libc::c_int == 0 as libc::c_int {
            *line_out
                .offset(
                    6 as libc::c_int as isize,
                ) = (raw__12 << 0 as libc::c_int) as nes_ntsc_out_t;
        }
        burst_phase = (burst_phase + 1 as libc::c_int)
            % nes_ntsc_burst_count as libc::c_int;
        input = input.offset(in_row_width as isize);
        rgb_out = (rgb_out as *mut libc::c_char).offset(out_pitch as isize)
            as *mut libc::c_void;
        in_height -= 1;
        in_height;
    }
}
