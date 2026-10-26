//! Core logic of Weasis, shared by the browser (`weasis-web`) and the server (`weasis-server`).
//!
//! This crate must not depend on web APIs or networking so that it can be tested with plain
//! `cargo test`. See `weasis-rs/docs/PLAN.md` §3.
//!
//! Java reference: `weasis-core`, `weasis-dicom/weasis-dicom-codec`, and the external
//! libraries `weasis-dicom-tools` and `weasis-core-img`.

pub mod model;
