//! Fast scanline triangle rasterizer and dirty-rectangle SSE evaluation.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BBox {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl BBox {
    pub const EMPTY: Self = Self {
        min_x: 1,
        min_y: 1,
        max_x: 0,
        max_y: 0,
    };

    #[inline]
    pub fn is_empty(self) -> bool {
        self.min_x > self.max_x || self.min_y > self.max_y
    }

    #[inline]
    pub fn width(self) -> i32 {
        if self.is_empty() {
            0
        } else {
            self.max_x - self.min_x + 1
        }
    }

    #[inline]
    pub fn height(self) -> i32 {
        if self.is_empty() {
            0
        } else {
            self.max_y - self.min_y + 1
        }
    }

    #[inline]
    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    #[inline]
    pub fn overlaps(self, other: Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }
}

/// Computes the tight pixel-center bounding box of a triangle clipped to [0, width) x [0, height).
pub fn triangle_bbox(tri: &Triangle, width: i32, height: i32) -> BBox {
    if tri.a == 0 || width <= 0 || height <= 0 {
        return BBox::EMPTY;
    }
    let min_xf = tri.x0.min(tri.x1).min(tri.x2);
    let max_xf = tri.x0.max(tri.x1).max(tri.x2);
    let min_yf = tri.y0.min(tri.y1).min(tri.y2);
    let max_yf = tri.y0.max(tri.y1).max(tri.y2);

    let min_x = ((min_xf - 0.5).ceil() as i32).max(0);
    let max_x = (((max_xf - 0.5).ceil() as i32) - 1).min(width - 1);
    let min_y = ((min_yf - 0.5).ceil() as i32).max(0);
    let max_y = (((max_yf - 0.5).ceil() as i32) - 1).min(height - 1);

    if min_x > max_x || min_y > max_y {
        BBox::EMPTY
    } else {
        BBox {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }
}

/// Fills an RGBA buffer representing a rectangle of size (buf_w x buf_h) with an opaque RGB background.
pub fn clear_rgba(buf: &mut [u8], pixel_count: usize, bg: [u8; 3]) {
    let needed = pixel_count * 4;
    for chunk in buf[..needed].as_chunks_mut::<4>().0 {
        chunk[0] = bg[0];
        chunk[1] = bg[1];
        chunk[2] = bg[2];
        chunk[3] = 255;
    }
}

/// Alpha-blends a single triangle into `buf` (which covers `[buf_x0, buf_x0 + buf_w) x [buf_y0, ...)`),
/// clipped to `clip`.
pub fn rasterize_triangle(
    buf: &mut [u8],
    buf_x0: i32,
    buf_y0: i32,
    buf_w: i32,
    clip: BBox,
    tri: &Triangle,
) {
    if tri.a == 0 || clip.is_empty() || buf_w <= 0 {
        return;
    }

    let mut pts = [(tri.x0, tri.y0), (tri.x1, tri.y1), (tri.x2, tri.y2)];
    if pts[0].1 > pts[1].1 {
        pts.swap(0, 1);
    }
    if pts[1].1 > pts[2].1 {
        pts.swap(1, 2);
    }
    if pts[0].1 > pts[1].1 {
        pts.swap(0, 1);
    }

    let (ax, ay) = pts[0];
    let (bx, by) = pts[1];
    let (cx, cy) = pts[2];
    let total_h = cy - ay;
    if total_h < 1e-4 {
        return;
    }

    let min_xf = ax.min(bx).min(cx);
    let max_xf = ax.max(bx).max(cx);

    let py_start = ((ay - 0.5).ceil() as i32).max(clip.min_y);
    let py_end = (((cy - 0.5).ceil() as i32) - 1).min(clip.max_y);
    if py_start > py_end {
        return;
    }

    let sa = u32::from(tri.a);
    let inv_a = 255 - sa;
    let sr = u32::from(tri.r) * sa;
    let sg = u32::from(tri.g) * sa;
    let sb = u32::from(tri.b) * sa;

    let inv_total_h = 1.0 / total_h;
    let seg_top = by - ay;
    let seg_bot = cy - by;

    for py in py_start..=py_end {
        let sy = py as f32 + 0.5;
        let x_ac = ax + (cx - ax) * ((sy - ay) * inv_total_h);
        let x_short = if sy < by {
            if seg_top > 1e-5 {
                ax + (bx - ax) * ((sy - ay) / seg_top)
            } else {
                ax
            }
        } else if seg_bot > 1e-5 {
            bx + (cx - bx) * ((sy - by) / seg_bot)
        } else {
            bx
        };

        let (x_l, x_r) = if x_ac <= x_short {
            (x_ac.max(min_xf), x_short.min(max_xf))
        } else {
            (x_short.max(min_xf), x_ac.min(max_xf))
        };

        let px_start = ((x_l - 0.5).ceil() as i32).max(clip.min_x);
        let px_end = (((x_r - 0.5).ceil() as i32) - 1).min(clip.max_x);
        if px_start > px_end {
            continue;
        }

        let row_offset = (py - buf_y0) * buf_w;
        let col_start = (row_offset + (px_start - buf_x0)) as usize * 4;
        let col_end = (row_offset + (px_end - buf_x0) + 1) as usize * 4;
        for px in buf[col_start..col_end].as_chunks_mut::<4>().0 {
            px[0] = ((sr + u32::from(px[0]) * inv_a + 127) / 255) as u8;
            px[1] = ((sg + u32::from(px[1]) * inv_a + 127) / 255) as u8;
            px[2] = ((sb + u32::from(px[2]) * inv_a + 127) / 255) as u8;
        }
    }
}

/// Renders all triangles into a full-frame RGBA buffer.
pub fn render_full(
    canvas: &mut [u8],
    width: i32,
    height: i32,
    bg: [u8; 3],
    triangles: &[Triangle],
) {
    let pixel_count = (width * height) as usize;
    clear_rgba(canvas, pixel_count, bg);
    let full = BBox {
        min_x: 0,
        min_y: 0,
        max_x: width - 1,
        max_y: height - 1,
    };
    for tri in triangles {
        rasterize_triangle(canvas, 0, 0, width, full, tri);
    }
}

/// Renders only triangles overlapping `rect` into a compact `rect.width() * rect.height() * 4` buffer.
pub fn render_rect(
    rect_buf: &mut [u8],
    rect: BBox,
    bg: [u8; 3],
    triangles: &[Triangle],
    bboxes: &[BBox],
) {
    let rw = rect.width();
    let rh = rect.height();
    if rw <= 0 || rh <= 0 {
        return;
    }
    clear_rgba(rect_buf, (rw * rh) as usize, bg);
    for (tri, &bbox) in triangles.iter().zip(bboxes.iter()) {
        if bbox.overlaps(rect) {
            rasterize_triangle(rect_buf, rect.min_x, rect.min_y, rw, rect, tri);
        }
    }
}

/// Copies a compact `rect` buffer back into the full `canvas`.
pub fn blit_rect(rect_buf: &[u8], rect: BBox, canvas: &mut [u8], img_w: i32) {
    let rw = rect.width();
    let rh = rect.height();
    if rw <= 0 || rh <= 0 {
        return;
    }
    let row_bytes = (rw as usize) * 4;
    for dy in 0..rh {
        let src_start = (dy as usize) * row_bytes;
        let dst_start = (((rect.min_y + dy) * img_w + rect.min_x) as usize) * 4;
        canvas[dst_start..dst_start + row_bytes]
            .copy_from_slice(&rect_buf[src_start..src_start + row_bytes]);
    }
}

/// Computes exact RGB sum-of-squared-errors inside `rect` between a buffer (origin `buf_x0, buf_y0`, stride `buf_w`)
/// and the full-frame `target` (stride `img_w`).
pub fn region_sse(
    buf: &[u8],
    buf_x0: i32,
    buf_y0: i32,
    buf_w: i32,
    target: &[u8],
    img_w: i32,
    rect: BBox,
) -> u64 {
    if rect.is_empty() {
        return 0;
    }
    let mut sse = 0_u64;
    let count = rect.width() as usize;
    for py in rect.min_y..=rect.max_y {
        let b_start = (((py - buf_y0) * buf_w + (rect.min_x - buf_x0)) as usize) * 4;
        let t_start = ((py * img_w + rect.min_x) as usize) * 4;
        let b_row = &buf[b_start..b_start + count * 4];
        let t_row = &target[t_start..t_start + count * 4];
        for (bp, tp) in b_row.as_chunks::<4>().0.iter().zip(t_row.as_chunks::<4>().0) {
            let dr = i32::from(bp[0]) - i32::from(tp[0]);
            let dg = i32::from(bp[1]) - i32::from(tp[1]);
            let db = i32::from(bp[2]) - i32::from(tp[2]);
            sse += (dr * dr + dg * dg + db * db) as u64;
        }
    }
    sse
}

/// Computes exact RGB sum-of-squared-errors over the full frame.
pub fn full_sse(canvas: &[u8], target: &[u8]) -> u64 {
    let mut sse = 0_u64;
    for (cp, tp) in canvas.as_chunks::<4>().0.iter().zip(target.as_chunks::<4>().0) {
        let dr = i32::from(cp[0]) - i32::from(tp[0]);
        let dg = i32::from(cp[1]) - i32::from(tp[1]);
        let db = i32::from(cp[2]) - i32::from(tp[2]);
        sse += (dr * dr + dg * dg + db * db) as u64;
    }
    sse
}
