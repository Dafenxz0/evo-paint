const PRESETS = [
  { id: 0, name: "Sunset", title: "Geometric Sunset" },
  { id: 1, name: "Eye", title: "Cyber Eye" },
  { id: 2, name: "Nebula", title: "Spiral Nebula" },
  { id: 3, name: "Alpine", title: "Retro Mountain" },
  { id: 4, name: "Mondrian", title: "Mondrian" },
];

const DEFAULT_W = 192;
const DEFAULT_H = 192;
const MAX_UPLOAD_DIM = 200;

const els = {
  playPauseBtn: document.getElementById("playPauseBtn"),
  stepBtn: document.getElementById("stepBtn"),
  resetBtn: document.getElementById("resetBtn"),
  downloadSvgBtn: document.getElementById("downloadSvgBtn"),
  downloadPngBtn: document.getElementById("downloadPngBtn"),
  imageInput: document.getElementById("imageInput"),
  presetGrid: document.getElementById("presetGrid"),
  wasmStatus: document.getElementById("wasmStatus"),
  triCountSlider: document.getElementById("triCountSlider"),
  triCountVal: document.getElementById("triCountVal"),
  saSlider: document.getElementById("saSlider"),
  saVal: document.getElementById("saVal"),
  batchSlider: document.getElementById("batchSlider"),
  batchVal: document.getElementById("batchVal"),
  sparklineCanvas: document.getElementById("sparklineCanvas"),
  tempVal: document.getElementById("tempVal"),
  fitnessVal: document.getElementById("fitnessVal"),
  rmseVal: document.getElementById("rmseVal"),
  psnrVal: document.getElementById("psnrVal"),
  acceptRateVal: document.getElementById("acceptRateVal"),
  genRateVal: document.getElementById("genRateVal"),
  genTotalVal: document.getElementById("genTotalVal"),
  dimensionsInfo: document.getElementById("dimensionsInfo"),
  sourceLabel: document.getElementById("sourceLabel"),
  acceptedLabel: document.getElementById("acceptedLabel"),
  targetCanvas: document.getElementById("targetCanvas"),
  evolvedCanvas: document.getElementById("evolvedCanvas"),
  diffCanvas: document.getElementById("diffCanvas"),
  splitCanvas: document.getElementById("splitCanvas"),
  stageDual: document.getElementById("stageDual"),
  stageSplit: document.getElementById("stageSplit"),
  splitFrame: document.getElementById("splitFrame"),
  splitHandle: document.getElementById("splitHandle"),
  targetDropZone: document.getElementById("targetDropZone"),
};

let wasm = null;
let enginePtr = 0;
let running = true;
let activePreset = 0;
let activeSourceName = PRESETS[0].title;
let viewMode = "side";
let splitRatio = 0.5;
let seedCounter = 42;
let rmseHistory = [];
let genRate = 0;
let lastRateTime = performance.now();
let lastRateGen = 0;

async function loadWasm() {
  const response = await fetch("./evo_paint.wasm");
  if (!response.ok) {
    throw new Error(`Failed to fetch evo_paint.wasm (${response.status})`);
  }
  const bytes = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return instance.exports;
}

function createEngine(w, h, triCount, seed) {
  if (enginePtr !== 0) {
    wasm.engine_destroy(enginePtr);
  }
  enginePtr = wasm.engine_create(w, h, triCount, seed);
  wasm.engine_set_sa_strength(enginePtr, parseFloat(els.saSlider.value));
  resizeCanvases(w, h);
  rmseHistory = [];
  lastRateTime = performance.now();
  lastRateGen = 0;
  genRate = 0;
}

function resizeCanvases(w, h) {
  for (const c of [els.targetCanvas, els.evolvedCanvas, els.diffCanvas]) {
    c.width = w;
    c.height = h;
  }
  els.splitCanvas.width = w;
  els.splitCanvas.height = h;
  updateDimensionsLabel();
}

function updateDimensionsLabel() {
  if (!wasm || enginePtr === 0) return;
  const w = wasm.engine_width(enginePtr);
  const h = wasm.engine_height(enginePtr);
  const count = wasm.engine_triangle_count(enginePtr);
  els.dimensionsInfo.textContent = `${w} × ${h} px · ${count} triangles`;
}

function drawTargetAndEvolved() {
  const w = wasm.engine_width(enginePtr);
  const h = wasm.engine_height(enginePtr);
  const byteLen = wasm.engine_pixel_bytes(enginePtr);
  const mem = new Uint8ClampedArray(wasm.memory.buffer);

  const tgtPtr = wasm.engine_target_ptr(enginePtr);
  const tgtSlice = new Uint8ClampedArray(mem.buffer, tgtPtr, byteLen);
  const tgtImg = new ImageData(new Uint8ClampedArray(tgtSlice), w, h);
  els.targetCanvas.getContext("2d").putImageData(tgtImg, 0, 0);

  const evoPtr = wasm.engine_canvas_ptr(enginePtr);
  const evoSlice = new Uint8ClampedArray(mem.buffer, evoPtr, byteLen);
  const evoImg = new ImageData(new Uint8ClampedArray(evoSlice), w, h);
  els.evolvedCanvas.getContext("2d").putImageData(evoImg, 0, 0);

  if (viewMode === "diff") {
    const diffData = new Uint8ClampedArray(byteLen);
    for (let i = 0; i < byteLen; i += 4) {
      diffData[i] = Math.min(255, Math.abs(evoSlice[i] - tgtSlice[i]) * 2.5);
      diffData[i + 1] = Math.min(255, Math.abs(evoSlice[i + 1] - tgtSlice[i + 1]) * 2.5);
      diffData[i + 2] = Math.min(255, Math.abs(evoSlice[i + 2] - tgtSlice[i + 2]) * 2.5);
      diffData[i + 3] = 255;
    }
    els.diffCanvas.getContext("2d").putImageData(new ImageData(diffData, w, h), 0, 0);
  } else if (viewMode === "split") {
    const ctx = els.splitCanvas.getContext("2d");
    const splitX = Math.round(w * splitRatio);
    ctx.putImageData(evoImg, 0, 0);
    if (splitX > 0) {
      ctx.putImageData(tgtImg, 0, 0, 0, 0, splitX, h);
    }
  }
}

function drawSparkline() {
  const c = els.sparklineCanvas;
  const ctx = c.getContext("2d");
  const w = c.width;
  const h = c.height;
  ctx.clearRect(0, 0, w, h);

  ctx.strokeStyle = "#1d2633";
  ctx.lineWidth = 1;
  for (let i = 1; i < 4; i += 1) {
    const y = Math.round((h * i) / 4) + 0.5;
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(w, y);
    ctx.stroke();
  }

  if (rmseHistory.length < 2) return;
  let min = Infinity;
  let max = -Infinity;
  for (const v of rmseHistory) {
    if (v < min) min = v;
    if (v > max) max = v;
  }
  const span = Math.max(1.0, max - min);

  ctx.strokeStyle = "#22d3ee";
  ctx.lineWidth = 2;
  ctx.beginPath();
  for (let i = 0; i < rmseHistory.length; i += 1) {
    const x = (i / (rmseHistory.length - 1)) * (w - 8) + 4;
    const norm = (rmseHistory[i] - min) / span;
    const y = h - 8 - (1 - norm) * (h - 16);
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();
}

function updateTelemetry() {
  const gen = wasm.engine_generation(enginePtr);
  const accepted = wasm.engine_accepted(enginePtr);
  const rmse = wasm.engine_rmse(enginePtr);
  const psnr = wasm.engine_psnr(enginePtr);
  const fitness = wasm.engine_fitness_pct(enginePtr);
  const acceptRate = wasm.engine_acceptance_rate(enginePtr);
  const temp = wasm.engine_temperature(enginePtr);

  const now = performance.now();
  const dt = (now - lastRateTime) / 1000;
  if (dt >= 0.35) {
    genRate = (gen - lastRateGen) / dt;
    lastRateTime = now;
    lastRateGen = gen;
  }

  if (rmseHistory.length === 0 || gen === 0 || rmseHistory.length < 180) {
    if (rmseHistory.length === 0 || Math.abs(rmseHistory[rmseHistory.length - 1] - rmse) > 1e-4) {
      rmseHistory.push(rmse);
    }
  } else {
    rmseHistory.push(rmse);
    rmseHistory.shift();
  }

  els.fitnessVal.textContent = `${fitness.toFixed(2)}%`;
  els.rmseVal.textContent = rmse.toFixed(2);
  els.psnrVal.textContent = `${psnr.toFixed(2)} dB`;
  els.acceptRateVal.textContent = `${(acceptRate * 100).toFixed(1)}%`;
  els.genRateVal.textContent = `${Math.round(genRate).toLocaleString()}/s`;
  els.genTotalVal.textContent = Math.round(gen).toLocaleString();
  els.acceptedLabel.textContent = `${Math.round(accepted).toLocaleString()} accepted`;
  els.tempVal.textContent = `T ${temp.toFixed(3)}`;
  drawSparkline();
}

function buildPresetGallery() {
  els.presetGrid.innerHTML = "";
  const thumbW = 48;
  const thumbH = 48;
  const thumbEngine = wasm.engine_create(thumbW, thumbH, 1, 1);

  for (const preset of PRESETS) {
    wasm.engine_load_preset(thumbEngine, preset.id);
    const bytes = wasm.engine_pixel_bytes(thumbEngine);
    const ptr = wasm.engine_target_ptr(thumbEngine);
    const rgba = new Uint8ClampedArray(wasm.memory.buffer, ptr, bytes);

    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `preset-btn${preset.id === activePreset ? " active" : ""}`;
    btn.dataset.preset = String(preset.id);
    btn.title = preset.title;

    const thumb = document.createElement("canvas");
    thumb.width = thumbW;
    thumb.height = thumbH;
    thumb.getContext("2d").putImageData(new ImageData(new Uint8ClampedArray(rgba), thumbW, thumbH), 0, 0);

    const label = document.createElement("span");
    label.textContent = preset.name;

    btn.append(thumb, label);
    btn.addEventListener("click", () => selectPreset(preset.id));
    els.presetGrid.appendChild(btn);
  }

  wasm.engine_destroy(thumbEngine);
}

function selectPreset(presetId) {
  activePreset = presetId;
  const preset = PRESETS.find((p) => p.id === presetId) ?? PRESETS[0];
  activeSourceName = preset.title;
  els.sourceLabel.textContent = activeSourceName;

  for (const btn of els.presetGrid.querySelectorAll(".preset-btn")) {
    btn.classList.toggle("active", Number(btn.dataset.preset) === presetId);
  }

  const w = wasm.engine_width(enginePtr);
  const h = wasm.engine_height(enginePtr);
  if (w !== DEFAULT_W || h !== DEFAULT_H) {
    createEngine(DEFAULT_W, DEFAULT_H, Number(els.triCountSlider.value), seedCounter);
  }
  wasm.engine_load_preset(enginePtr, presetId);
  rmseHistory = [];
  lastRateTime = performance.now();
  lastRateGen = 0;
  drawTargetAndEvolved();
  updateTelemetry();
}

function loadUploadedImage(file) {
  if (!file || !file.type.startsWith("image/")) return;
  const url = URL.createObjectURL(file);
  const img = new Image();
  img.onload = () => {
    URL.revokeObjectURL(url);
    const scale = Math.min(1, MAX_UPLOAD_DIM / Math.max(img.width, img.height));
    const w = Math.max(16, Math.round(img.width * scale));
    const h = Math.max(16, Math.round(img.height * scale));

    const off = document.createElement("canvas");
    off.width = w;
    off.height = h;
    const ctx = off.getContext("2d");
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, w, h);
    ctx.drawImage(img, 0, 0, w, h);
    const imgData = ctx.getImageData(0, 0, w, h).data;

    createEngine(w, h, Number(els.triCountSlider.value), seedCounter);
    const ptr = wasm.alloc(imgData.length);
    new Uint8Array(wasm.memory.buffer, ptr, imgData.length).set(imgData);
    wasm.engine_set_target_rgba(enginePtr, ptr, imgData.length);
    wasm.dealloc(ptr, imgData.length);

    activePreset = -1;
    activeSourceName = file.name.slice(0, 28);
    els.sourceLabel.textContent = activeSourceName;
    for (const btn of els.presetGrid.querySelectorAll(".preset-btn")) {
      btn.classList.remove("active");
    }

    drawTargetAndEvolved();
    updateTelemetry();
  };
  img.onerror = () => URL.revokeObjectURL(url);
  img.src = url;
}

function downloadSvg() {
  const len = wasm.engine_export_svg(enginePtr);
  const ptr = wasm.engine_svg_ptr(enginePtr);
  const bytes = new Uint8Array(wasm.memory.buffer, ptr, len);
  const text = new TextDecoder().decode(bytes);
  const blob = new Blob([text], { type: "image/svg+xml;charset=utf-8" });
  triggerDownload(blob, `evo-paint-${Math.round(wasm.engine_generation(enginePtr))}.svg`);
}

function downloadPng() {
  const svgLen = wasm.engine_export_svg(enginePtr);
  const svgPtr = wasm.engine_svg_ptr(enginePtr);
  const svgBytes = new Uint8Array(wasm.memory.buffer, svgPtr, svgLen);
  const svgText = new TextDecoder().decode(svgBytes);
  const w = wasm.engine_width(enginePtr);
  const h = wasm.engine_height(enginePtr);
  const scale = 4;

  const svgBlob = new Blob([svgText], { type: "image/svg+xml;charset=utf-8" });
  const url = URL.createObjectURL(svgBlob);
  const img = new Image();
  img.onload = () => {
    const out = document.createElement("canvas");
    out.width = w * scale;
    out.height = h * scale;
    out.getContext("2d").drawImage(img, 0, 0, out.width, out.height);
    URL.revokeObjectURL(url);
    out.toBlob((blob) => {
      if (blob) {
        triggerDownload(blob, `evo-paint-${Math.round(wasm.engine_generation(enginePtr))}.png`);
      }
    }, "image/png");
  };
  img.onerror = () => {
    URL.revokeObjectURL(url);
    els.evolvedCanvas.toBlob((blob) => {
      if (blob) {
        triggerDownload(blob, `evo-paint-${Math.round(wasm.engine_generation(enginePtr))}.png`);
      }
    }, "image/png");
  };
  img.src = url;
}

function triggerDownload(blob, filename) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function setRunning(next) {
  running = next;
  els.playPauseBtn.textContent = running ? "Pause" : "Play";
}

function setViewMode(mode) {
  viewMode = mode;
  for (const btn of document.querySelectorAll(".seg-btn")) {
    btn.classList.toggle("active", btn.dataset.mode === mode);
  }
  els.stageDual.classList.toggle("hidden", mode === "split");
  els.stageDual.classList.toggle("with-diff", mode === "diff");
  els.stageSplit.classList.toggle("hidden", mode !== "split");
  if (wasm && enginePtr !== 0) {
    drawTargetAndEvolved();
  }
}

function wireEvents() {
  els.playPauseBtn.addEventListener("click", () => setRunning(!running));

  els.stepBtn.addEventListener("click", () => {
    const steps = Number(els.batchSlider.value);
    wasm.engine_step(enginePtr, steps);
    drawTargetAndEvolved();
    updateTelemetry();
  });

  els.resetBtn.addEventListener("click", () => {
    seedCounter += 1;
    wasm.engine_reset(enginePtr, seedCounter);
    rmseHistory = [];
    lastRateTime = performance.now();
    lastRateGen = 0;
    drawTargetAndEvolved();
    updateTelemetry();
  });

  els.triCountSlider.addEventListener("input", () => {
    const val = Number(els.triCountSlider.value);
    els.triCountVal.textContent = String(val);
    wasm.engine_set_triangle_count(enginePtr, val);
    updateDimensionsLabel();
    drawTargetAndEvolved();
    updateTelemetry();
  });

  els.saSlider.addEventListener("input", () => {
    const val = parseFloat(els.saSlider.value);
    els.saVal.textContent = val.toFixed(2);
    wasm.engine_set_sa_strength(enginePtr, val);
  });

  els.batchSlider.addEventListener("input", () => {
    const val = Number(els.batchSlider.value);
    els.batchVal.textContent = String(val);
    els.stepBtn.textContent = `+${val} Steps`;
  });

  els.imageInput.addEventListener("change", (e) => {
    const file = e.target.files?.[0];
    if (file) loadUploadedImage(file);
    e.target.value = "";
  });

  els.targetDropZone.addEventListener("dragover", (e) => {
    e.preventDefault();
    els.targetDropZone.classList.add("drag-over");
  });
  els.targetDropZone.addEventListener("dragleave", () => {
    els.targetDropZone.classList.remove("drag-over");
  });
  els.targetDropZone.addEventListener("drop", (e) => {
    e.preventDefault();
    els.targetDropZone.classList.remove("drag-over");
    const file = e.dataTransfer?.files?.[0];
    if (file) loadUploadedImage(file);
  });

  els.downloadSvgBtn.addEventListener("click", downloadSvg);
  els.downloadPngBtn.addEventListener("click", downloadPng);

  for (const btn of document.querySelectorAll(".seg-btn")) {
    btn.addEventListener("click", () => setViewMode(btn.dataset.mode));
  }

  let draggingSplit = false;
  const updateSplit = (clientX) => {
    const rect = els.splitFrame.getBoundingClientRect();
    splitRatio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    els.splitHandle.style.left = `${(splitRatio * 100).toFixed(2)}%`;
    if (wasm && enginePtr !== 0 && viewMode === "split") {
      drawTargetAndEvolved();
    }
  };
  els.splitFrame.addEventListener("pointerdown", (e) => {
    draggingSplit = true;
    els.splitFrame.setPointerCapture(e.pointerId);
    updateSplit(e.clientX);
  });
  els.splitFrame.addEventListener("pointermove", (e) => {
    if (draggingSplit) updateSplit(e.clientX);
  });
  els.splitFrame.addEventListener("pointerup", () => {
    draggingSplit = false;
  });

  window.addEventListener("keydown", (e) => {
    if (e.code === "Space" && e.target.tagName !== "INPUT" && e.target.tagName !== "BUTTON") {
      e.preventDefault();
      setRunning(!running);
    }
  });
}

function tick() {
  if (running && wasm && enginePtr !== 0) {
    const steps = Number(els.batchSlider.value);
    wasm.engine_step(enginePtr, steps);
    drawTargetAndEvolved();
    updateTelemetry();
  }
  requestAnimationFrame(tick);
}

async function init() {
  try {
    wasm = await loadWasm();
    buildPresetGallery();
    createEngine(DEFAULT_W, DEFAULT_H, Number(els.triCountSlider.value), seedCounter);
    wasm.engine_load_preset(enginePtr, activePreset);
    drawTargetAndEvolved();
    updateTelemetry();
    wireEvents();

    els.wasmStatus.textContent = "WASM Ready";
    els.wasmStatus.classList.add("ready");
    for (const btn of [
      els.playPauseBtn,
      els.stepBtn,
      els.resetBtn,
      els.downloadSvgBtn,
      els.downloadPngBtn,
    ]) {
      btn.disabled = false;
    }

    requestAnimationFrame(tick);
  } catch (err) {
    els.wasmStatus.textContent = "WASM Error";
    console.error(err);
  }
}

init();
