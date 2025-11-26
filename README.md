# EQ Layer

A real-time audio equalizer application built in Rust that applies parametric EQ filters to audio streams. It supports both a graphical user interface (GUI) and command-line interface (CLI).

## Features

- Real-time audio processing with configurable latency
- Parametric equalizer with multiple filter types
- Support for Equalizer APO-compatible EQ profiles
- Graphical user interface built with egui
- Configurable input and output audio devices
- Persistent configuration storage

## Supported Filter Types

| Type | Abbreviations | Description |
|------|---------------|-------------|
| Peak | PK, Peak | Bell/peaking filter |
| Low Shelf | LSC, LowShelf | Low shelf filter |
| High Shelf | HSC, HighShelf | High shelf filter |
| Low Pass | LP, LowPass | Low pass filter |
| High Pass | HP, HighPass | High pass filter |
| Band Pass | BP | Band pass filter |
| Notch | NO, Notch | Notch filter |
| All Pass | AP | All pass filter |

## Requirements

- Rust (edition 2024)
- A compatible audio driver/backend (ALSA on Linux, CoreAudio on macOS, WASAPI on Windows)

## Installation

```bash
git clone https://github.com/Moeweb647252/eq_layer.git
cd eq_layer
cargo build --release
```

The compiled binary will be available at `target/release/eq_layer`.

## Usage

### GUI Mode

Run the application without arguments to launch the graphical interface:

```bash
cargo run --release
```

The GUI allows you to:
- Select input and output audio devices
- Enable/disable the equalizer
- Adjust EQ filter parameters visually
- View the frequency response graph

### Configuration

The application stores its configuration in `~/.config/eq_layer/config.toml` (on Linux/macOS) or the appropriate config directory on Windows.

Configuration includes:
- Input and output device names
- Latency settings
- EQ profile (filter settings)

## EQ Profile Format

EQ profiles follow the Equalizer APO format:

```
Preamp: -3.0 dB
Filter 1: ON PK Fc 100 Hz Gain 2.5 dB Q 1.41
Filter 2: ON LSC Fc 80 Hz Gain -2.0 dB Q 0.71
Filter 3: ON HSC Fc 10000 Hz Gain 1.5 dB Q 0.71
```

### Filter Line Syntax

```
Filter N: ON|OFF <type> Fc <frequency> Hz Gain <gain> dB Q <q_factor>
```

- `ON|OFF`: Enable or disable the filter
- `<type>`: Filter type (PK, LSC, HSC, LP, HP, BP, NO, AP)
- `Fc`: Center/cutoff frequency in Hz
- `Gain`: Gain in dB (for filters that support it)
- `Q`: Q factor (quality factor)

## Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

## License

See the repository for license information.
