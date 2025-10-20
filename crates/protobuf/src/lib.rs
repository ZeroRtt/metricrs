//! A simple in-memory `metrics` collector compatible with the `metrics` facade

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod protos;

#[cfg(feature = "registry")]
#[cfg_attr(docsrs, doc(cfg(feature = "registry")))]
pub mod registry;

#[cfg(feature = "fetch")]
#[cfg_attr(docsrs, doc(cfg(feature = "fetch")))]
pub mod fetch;
