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

| Argument | Description | Example |
|----------|-------------|---------|
| `--procedural` | Enable high-detail procedural shaders for stars and planets. | `cargo run --release -- --procedural` |
| `--star <type>` | Teleport to a specific star type at the origin for rendering inspection. | `cargo run --release -- --star NeutronStar` |
| `--planet <type>` | Teleport to a specific planet type at the origin for rendering inspection. | `cargo run --release -- --planet GasGiant` |
| `--scenario <name>`| Load a specific scenario (e.g., `jupiter`, `our_system`, `milky_way`). | `cargo run --release -- --scenario jupiter` |
| `--origin` | Force the player to spawn at the coordinate origin `(0,0,0)`. | `cargo run --release -- --origin` |
| `video` | Set resolution to 1920x1080 (16:9) for landscape capture. | `cargo run --release -- video` |
| `shorts` | Set resolution to 1080x1920 (9:16) for portrait capture. | `cargo run --release -- shorts` |

#### Available Star Types
Use these with the `--star` parameter:
- `O_BlueGiant` (Hottest, largest, violet-blue)
- `B_BlueWhite` (Bright blue-white)
- `A_White` (Pure white)
- `F_YellowWhite` (Cream/Off-white)
- `G_YellowDwarf` (Golden yellow, Sun-like)
- `K_OrangeDwarf` (Warm orange)
- `M_RedDwarf` (Cool red-orange, most common)
- `NeutronStar` (Tiny, extreme brightness, violet-white)
- `BlackHole` (Gravitational singularity, black core)

#### Available Planet Types
Use these with the `--planet` parameter:
- `Terran` (Earth-like with life/water)
- `Ice` (Cold, high albedo, heavy clouds)
- `Magma` (Volcanic, glowing surface, thick haze)
- `GasGiant` (Massive, dense clouds, low rim power)
- `Desert` (Arid, sweeping winds, high dust/haze)
- `Ocean` (Water world, high specular shine)

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
