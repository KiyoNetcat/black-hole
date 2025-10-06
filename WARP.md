# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Black Hole is a Stardust XR client that provides spatial storage functionality - like Windows Peek but in 3D. It allows users to minimize and restore XR clients within a radius by interacting with a minimize button that appears on controllers or hands.

## Development Commands

### Building and Running
```bash
# Build the project
cargo build

# Run the application (requires Stardust XR server to be running)
cargo run

# Build optimized release version
cargo build --release

# Run the binary directly (if available in PATH)
black-hole
```

### Development Tools
```bash
# Format code (uses project-specific rustfmt.toml)
cargo fmt

# Check for compilation errors without building
cargo check

# Run linter
cargo clippy

# Build with Nix (if available)
nix build
```

### Testing
```bash
# Run tests (if any)
cargo test
```

## Architecture Overview

### Core Components

**Main Entry Point (`main.rs`)**
- Sets up the Stardust XR client connection
- Manages the async event loop using tokio
- Handles controller and hand tracking integration
- Creates and manages BlackHole and MinimizeButton instances

**BlackHole (`black_hole.rs`)**
- Core spatial storage functionality
- Manages 3D model visualization and animations using tweening
- Handles object queries for reparentable clients
- Provides expand/contract animations with exponential easing
- Manages spatial transformations and client capture/release

**MinimizeButton (`minimize.rs`)**
- Interactive UI button component using stardust-xr-molecules
- Handles touch events from controllers and hands
- Provides visual feedback (- for open, + for closed state)
- Manages enabled/disabled state based on tracking

### Key Dependencies

**Stardust XR Ecosystem:**
- `stardust-xr-fusion`: Core client SDK for connecting to Stardust XR server
- `stardust-xr-molecules`: UI component library for buttons and interactive elements
- Uses D-Bus for communication via `zbus`

**3D and Animation:**
- `glam`: 3D math library for vectors, matrices, and transforms
- `tween`: Animation library for smooth expand/contract effects
- `mint`: Math interoperability

**Async Runtime:**
- `tokio`: Async runtime with current_thread flavor
- `tokio-stream`: Stream utilities for handling tracking events

### Spatial Architecture

The application creates spatial hierarchies:
1. **Root Spatial**: Attached to Stardust XR's spatial tree
2. **BlackHole Spatial**: Contains the 3D model and manages client capture
3. **Button Spatials**: Attached to controllers/hands for interaction
4. **Model Visuals**: 3D representation loaded from `black_hole` resource

### Event Handling Flow

1. **Initialization**: Connect to server, create spatial objects, attach to controllers/hands
2. **Frame Loop**: Process root events, handle animations, update button states
3. **Object Queries**: Monitor for reparentable clients in the XR space
4. **User Interaction**: Detect button presses, toggle BlackHole state
5. **Animation**: Smooth transitions between open/closed states with spatial scaling

## Development Dependencies

- **Rust toolchain**: Edition 2021
- **Stardust XR Server**: Must be running for development and testing
- **D-Bus**: Used for inter-process communication with XR services
- **Controllers/Hands**: For testing interaction (will fallback to static position)

## Code Style

- Uses hard tabs (configured in `rustfmt.toml`)
- Crate-level import granularity
- Async/await patterns throughout
- Error handling with `color-eyre` for enhanced error reporting

## Resource Management

- 3D models loaded via ResourceID system (`"black_hole"` namespace)
- Assets stored in `assets/` directory (includes `.blend` source files)
- Resource prefixes set via `directory_relative_path!("res")`

## Nix Support

The project includes a `flake.nix` for reproducible builds:
- Targets `x86_64-unknown-linux-musl` for static linking
- Includes resource handling in the derivation
- Provides development shell with Rust toolchain