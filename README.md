# The Silent Reach

An immersive space exploration and simulation engine built in Rust using the Bevy game engine. The Silent Reach features a procedurally generated universe, physics-based flight mechanics, and realistic celestial bodies.

## Features

- **Procedural Universe**: Infinite, procedurally generated star systems with diverse spectral types and planetary formations.
- **Physics-Based Flight**: Newtonian flight mechanics with 6-DOF control (Throttle, Pitch, Yaw, Roll).
- **Scale & Realism**: Vast distances and massive celestial bodies rendered with high-performance techniques.
- **Warp Drive**: Superluminal travel to traverse the vast emptiness between stars.
- **System Console**: Interact with the universe, name your discoveries, and leave notes in the persistent database.
- **Persistence**: Universe state and discoveries are saved locally using SQLite.
- **Cinematic Tools**: Built-in recording modes for high-resolution video capture.

## Controls

### Flight Controls
| Action | Key / Input |
|--------|-------------|
| **Throttle** | `Up Arrow` (Increase), `Down Arrow` (Decrease) |
| **Brake** | `Space` (Cut Throttle) |
| **Pitch** | `W` (Down), `S` (Up) |
| **Yaw** | `A` / `Left Arrow` (Left), `D` / `Right Arrow` (Right) |
| **Roll** | `Q` (Left), `E` (Right) |
| **Warp Mode** | `0` (Zero) |
| **Teleport Origin** | `O` |

### System Console
| Action | Key |
|--------|-----|
| **Open Console** | `Enter` (When near a target) |
| **Switch Field** | `Tab` |
| **Save Entry** | `Enter` |
| **Close / Cancel** | `Esc` |

*Note: Touch controls are available for compatible touch devices (e.g., iOS).*

## Installation & Usage

### Prerequisites
- [Rust Toolchain](https://www.rust-lang.org/tools/install)
- Start with a GPU that supports wgpu (Vulkan/Metal/DX12)

### Running the Project

Run the simulation in release mode for best performance:
```bash
cargo run --release
```

### CLI Arguments

Customize the experience with command-line arguments:

- **Procedural Rendering**: Enable high-detail procedural shaders.
  ```bash
  cargo run --release -- --procedural
  ```

- **Scenarios**: Load a specific starting scenario.
  ```bash
  cargo run --release -- --scenario milky_way
  ```
  *(Default is a random seed if unspecified)*

- **Video Recording**: Run with specific resolutions for capture.
  ```bash
  cargo run --release -- video   # 1920x1080
  cargo run --release -- shorts  # 1080x1920
  ```

### Black Hole Simulation
A separate binary is available for the black hole simulation:
```bash
cargo run --release --bin black_hole
```

## Technology Stack
- **Engine**: [Bevy](https://bevyengine.org/)
- **Graphics**: wgpu (WebGPU)
- **Database**: rusqlite
- **Async Runtime**: Tokio
