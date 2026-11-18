//! Evolutionary Triangle Painter — Rust core and C/WASM FFI.

#![allow(clippy::missing_safety_doc)]

pub mod engine;
pub mod presets;
pub mod prng;
pub mod raster;
pub mod svg;

use std::alloc::{alloc as std_alloc, dealloc as std_dealloc, Layout};
use std::slice;

use engine::Engine;
use raster::Triangle;

#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let Ok(layout) = Layout::from_size_align(size, 4) else {
        return std::ptr::null_mut();
    };
    unsafe { std_alloc(layout) }
}

#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    if let Ok(layout) = Layout::from_size_align(size, 4) {
        unsafe { std_dealloc(ptr, layout) };
    }
}

#[no_mangle]
pub extern "C" fn engine_create(
    width: u32,
    height: u32,
    num_triangles: u32,
    seed: u32,
) -> *mut Engine {
    let engine = Engine::new(width, height, num_triangles, u64::from(seed));
    Box::into_raw(Box::new(engine))
}

#[no_mangle]
pub unsafe extern "C" fn engine_destroy(ptr: *mut Engine) {
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr) });
    }
}

#[no_mangle]
pub unsafe extern "C" fn engine_load_preset(ptr: *mut Engine, preset_id: u32) {
    if let Some(engine) = unsafe { ptr.as_mut() } {
        engine.load_preset(preset_id);
    }
}

#[no_mangle]
pub unsafe extern "C" fn engine_set_target_rgba(
    ptr: *mut Engine,
    rgba_ptr: *const u8,
    byte_len: usize,
) -> u32 {
    let Some(engine) = (unsafe { ptr.as_mut() }) else {
        return 0;
    };
    if rgba_ptr.is_null() || byte_len == 0 {
        return 0;
    }
    let bytes = unsafe { slice::from_raw_parts(rgba_ptr, byte_len) };
    u32::from(engine.set_target_rgba(bytes))
}

#[no_mangle]
pub unsafe extern "C" fn engine_reset(ptr: *mut Engine, seed: u32) {
    if let Some(engine) = unsafe { ptr.as_mut() } {
        engine.reset(u64::from(seed));
    }
}

#[no_mangle]
pub unsafe extern "C" fn engine_set_triangle_count(ptr: *mut Engine, num_triangles: u32) {
    if let Some(engine) = unsafe { ptr.as_mut() } {
        engine.set_triangle_count(num_triangles);
    }
}

#[no_mangle]
pub unsafe extern "C" fn engine_set_sa_strength(ptr: *mut Engine, strength: f32) {
    if let Some(engine) = unsafe { ptr.as_mut() } {
        engine.set_sa_strength(strength);
    }
}

#[no_mangle]
pub unsafe extern "C" fn engine_step(ptr: *mut Engine, steps: u32) -> u32 {
    let Some(engine) = (unsafe { ptr.as_mut() }) else {
        return 0;
    };
    engine.step(steps)
}

#[no_mangle]
pub unsafe extern "C" fn engine_width(ptr: *const Engine) -> u32 {
    unsafe { ptr.as_ref() }.map_or(0, Engine::width)
}

#[no_mangle]
pub unsafe extern "C" fn engine_height(ptr: *const Engine) -> u32 {
    unsafe { ptr.as_ref() }.map_or(0, Engine::height)
}

#[no_mangle]
pub unsafe extern "C" fn engine_canvas_ptr(ptr: *const Engine) -> *const u8 {
    unsafe { ptr.as_ref() }.map_or(std::ptr::null(), |e| e.canvas().as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn engine_target_ptr(ptr: *const Engine) -> *const u8 {
    unsafe { ptr.as_ref() }.map_or(std::ptr::null(), |e| e.target().as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn engine_pixel_bytes(ptr: *const Engine) -> usize {
    unsafe { ptr.as_ref() }.map_or(0, |e| e.canvas().len())
}

#[no_mangle]
pub unsafe extern "C" fn engine_triangles_ptr(ptr: *const Engine) -> *const Triangle {
    unsafe { ptr.as_ref() }.map_or(std::ptr::null(), |e| e.triangles().as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn engine_triangle_count(ptr: *const Engine) -> u32 {
    unsafe { ptr.as_ref() }.map_or(0, |e| e.triangles().len() as u32)
}

#[no_mangle]
pub extern "C" fn engine_triangle_stride() -> usize {
    std::mem::size_of::<Triangle>()
}

#[no_mangle]
pub unsafe extern "C" fn engine_generation(ptr: *const Engine) -> f64 {
    unsafe { ptr.as_ref() }.map_or(0.0, |e| e.generation() as f64)
}

#[no_mangle]
pub unsafe extern "C" fn engine_accepted(ptr: *const Engine) -> f64 {
    unsafe { ptr.as_ref() }.map_or(0.0, |e| e.accepted() as f64)
}

#[no_mangle]
pub unsafe extern "C" fn engine_rmse(ptr: *const Engine) -> f64 {
    unsafe { ptr.as_ref() }.map_or(0.0, Engine::rmse)
}

#[no_mangle]
pub unsafe extern "C" fn engine_psnr(ptr: *const Engine) -> f64 {
    unsafe { ptr.as_ref() }.map_or(0.0, Engine::psnr)
}

#[no_mangle]
pub unsafe extern "C" fn engine_fitness_pct(ptr: *const Engine) -> f64 {
    unsafe { ptr.as_ref() }.map_or(0.0, Engine::fitness_pct)
}

#[no_mangle]
pub unsafe extern "C" fn engine_acceptance_rate(ptr: *const Engine) -> f64 {
    unsafe { ptr.as_ref() }.map_or(0.0, Engine::acceptance_rate)
}

#[no_mangle]
pub unsafe extern "C" fn engine_temperature(ptr: *const Engine) -> f64 {
    unsafe { ptr.as_ref() }.map_or(0.0, Engine::temperature)
}

#[no_mangle]
pub unsafe extern "C" fn engine_export_svg(ptr: *mut Engine) -> usize {
    let Some(engine) = (unsafe { ptr.as_mut() }) else {
        return 0;
    };
    engine.export_svg().len()
}

#[no_mangle]
pub unsafe extern "C" fn engine_svg_ptr(ptr: *const Engine) -> *const u8 {
    unsafe { ptr.as_ref() }.map_or(std::ptr::null(), |e| e.svg_bytes().as_ptr())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::{generate_preset, PRESET_COUNT};
    use crate::prng::Rng;
    use crate::raster::{render_full, Triangle};
    use crate::svg::export_svg;

    #[test]
    fn deterministic_seeded_prng() {
        let mut a = Rng::new(1337);
        let mut b = Rng::new(1337);
        let mut c = Rng::new(7331);
        for _ in 0..128 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(Rng::new(1337).next_u64(), c.next_u64());
        for _ in 0..200 {
            let v = a.next_f32();
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn scanline_rasterizer_covers_interior_and_blends_alpha() {
        let tri = Triangle {
            x0: 10.0,
            y0: 10.0,
            x1: 30.0,
            y1: 10.0,
            x2: 20.0,
            y2: 30.0,
            r: 200,
            g: 100,
            b: 50,
            a: 255,
        };
        let buf = Engine::debug_rasterize_single(40, 40, [0, 0, 0], &tri);
        // Outside vertex
        let out_idx = 0;
        assert_eq!(&buf[out_idx..out_idx + 4], &[0, 0, 0, 255]);
        // Well inside centroid (20, 16)
        let in_idx = (16 * 40 + 20) * 4;
        assert_eq!(&buf[in_idx..in_idx + 4], &[200, 100, 50, 255]);

        // Translucent 50% alpha over black
        let half = Triangle { a: 128, ..tri };
        let half_buf = Engine::debug_rasterize_single(40, 40, [0, 0, 0], &half);
        assert!(half_buf[in_idx] >= 99 && half_buf[in_idx] <= 102);
        assert!(half_buf[in_idx + 1] >= 49 && half_buf[in_idx + 1] <= 51);
    }

    #[test]
    fn incremental_dirty_rect_matches_full_render() {
        let mut engine = Engine::new(64, 64, 40, 99);
        engine.set_sa_strength(0.35);
        engine.step(300);
        let fresh = engine.debug_render_fresh();
        assert_eq!(
            engine.canvas(),
            fresh.as_slice(),
            "Incremental canvas diverged from full render"
        );
    }

    #[test]
    fn rmse_drops_over_evolution_on_synthetic_target() {
        let w = 64;
        let h = 64;
        let mut target = vec![0_u8; w * h * 4];
        // Synthetic bright crimson diagonal band on deep navy background
        let tri_gt = Triangle {
            x0: 8.0,
            y0: 8.0,
            x1: 58.0,
            y1: 16.0,
            x2: 22.0,
            y2: 58.0,
            r: 240,
            g: 48,
            b: 64,
            a: 255,
        };
        render_full(&mut target, w as i32, h as i32, [14, 24, 52], &[tri_gt]);

        let mut engine = Engine::new(w as u32, h as u32, 30, 42);
        engine.set_sa_strength(0.0); // Pure hill climbing guarantees monotonic RMSE drop
        assert!(engine.set_target_rgba(&target));
        let initial_rmse = engine.rmse();
        let accepted = engine.step(1200);
        let final_rmse = engine.rmse();

        assert!(accepted > 0, "Expected accepted mutations");
        assert!(
            final_rmse < initial_rmse * 0.7,
            "RMSE should drop substantially: initial={initial_rmse}, final={final_rmse}"
        );
        assert!(engine.psnr() > 15.0);
    }

    #[test]
    fn presets_generate_valid_opaque_buffers() {
        for id in 0..PRESET_COUNT {
            let rgba = generate_preset(id, 48, 36);
            assert_eq!(rgba.len(), 48 * 36 * 4);
            assert!(rgba.as_chunks::<4>().0.iter().all(|px| px[3] == 255));
        }
    }

    #[test]
    fn triangle_struct_stride_and_svg_export_valid() {
        assert_eq!(std::mem::size_of::<Triangle>(), 28);
        let mut engine = Engine::new(80, 60, 25, 7);
        engine.step(50);
        let svg = engine.export_svg().to_owned();
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 80 60\""));
        assert!(svg.contains("<rect width=\"100%\" height=\"100%\" fill=\"#"));
        assert!(svg.contains("<polygon points=\""));
        assert!(svg.trim_end().ends_with("</svg>"));

        // Zero-alpha triangle skipped in export
        let empty = export_svg(
            10,
            10,
            [0, 0, 0],
            &[Triangle {
                x0: 0.0,
                y0: 0.0,
                x1: 5.0,
                y1: 0.0,
                x2: 0.0,
                y2: 5.0,
                r: 255,
                g: 0,
                b: 0,
                a: 0,
            }],
        );
        assert!(!empty.contains("<polygon"));
    }

    #[test]
    fn ffi_roundtrip_and_resize_triangle_count() {
        let ptr = engine_create(48, 48, 20, 123);
        assert!(!ptr.is_null());
        unsafe {
            assert_eq!(engine_width(ptr), 48);
            assert_eq!(engine_height(ptr), 48);
            assert_eq!(engine_triangle_count(ptr), 20);
            assert_eq!(engine_pixel_bytes(ptr), 48 * 48 * 4);
            engine_load_preset(ptr, 4);
            let accepted = engine_step(ptr, 100);
            assert!(accepted > 0);
            engine_set_triangle_count(ptr, 45);
            assert_eq!(engine_triangle_count(ptr), 45);
            let svg_len = engine_export_svg(ptr);
            assert!(svg_len > 80);
            assert!(!engine_svg_ptr(ptr).is_null());
            engine_destroy(ptr);
        }
    }
}
