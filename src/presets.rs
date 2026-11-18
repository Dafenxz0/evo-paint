//! Procedural RGBA target presets for evolutionary triangle approximation.

use std::f32::consts::{PI, TAU};

pub const PRESET_COUNT: u32 = 5;

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[inline]
fn lerp_rgb(c0: [f32; 3], c1: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp(c0[0], c1[0], t),
        lerp(c0[1], c1[1], t),
        lerp(c0[2], c1[2], t),
    ]
}

#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn to_u8(v: f32) -> u8 {
    v.clamp(0.0, 255.0).round() as u8
}

/// Generates a crisp RGBA image of size `width x height` for `preset_id % PRESET_COUNT`.
pub fn generate_preset(preset_id: u32, width: u32, height: u32) -> Vec<u8> {
    let w = width.max(1);
    let h = height.max(1);
    let mut out = vec![0_u8; (w * h * 4) as usize];
    let id = preset_id % PRESET_COUNT;

    for py in 0..h {
        let v = (py as f32 + 0.5) / h as f32;
        for px in 0..w {
            let u = (px as f32 + 0.5) / w as f32;
            let rgb = match id {
                0 => geometric_sunset(u, v),
                1 => cyber_eye(u, v),
                2 => spiral_nebula(u, v),
                3 => retro_mountain(u, v),
                _ => mondrian(u, v),
            };
            let idx = ((py * w + px) * 4) as usize;
            out[idx] = to_u8(rgb[0]);
            out[idx + 1] = to_u8(rgb[1]);
            out[idx + 2] = to_u8(rgb[2]);
            out[idx + 3] = 255;
        }
    }
    out
}

fn geometric_sunset(u: f32, v: f32) -> [f32; 3] {
    let horizon = 0.62;
    let mut col = if v < horizon {
        let t = v / horizon;
        let top = [20.0, 14.0, 56.0];
        let mid = [148.0, 32.0, 118.0];
        let low = [252.0, 146.0, 62.0];
        if t < 0.55 {
            lerp_rgb(top, mid, t / 0.55)
        } else {
            lerp_rgb(mid, low, (t - 0.55) / 0.45)
        }
    } else {
        let t = (v - horizon) / (1.0 - horizon);
        lerp_rgb([44.0, 16.0, 74.0], [10.0, 8.0, 28.0], t)
    };

    // Banded sun disc above horizon
    let dx = u - 0.5;
    let dy = v - 0.43;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 0.23 && v < horizon {
        let sun_t = ((v - 0.20) / 0.42).clamp(0.0, 1.0);
        let sun_col = lerp_rgb([255.0, 236.0, 110.0], [255.0, 54.0, 110.0], sun_t);
        let stripe = if v > 0.36 {
            ((v - 0.36) * 42.0).fract() > 0.26
        } else {
            true
        };
        if stripe {
            let edge = 1.0 - smoothstep(0.222, 0.23, dist);
            col = lerp_rgb(col, sun_col, edge);
        }
    }

    // Layered mountain silhouettes
    let ridge_far = 0.56 - 0.14 * (1.0 - (u - 0.25).abs() * 3.2).max(0.0)
        - 0.17 * (1.0 - (u - 0.72).abs() * 2.8).max(0.0);
    if v >= ridge_far && v < horizon {
        col = [42.0, 22.0, 68.0];
    }

    let ridge_near = 0.62 - 0.11 * (1.0 - (u - 0.14).abs() * 3.8).max(0.0)
        - 0.08 * (1.0 - (u - 0.52).abs() * 4.5).max(0.0)
        - 0.13 * (1.0 - (u - 0.86).abs() * 3.4).max(0.0);
    if v >= ridge_near && v < horizon {
        col = [18.0, 10.0, 34.0];
    }

    // Water sun reflection + perspective grid
    if v >= horizon {
        let wt = (v - horizon) / (1.0 - horizon);
        let shimmer = (u - 0.5).abs() < (0.18 * (1.0 - wt * 0.45));
        if shimmer && ((v * 58.0).sin() > -0.1) {
            let ref_col = lerp_rgb([255.0, 190.0, 82.0], [215.0, 48.0, 128.0], wt);
            col = lerp_rgb(col, ref_col, 0.72 * (1.0 - wt * 0.5));
        }
        let gx = ((u - 0.5) / (0.18 + wt * 0.9) * 8.0).fract().abs();
        let gy = (wt * 9.0).fract();
        if gx < 0.07 || gy < 0.09 {
            col = lerp_rgb(col, [0.0, 224.0, 255.0], 0.55);
        }
    }

    col
}

fn cyber_eye(u: f32, v: f32) -> [f32; 3] {
    let cx = u - 0.5;
    let cy = v - 0.5;
    let r = (cx * cx + cy * cy).sqrt();
    let angle = cy.atan2(cx);

    let mut col = [10.0, 12.0, 24.0];

    // Outer metallic bezel
    if r < 0.45 && r >= 0.36 {
        let ring = ((r - 0.36) / 0.09 * PI).sin();
        col = lerp_rgb([26.0, 34.0, 58.0], [72.0, 108.0, 160.0], ring);
        if (angle * 12.0).sin() > 0.85 {
            col = [0.0, 232.0, 255.0];
        }
    }

    // Iris radial fibers
    if r < 0.36 && r >= 0.14 {
        let t = (r - 0.14) / 0.22;
        let fiber = 0.5
            + 0.3 * (angle * 18.0).sin()
            + 0.2 * (angle * 31.0 + r * 24.0).cos();
        let inner_c = [255.0, 168.0, 36.0];
        let mid_c = [0.0, 210.0, 235.0];
        let outer_c = [18.0, 52.0, 128.0];
        let base = if t < 0.45 {
            lerp_rgb(inner_c, mid_c, t / 0.45)
        } else {
            lerp_rgb(mid_c, outer_c, (t - 0.45) / 0.55)
        };
        col = lerp_rgb(base, [230.0, 252.0, 255.0], fiber.clamp(0.0, 1.0) * 0.35);
        if (r - 0.25).abs() < 0.012 {
            col = [255.0, 62.0, 152.0];
        }
    }

    // Pupil + specular glints
    if r < 0.14 {
        col = [6.0, 7.0, 14.0];
    }

    let g1 = ((u - 0.43).powi(2) + (v - 0.41).powi(2)).sqrt();
    if g1 < 0.055 {
        col = lerp_rgb(col, [255.0, 255.0, 255.0], 1.0 - smoothstep(0.03, 0.055, g1));
    }
    let g2 = ((u - 0.59).powi(2) + (v - 0.57).powi(2)).sqrt();
    if g2 < 0.025 {
        col = lerp_rgb(col, [180.0, 245.0, 255.0], 0.9);
    }

    col
}

fn spiral_nebula(u: f32, v: f32) -> [f32; 3] {
    let cx = u - 0.5;
    let cy = v - 0.5;
    let r = (cx * cx + cy * cy).sqrt().max(1e-4);
    let theta = cy.atan2(cx);

    let spiral = (3.0 * theta - 13.0 * r.ln()).sin() * 0.5 + 0.5;
    let falloff = (-r * 3.6).exp();
    let arm_glow = spiral.powf(2.2) * (1.0 - smoothstep(0.12, 0.54, r));

    let bg = [8.0, 10.0, 26.0];
    let arm_a = [24.0, 205.0, 232.0];
    let arm_b = [212.0, 52.0, 180.0];
    let core = [255.0, 224.0, 148.0];

    let arm_col = lerp_rgb(arm_a, arm_b, 0.5 + 0.5 * (theta + r * 6.0).cos());
    let mut col = lerp_rgb(bg, arm_col, arm_glow * 0.92);
    col = lerp_rgb(col, core, falloff.powf(1.6));

    // Crisp procedural stars
    let sx = ((u * 29.0).sin() * (v * 37.0).cos()).abs();
    let sy = ((u * 53.0 + v * 17.0).sin()).abs();
    if sx > 0.96 && sy > 0.92 {
        col = [255.0, 250.0, 235.0];
    }

    col
}

fn retro_mountain(u: f32, v: f32) -> [f32; 3] {
    // Warm sunlit sky
    let mut col = lerp_rgb([42.0, 74.0, 134.0], [250.0, 186.0, 118.0], (v / 0.66).clamp(0.0, 1.0));

    // Sun orb
    let sd = ((u - 0.74).powi(2) + (v - 0.24).powi(2)).sqrt();
    if sd < 0.095 {
        col = lerp_rgb(col, [255.0, 242.0, 178.0], 1.0 - smoothstep(0.085, 0.095, sd));
    }

    // Twin geometric peaks with lit right slope and shaded left slope
    let p1 = 0.22 + (u - 0.34).abs() * 1.15;
    let p2 = 0.30 + (u - 0.66).abs() * 1.10;
    let lake_y = 0.68;

    if v < lake_y {
        if v >= p2 {
            let snow = v < 0.42 + (u - 0.66).abs() * 0.4;
            col = if u >= 0.66 {
                if snow {
                    [248.0, 246.0, 244.0]
                } else {
                    [126.0, 146.0, 176.0]
                }
            } else if snow {
                [192.0, 206.0, 224.0]
            } else {
                [56.0, 74.0, 110.0]
            };
        }
        if v >= p1 {
            let snow = v < 0.37 + (u - 0.34).abs() * 0.45;
            col = if u >= 0.34 {
                if snow {
                    [252.0, 248.0, 240.0]
                } else {
                    [112.0, 132.0, 166.0]
                }
            } else if snow {
                [178.0, 196.0, 222.0]
            } else {
                [42.0, 58.0, 94.0]
            };
        }
        // Forest foothill band
        let tree_line = 0.63 + 0.025 * (u * TAU * 7.0).sin();
        if v >= tree_line {
            col = [20.0, 54.0, 62.0];
        }
    } else {
        let wt = (v - lake_y) / (1.0 - lake_y);
        col = lerp_rgb([46.0, 138.0, 162.0], [18.0, 52.0, 86.0], wt);
        if (u - 0.74).abs() < 0.075 && ((v * 48.0).sin() > 0.0) {
            col = lerp_rgb(col, [255.0, 224.0, 152.0], 0.65);
        }
    }

    col
}

fn mondrian(u: f32, v: f32) -> [f32; 3] {
    // Crisp Piet Mondrian De Stijl composition
    let black = [18.0, 18.0, 22.0];
    let white = [245.0, 242.0, 232.0];
    let red = [216.0, 38.0, 34.0];
    let blue = [24.0, 74.0, 168.0];
    let yellow = [244.0, 204.0, 36.0];

    let bar = 0.022;
    let v1 = 0.26;
    let v2 = 0.78;
    let h1 = 0.30;
    let h2 = 0.72;
    let h3 = 0.86;

    // Structural black grid lines
    if (u - v1).abs() < bar
        || ((u - v2).abs() < bar && v > h2)
        || (v - h2).abs() < bar
        || ((v - h1).abs() < bar && u < v1)
        || ((v - h3).abs() < bar && u > v2)
    {
        return black;
    }

    if u > v1 && v < h2 {
        red
    } else if u < v1 && v > h2 {
        blue
    } else if u > v2 && v > h3 {
        yellow
    } else {
        white
    }
}
