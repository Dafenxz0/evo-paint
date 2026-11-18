# evo-paint

[![CI](https://github.com/Dafenxz0/evo-paint/actions/workflows/ci.yml/badge.svg)](https://github.com/Dafenxz0/evo-paint/actions/workflows/ci.yml)

**Live Studio:** [https://dafenxz0.github.io/evo-paint/](https://dafenxz0.github.io/evo-paint/)

An evolutionary triangle painter written in zero-dependency Rust compiled to WebAssembly (`wasm32-unknown-unknown`), paired with an interactive browser studio and standalone `.svg` / `.png` export.

## How It Works

`evo-paint` evolves a fixed-size genome of translucent RGBA triangles (`50..250`, default `120`) over an initial mean-color background to approximate any target RGBA raster image:

1. **Incremental Dirty-Rectangle Rasterization:** Instead of re-rendering and re-scoring the entire canvas on every mutation, only the union bounding box of the affected triangle(s) is re-rasterized via an exact half-open scanline rasterizer into a reusable scratch buffer, computing exact `u64` RGB sum-of-squared-errors (`SSE`) delta in O(dirty pixels).
2. **Mutation Operators:**
   - **Fine Vertex Nudge (38%):** Perturbs a single vertex within ±8% of canvas dimensions.
   - **Coarse Vertex Move (12%):** Larger spatial jump (±35%) to escape local geometry traps.
   - **Color / Alpha Shift (28%):** Adjusts RGBA channels or samples the local target color under the triangle centroid.
   - **Replace / Re-seed Weak Triangle (14%):** Re-initializes low-impact (low alpha × area) or random triangles near target pixels.
   - **Z-Order Swap (8%):** Swaps the painter's-algorithm depth order of two triangles.
3. **Hill Climbing & Simulated Annealing:** Decreasing `SSE` is always accepted; slight increases can be accepted early according to `exp(-ΔMSE / T)` with an exponential cooling schedule (`T = strength * 1.6 * exp(-gen / 3500)`). Setting **Annealing Heat** to `0.00` runs pure hill climbing.
4. **Vector & Raster Export:** Exports the active triangle genome directly from Rust as a clean standalone `<svg>` document, or renders a 4× upscaled PNG in the browser.

## Studio Controls

- **Target Source:** Choose from 5 procedural presets (*Geometric Sunset*, *Cyber Eye*, *Spiral Nebula*, *Retro Mountain*, *Mondrian*) or upload / drag-and-drop any `PNG`, `JPG`, or `WebP` image.
- **Comparison Modes:** Switch between **Side-by-Side**, **Overlay Diff** (`|ΔRGB| × 2.5`), and **Split Wipe** (interactive drag divider).
- **Transport:** **Play / Pause** (`Space`), **+N Steps** manual stepping, **Reset** (new deterministic seed), and live sliders for **Triangles**, **Annealing Heat**, and **Steps / Frame**.
- **Telemetry:** Real-time RMSE convergence sparkline, Fitness %, RMSE, PSNR (dB), Acceptance Rate %, Generations/sec, and current Annealing Temperature.

## Architecture & Build

```text
src/
  lib.rs       # C/WASM FFI exports & unit test suite
  engine.rs    # Evolutionary loop, SA acceptance, mutations, genome resizing
  raster.rs    # Scanline triangle rasterizer, bbox overlap & dirty-rect SSE
  presets.rs   # 5 deterministic procedural RGBA target generators
  prng.rs      # Deterministic SplitMix64 + XorShift64* PRNG
  svg.rs       # Clean standalone SVG serializer
web/
  index.html   # Studio layout
  style.css    # Darkroom studio styling
  app.js       # WASM loader, canvas views, sparkline & PNG/SVG downloads
```

### Local Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
cargo test
cargo build --release --target wasm32-unknown-unknown
```

## License

MIT — see [LICENSE](./LICENSE).
