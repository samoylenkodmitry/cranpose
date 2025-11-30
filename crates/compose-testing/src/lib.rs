//! Testing utilities and harness for Compose-RS

#![allow(non_snake_case)]

pub mod robot;
pub mod testing;

// Re-export testing utilities
pub use robot::{rect_center, RobotApp, SceneSnapshot};
pub use testing::*;

pub mod prelude {
    pub use crate::robot::{rect_center, RobotApp, SceneSnapshot};
    pub use crate::testing::*;
}
