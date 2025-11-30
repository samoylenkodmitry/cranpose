//! Testing utilities and harness for Compose-RS

#![allow(non_snake_case)]

pub mod testing;
pub mod robot;
pub mod robot_assertions;

#[cfg(feature = "robot-app")]
pub mod robot_app;

// Re-export testing utilities
pub use testing::*;
pub use robot::*;

#[cfg(feature = "robot-app")]
pub use robot_app::*;

pub mod prelude {
    pub use crate::testing::*;
    pub use crate::robot::*;
    pub use crate::robot_assertions;

    #[cfg(feature = "robot-app")]
    pub use crate::robot_app::*;
}
