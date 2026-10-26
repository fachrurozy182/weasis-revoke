//! Bytes that are not DICOM must give an error, never abort, in WebAssembly too: there, a
//! length read from garbage cannot be allocated and stops the whole module.

use wasm_bindgen_test::wasm_bindgen_test;
use weasis_core::dicom::read_dicom;
use weasis_core::explorer::read_file;

#[wasm_bindgen_test]
#[test]
fn text_is_not_dicom() {
    assert!(read_dicom(b"not a DICOM file").is_err());
    assert!(read_dicom(b"not dicom").is_err());
    assert!(read_dicom(&[0xff; 300]).is_err());
    assert_eq!(read_file(0, "broken.dcm", b"not dicom").errors.len(), 1);
}
