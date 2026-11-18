//! Clean standalone SVG exporter for evolved triangle genomes.

use std::fmt::Write;

use crate::raster::Triangle;

/// Exports the background rectangle and non-transparent triangles as a standalone SVG string.
pub fn export_svg(width: u32, height: u32, bg: [u8; 3], triangles: &[Triangle]) -> String {
    let mut svg = String::with_capacity(256 + triangles.len() * 96);
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\">"
    );
    let _ = writeln!(
        svg,
        "  <rect width=\"100%\" height=\"100%\" fill=\"#{:02x}{:02x}{:02x}\"/>",
        bg[0], bg[1], bg[2]
    );
    for tri in triangles {
        if tri.a == 0 {
            continue;
        }
        let opacity = f32::from(tri.a) / 255.0;
        let _ = writeln!(
            svg,
            "  <polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"rgb({},{},{})\" fill-opacity=\"{:.3}\"/>",
            tri.x0, tri.y0, tri.x1, tri.y1, tri.x2, tri.y2, tri.r, tri.g, tri.b, opacity
        );
    }
    svg.push_str("</svg>\n");
    svg
}
