pub mod core;
pub mod identity;

#[cfg(feature = "native")]
pub mod config;
#[cfg(feature = "native")]
pub mod networking;
#[cfg(feature = "native")]
pub mod persistence;
#[cfg(feature = "native")]
pub mod sync;

// Re-export key types for external use
pub use core::{models::*, error::*};
pub use identity::plc::*;

#[cfg(feature = "native")]
pub use identity::claims::*;

#[cfg(feature = "native")]
pub use persistence::database::Database;
#[cfg(feature = "native")]
pub use networking::protocol::OcmNetworking;
#[cfg(feature = "native")]
pub use sync::manager::SyncManager;