//! A lightweight metrics facade for `Rust`.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod registry;
pub use registry::*;

#[cfg(feature = "global")]
#[cfg_attr(docsrs, doc(cfg(feature = "global")))]
pub mod global;

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
mod derive;

#[cfg(feature = "derive")]
pub use derive::*;
