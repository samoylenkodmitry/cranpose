//! Testing utilities and harness for Compose-RS

#![allow(non_snake_case)]

pub mod robot;
pub mod testing;
pub mod wgpu_robot;

// Re-export testing utilities
pub use robot::{rect_center, RobotApp, SceneSnapshot};
pub use testing::*;
pub use wgpu_robot::{FrameCapture, WgpuRobotApp, WgpuRobotError};

pub mod prelude {
    pub use crate::robot::{rect_center, RobotApp, SceneSnapshot};
    pub use crate::testing::*;
    pub use crate::wgpu_robot::{FrameCapture, WgpuRobotApp, WgpuRobotError};
}
