# Cranpose

Cranpose is a Jetpack Compose-inspired declarative UI framework for Rust.

## Install

```bash
cargo add cranpose
```

## Compose import

```rust
use cranpose::prelude::*;

#[composable]
fn MyApp() {
    Text("Hello, Cranpose!");
}
```

## Desktop starter

```rust
use cranpose::prelude::*;

fn main() {
    AppLauncher::new()
        .with_title("My Cranpose App")
        .with_size(800, 600)
        .run(MyApp);
}

#[composable]
fn MyApp() {
    Text("Hello, Cranpose!");
}
```

Default features enable the desktop + wgpu stack. For other targets, disable
default features and enable the platform/renderer features you need.
