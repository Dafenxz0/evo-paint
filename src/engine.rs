//! Evolutionary Triangle Painter engine (Hill Climbing + Simulated Annealing).

use crate::presets::generate_preset;
use crate::prng::Rng;
use crate::raster::{
    blit_rect, full_sse, region_sse, render_full, render_rect, triangle_bbox, BBox, Triangle,
};
use crate::svg::export_svg;

pub struct Engine {
    width: u32,
    height: u32,
    target: Vec<u8>,
    canvas: Vec<u8>,
    dirty_buf: Vec<u8>,
    bg: [u8; 3],
    triangles: Vec<Triangle>,
    bboxes: Vec<BBox>,
    target_count: usize,
    rng: Rng,
    seed: u64,
    total_sse: u64,
    generation: u64,
    accepted: u64,
    ema_accept: f64,
    sa_strength: f32,
    svg_cache: String,
}

impl Engine {
    pub fn new(width: u32, height: u32, num_triangles: u32, seed: u64) -> Self {
        let w = width.clamp(8, 512);
        let h = height.clamp(8, 512);
        let count = (num_triangles as usize).clamp(1, 500);
        let byte_len = (w * h * 4) as usize;
        let target = generate_preset(0, w, h);
        let bg = compute_mean_bg(&target);

        let mut engine = Self {
            width: w,
            height: h,
            target,
            canvas: vec![0_u8; byte_len],
            dirty_buf: vec![0_u8; byte_len],
            bg,
            triangles: Vec::with_capacity(count),
            bboxes: Vec::with_capacity(count),
            target_count: count,
            rng: Rng::new(seed),
            seed,
            total_sse: 0,
            generation: 0,
            accepted: 0,
            ema_accept: 0.0,
            sa_strength: 0.35,
            svg_cache: String::new(),
        };
        engine.reseed_genome();
        engine
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn canvas(&self) -> &[u8] {
        &self.canvas
    }

    pub fn target(&self) -> &[u8] {
        &self.target
    }

    pub fn triangles(&self) -> &[Triangle] {
        &self.triangles
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn accepted(&self) -> u64 {
        self.accepted
    }

    pub fn acceptance_rate(&self) -> f64 {
        self.ema_accept
    }

    pub fn sa_strength(&self) -> f32 {
        self.sa_strength
    }

    pub fn set_sa_strength(&mut self, strength: f32) {
        self.sa_strength = if strength.is_finite() {
            strength.clamp(0.0, 3.0)
        } else {
            0.0
        };
    }

    pub fn temperature(&self) -> f64 {
        f64::from(self.sa_strength) * 1.6 * (-(self.generation as f64) / 3500.0).exp()
    }

    pub fn total_sse(&self) -> u64 {
        self.total_sse
    }

    pub fn rmse(&self) -> f64 {
        let denom = f64::from(self.width) * f64::from(self.height) * 3.0;
        (self.total_sse as f64 / denom).sqrt()
    }

    pub fn psnr(&self) -> f64 {
        let r = self.rmse();
        if r <= 1e-6 {
            99.9
        } else {
            20.0 * (255.0 / r).log10()
        }
    }

    pub fn fitness_pct(&self) -> f64 {
        ((1.0 - self.rmse() / 255.0) * 100.0).clamp(0.0, 100.0)
    }

    pub fn load_preset(&mut self, preset_id: u32) {
        self.target = generate_preset(preset_id, self.width, self.height);
        self.bg = compute_mean_bg(&self.target);
        self.reseed_genome();
    }

    pub fn set_target_rgba(&mut self, rgba: &[u8]) -> bool {
        let expected = (self.width * self.height * 4) as usize;
        if rgba.len() != expected {
            return false;
        }
        self.target.copy_from_slice(rgba);
        self.bg = compute_mean_bg(&self.target);
        self.reseed_genome();
        true
    }

    pub fn reset(&mut self, seed: u64) {
        self.seed = seed;
        self.reseed_genome();
    }

    pub fn set_triangle_count(&mut self, num_triangles: u32) {
        let new_count = (num_triangles as usize).clamp(1, 500);
        if new_count == self.target_count && self.triangles.len() == new_count {
            return;
        }
        self.target_count = new_count;
        let w = self.width as i32;
        let h = self.height as i32;

        while self.triangles.len() < new_count {
            let tri = seed_triangle(&mut self.rng, self.width, self.height, &self.target);
            let bbox = triangle_bbox(&tri, w, h);
            self.triangles.push(tri);
            self.bboxes.push(bbox);
        }
        while self.triangles.len() > new_count {
            let drop_idx = self.find_weakest_idx();
            self.triangles.remove(drop_idx);
            self.bboxes.remove(drop_idx);
        }
        render_full(&mut self.canvas, w, h, self.bg, &self.triangles);
        self.total_sse = full_sse(&self.canvas, &self.target);
    }

    /// Runs `steps` evolutionary mutations and returns the number of accepted mutations.
    pub fn step(&mut self, steps: u32) -> u32 {
        if self.triangles.is_empty() {
            return 0;
        }
        let mut batch_accepted = 0_u32;
        for _ in 0..steps {
            if self.mutate_once() {
                batch_accepted += 1;
            }
        }
        batch_accepted
    }

    pub fn export_svg(&mut self) -> &str {
        self.svg_cache = export_svg(self.width, self.height, self.bg, &self.triangles);
        &self.svg_cache
    }

    pub fn svg_bytes(&self) -> &[u8] {
        self.svg_cache.as_bytes()
    }

    fn reseed_genome(&mut self) {
        self.rng = Rng::new(self.seed);
        self.generation = 0;
        self.accepted = 0;
        self.ema_accept = 0.0;
        self.triangles.clear();
        self.bboxes.clear();

        let w = self.width as i32;
        let h = self.height as i32;
        for _ in 0..self.target_count {
            let tri = seed_triangle(&mut self.rng, self.width, self.height, &self.target);
            let bbox = triangle_bbox(&tri, w, h);
            self.triangles.push(tri);
            self.bboxes.push(bbox);
        }

        render_full(&mut self.canvas, w, h, self.bg, &self.triangles);
        self.total_sse = full_sse(&self.canvas, &self.target);
    }

    fn mutate_once(&mut self) -> bool {
        self.generation += 1;
        let n = self.triangles.len();
        let w = self.width as i32;
        let h = self.height as i32;
        let roll = self.rng.next_f32();

        if n >= 2 && roll < 0.08 {
            let i = self.rng.range_usize(n);
            let offset = 1 + self.rng.range_usize(n - 1);
            let j = (i + offset) % n;
            let dirty = self.bboxes[i].union(self.bboxes[j]);
            if dirty.is_empty() {
                self.record_outcome(false);
                return false;
            }

            let old_region = region_sse(&self.canvas, 0, 0, w, &self.target, w, dirty);
            self.triangles.swap(i, j);
            self.bboxes.swap(i, j);
            render_rect(
                &mut self.dirty_buf,
                dirty,
                self.bg,
                &self.triangles,
                &self.bboxes,
            );
            let new_region = region_sse(
                &self.dirty_buf,
                dirty.min_x,
                dirty.min_y,
                dirty.width(),
                &self.target,
                w,
                dirty,
            );

            if self.should_accept(old_region, new_region) {
                blit_rect(&self.dirty_buf, dirty, &mut self.canvas, w);
                self.total_sse = self.total_sse - old_region + new_region;
                self.record_outcome(true);
                true
            } else {
                self.triangles.swap(i, j);
                self.bboxes.swap(i, j);
                self.record_outcome(false);
                false
            }
        } else {
            let mut idx = self.rng.range_usize(n);
            if roll >= 0.86 && self.rng.next_f32() < 0.35 {
                idx = self.find_weakest_idx();
            }

            let old_tri = self.triangles[idx];
            let old_bbox = self.bboxes[idx];
            let new_tri = if roll < 0.46 {
                self.mutate_vertex_fine(old_tri)
            } else if roll < 0.58 {
                self.mutate_vertex_coarse(old_tri)
            } else if roll < 0.86 {
                self.mutate_color(old_tri)
            } else {
                seed_triangle(&mut self.rng, self.width, self.height, &self.target)
            };

            let new_bbox = triangle_bbox(&new_tri, w, h);
            let dirty = old_bbox.union(new_bbox);
            if dirty.is_empty() {
                self.record_outcome(false);
                return false;
            }

            let old_region = region_sse(&self.canvas, 0, 0, w, &self.target, w, dirty);
            self.triangles[idx] = new_tri;
            self.bboxes[idx] = new_bbox;

            // Fast path when only color/alpha changed and bbox stayed inside old_bbox?
            // General dirty-rect render is already tightly scoped to dirty.
            render_rect(
                &mut self.dirty_buf,
                dirty,
                self.bg,
                &self.triangles,
                &self.bboxes,
            );
            let new_region = region_sse(
                &self.dirty_buf,
                dirty.min_x,
                dirty.min_y,
                dirty.width(),
                &self.target,
                w,
                dirty,
            );

            if self.should_accept(old_region, new_region) {
                blit_rect(&self.dirty_buf, dirty, &mut self.canvas, w);
                self.total_sse = self.total_sse - old_region + new_region;
                self.record_outcome(true);
                true
            } else {
                self.triangles[idx] = old_tri;
                self.bboxes[idx] = old_bbox;
                self.record_outcome(false);
                false
            }
        }
    }

    fn should_accept(&mut self, old_region: u64, new_region: u64) -> bool {
        if new_region < old_region {
            return true;
        }
        if new_region == old_region {
            return false;
        }
        let temp = self.temperature();
        if temp <= 1e-7 {
            return false;
        }
        let denom = f64::from(self.width) * f64::from(self.height) * 3.0;
        let delta_mse = (new_region - old_region) as f64 / denom;
        let prob = (-delta_mse / temp).exp();
        self.rng.next_f64() < prob
    }

    #[inline]
    fn record_outcome(&mut self, accepted: bool) {
        if accepted {
            self.accepted += 1;
        }
        let sample = if accepted { 1.0 } else { 0.0 };
        self.ema_accept = self.ema_accept * 0.996 + sample * 0.004;
    }

    fn mutate_vertex_fine(&mut self, mut tri: Triangle) -> Triangle {
        let wf = self.width as f32;
        let hf = self.height as f32;
        let sx = (wf * 0.08).max(2.0);
        let sy = (hf * 0.08).max(2.0);
        let v = self.rng.range_usize(3);
        let dx = self.rng.range_f32(-sx, sx);
        let dy = self.rng.range_f32(-sy, sy);
        apply_vertex_delta(&mut tri, v, dx, dy, wf, hf);
        tri
    }

    fn mutate_vertex_coarse(&mut self, mut tri: Triangle) -> Triangle {
        let wf = self.width as f32;
        let hf = self.height as f32;
        let v = self.rng.range_usize(3);
        let dx = self.rng.range_f32(-wf * 0.35, wf * 0.35);
        let dy = self.rng.range_f32(-hf * 0.35, hf * 0.35);
        apply_vertex_delta(&mut tri, v, dx, dy, wf, hf);
        tri
    }

    fn mutate_color(&mut self, mut tri: Triangle) -> Triangle {
        if self.rng.next_f32() < 0.25 {
            // Pull color toward target pixel under centroid for faster convergence
            let cx = ((tri.x0 + tri.x1 + tri.x2) / 3.0).clamp(0.0, (self.width - 1) as f32) as u32;
            let cy = ((tri.y0 + tri.y1 + tri.y2) / 3.0).clamp(0.0, (self.height - 1) as f32) as u32;
            let idx = ((cy * self.width + cx) * 4) as usize;
            tri.r = (i32::from(self.target[idx]) + self.rng.range_i32(-22, 22)).clamp(0, 255) as u8;
            tri.g =
                (i32::from(self.target[idx + 1]) + self.rng.range_i32(-22, 22)).clamp(0, 255) as u8;
            tri.b =
                (i32::from(self.target[idx + 2]) + self.rng.range_i32(-22, 22)).clamp(0, 255) as u8;
            tri.a = (i32::from(tri.a) + self.rng.range_i32(-30, 30)).clamp(12, 230) as u8;
            return tri;
        }

        match self.rng.range_usize(5) {
            0 => tri.r = (i32::from(tri.r) + self.rng.range_i32(-36, 36)).clamp(0, 255) as u8,
            1 => tri.g = (i32::from(tri.g) + self.rng.range_i32(-36, 36)).clamp(0, 255) as u8,
            2 => tri.b = (i32::from(tri.b) + self.rng.range_i32(-36, 36)).clamp(0, 255) as u8,
            3 => tri.a = (i32::from(tri.a) + self.rng.range_i32(-32, 32)).clamp(12, 230) as u8,
            _ => {
                tri.r = (i32::from(tri.r) + self.rng.range_i32(-20, 20)).clamp(0, 255) as u8;
                tri.g = (i32::from(tri.g) + self.rng.range_i32(-20, 20)).clamp(0, 255) as u8;
                tri.b = (i32::from(tri.b) + self.rng.range_i32(-20, 20)).clamp(0, 255) as u8;
            }
        }
        tri
    }

    fn find_weakest_idx(&self) -> usize {
        let mut best_idx = 0;
        let mut best_score = f32::MAX;
        for (i, tri) in self.triangles.iter().enumerate() {
            let area = ((tri.x1 - tri.x0) * (tri.y2 - tri.y0)
                - (tri.x2 - tri.x0) * (tri.y1 - tri.y0))
                .abs()
                * 0.5;
            let score = area * f32::from(tri.a);
            if score < best_score {
                best_score = score;
                best_idx = i;
            }
        }
        best_idx
    }

    /// Helper used in unit tests to verify rasterization consistency.
    pub fn debug_render_fresh(&self) -> Vec<u8> {
        let mut buf = vec![0_u8; (self.width * self.height * 4) as usize];
        render_full(
            &mut buf,
            self.width as i32,
            self.height as i32,
            self.bg,
            &self.triangles,
        );
        buf
    }

    /// Exposes a single scanline render helper for testing.
    pub fn debug_rasterize_single(width: u32, height: u32, bg: [u8; 3], tri: &Triangle) -> Vec<u8> {
        let mut buf = vec![0_u8; (width * height * 4) as usize];
        render_full(&mut buf, width as i32, height as i32, bg, &[*tri]);
        buf
    }
}

fn apply_vertex_delta(tri: &mut Triangle, idx: usize, dx: f32, dy: f32, wf: f32, hf: f32) {
    let min_x = -wf * 0.08;
    let max_x = wf * 1.08;
    let min_y = -hf * 0.08;
    let max_y = hf * 1.08;
    match idx {
        0 => {
            tri.x0 = (tri.x0 + dx).clamp(min_x, max_x);
            tri.y0 = (tri.y0 + dy).clamp(min_y, max_y);
        }
        1 => {
            tri.x1 = (tri.x1 + dx).clamp(min_x, max_x);
            tri.y1 = (tri.y1 + dy).clamp(min_y, max_y);
        }
        _ => {
            tri.x2 = (tri.x2 + dx).clamp(min_x, max_x);
            tri.y2 = (tri.y2 + dy).clamp(min_y, max_y);
        }
    }
}

fn compute_mean_bg(target: &[u8]) -> [u8; 3] {
    let pixels = (target.len() / 4).max(1) as u64;
    let mut sr = 0_u64;
    let mut sg = 0_u64;
    let mut sb = 0_u64;
    for chunk in target.chunks_exact(4) {
        sr += u64::from(chunk[0]);
        sg += u64::from(chunk[1]);
        sb += u64::from(chunk[2]);
    }
    [
        (sr / pixels) as u8,
        (sg / pixels) as u8,
        (sb / pixels) as u8,
    ]
}

fn seed_triangle(rng: &mut Rng, width: u32, height: u32, target: &[u8]) -> Triangle {
    let wf = width as f32;
    let hf = height as f32;
    let cx = rng.range_f32(0.0, wf);
    let cy = rng.range_f32(0.0, hf);

    let px = (cx as u32).min(width - 1);
    let py = (cy as u32).min(height - 1);
    let idx = ((py * width + px) * 4) as usize;

    let r = (i32::from(target[idx]) + rng.range_i32(-20, 20)).clamp(0, 255) as u8;
    let g = (i32::from(target[idx + 1]) + rng.range_i32(-20, 20)).clamp(0, 255) as u8;
    let b = (i32::from(target[idx + 2]) + rng.range_i32(-20, 20)).clamp(0, 255) as u8;
    let a = rng.range_i32(40, 145) as u8;

    let rad_x = rng.range_f32(wf * 0.06, wf * 0.28);
    let rad_y = rng.range_f32(hf * 0.06, hf * 0.28);
    let min_x = -wf * 0.08;
    let max_x = wf * 1.08;
    let min_y = -hf * 0.08;
    let max_y = hf * 1.08;

    Triangle {
        x0: (cx + rng.range_f32(-rad_x, rad_x)).clamp(min_x, max_x),
        y0: (cy + rng.range_f32(-rad_y, rad_y)).clamp(min_y, max_y),
        x1: (cx + rng.range_f32(-rad_x, rad_x)).clamp(min_x, max_x),
        y1: (cy + rng.range_f32(-rad_y, rad_y)).clamp(min_y, max_y),
        x2: (cx + rng.range_f32(-rad_x, rad_x)).clamp(min_x, max_x),
        y2: (cy + rng.range_f32(-rad_y, rad_y)).clamp(min_y, max_y),
        r,
        g,
        b,
        a,
    }
}
