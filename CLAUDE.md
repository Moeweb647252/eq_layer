# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build              # Debug build
cargo build --release    # Release build (LTO, stripped, panic=abort)
cargo test               # Run all tests
cargo run --release      # Launch GUI
```

Rust edition 2024. No special feature flags or environment variables required.

## Architecture

A real-time audio equalizer that routes audio through a parametric EQ chain. Two processes run concurrently, communicating via an `mpsc::sync_channel`:

**GUI thread** (`main.rs` → `ui/`) — eframe/egui native window. User actions (toggle EQ, adjust bands, load profiles, switch devices) are sent as `Command` enum variants to the executor thread.

**Executor thread** (`executor.rs`) — command-processing loop that manages the audio pipeline. On `Command::SetState`, it spawns a cpal audio thread; on `Command::Restart`, it increments `instance_id` to signal the old audio thread to exit before spawning a new one.

### Audio pipeline (`run.rs`)

Two modes, both structured the same way:
1. **`run()`** — non-realtime. Loops with `sleep(latency)`, checking `instance_id` for restart signals.
2. **`run_realtime()`** — blocks on `receiver.recv()` for profile updates, atomically swaps the `ParametricEq` under `Arc<Mutex<>>`.

Both modes: input callback pushes samples into a ring buffer → output callback pops from ring buffer → if EQ enabled, runs through `ParametricEq::process_buffer()`.

Ring buffer size = `(sample_rate * latency_ms / 1000) * 2 channels * 2` frames.

### EQ engine (`eq.rs`)

- **`EqProfile` / `Filter` / `FilterType`** — data model, serde-serializable, also parsable from Equalizer APO text format via `FromStr`.
- **`BiquadCoeffs`** — RBJ Audio EQ Cookbook formulas for Peaking, LowShelf, HighShelf, LowPass, HighPass (f32).
- **`SimdBiquad`** — Direct Form I biquad using AArch64 NEON intrinsics, processes 4 interleaved channels in parallel.
- **`ParametricEq`** — cascade of `SimdBiquad` bands.

**Critical:** The SIMD path is `#[cfg(target_arch = "aarch64")]` only. There is no x86 SIMD fallback and no scalar fallback — on non-aarch64, `process_buffer()` is a no-op. EQ only works on Apple Silicon / ARM Linux.

### Settings & Config

- **`Settings`** — runtime shared state. `enable_eq` and `instance_id` are `Arc<Atomic*>` so both GUI and audio threads can read them lock-free. `latency` is a plain `u32` set before spawning the audio thread.
- **`Config`** — persisted as TOML to `<OS config dir>/eq_layer/config.toml`. Holds device names, latency, and the EQ profile.

### UI (`ui/`)

- **`heading.rs`** — top bar: Start/Stop, Enable/Disable EQ, device pickers, Load EQ file (rfd dialog), latency/preamp sliders, Realtime checkbox, Apply, Save, Add Band.
- **`equalizer.rs`** — horizontal scroll of per-band controls (filter type, freq slider, Q, gain, enabled, remove).
- **`graph.rs`** — frequency response plot via `egui_plot`, log-scale X axis 20Hz–20kHz, computes total magnitude from biquad coefficients using f64 precision (separate from the f32 runtime coeffs).
- **`command.rs`** — `Command` enum (the GUI↔executor protocol), `State`, `Info`.

`DerefMutHook<T>` in `utils.rs` wraps a value and fires a callback on `deref_mut()`. In realtime mode, this is used to send profile changes to the audio thread via mpsc whenever the UI modifies the profile.

### Dead code

`src/cli/mod.rs` defines a CLI interface (`cli_main()`) but is not declared as a module in `main.rs` and is not compiled. It was a previous entry point replaced by the GUI.
