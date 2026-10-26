//! Network access to DICOM sources: DICOMweb (QIDO/WADO-RS/STOW), WADO-URI, XML/JSON
//! manifests and DIMSE (C-ECHO, C-FIND, C-MOVE, C-GET, C-STORE).
//!
//! Browsers cannot open raw TCP connections, so all DICOM networking runs on the server
//! (decision D9 in `weasis-rs/docs/PLAN.md`).
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer` (`wado`, `rs`, `pref/node`),
//! `weasis-dicom/weasis-dicom-qr`, `weasis-dicom/weasis-dicom-send`, and
//! `weasis-dicom-tools` (`org.weasis.dicom.op`, `org.weasis.dicom.web`, `org.weasis.dicom.mf`).
