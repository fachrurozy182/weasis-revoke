//! The import worker: reads the files of an import request away from the interface thread
//! (decision D3) and posts, as JSON, an `ImportEvent::Read` per file then `Finished`.
//!
//! It posts `Ready` once started; a request is then an array of `File`s, and requests are
//! handled one after the other.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/LoadLocalDicom.java`
//! (the reading threads of `Importer`).

use js_sys::{Array, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use weasis_core::explorer::{
    FileKind, HEAD_LENGTH, ImportEvent, ImportedFile, classify, read_file,
};
use web_sys::{Blob, DedicatedWorkerGlobalScope, File, FileReaderSync, MessageEvent};

fn main() {
    console_error_panic_hook::set_once();
    let on_message = Closure::<dyn Fn(MessageEvent)>::new(|event: MessageEvent| {
        import(&Array::from(&event.data()));
    });
    scope().set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    // The handler lives as long as the worker.
    on_message.forget();
    post(&ImportEvent::Ready);
}

fn scope() -> DedicatedWorkerGlobalScope {
    js_sys::global().unchecked_into()
}

fn import(files: &Array) {
    let reader = FileReaderSync::new().ok();
    for (index, value) in files.iter().enumerate() {
        let Ok(file) = value.dyn_into::<File>() else {
            continue;
        };
        let imported = match &reader {
            Some(reader) => read(reader, index, &file),
            None => failed(index, &file),
        };
        post(&ImportEvent::Read(imported));
    }
    post(&ImportEvent::Finished);
}

fn read(reader: &FileReaderSync, index: usize, file: &File) -> ImportedFile {
    let name = file.name();
    let head = file
        .slice_with_i32_and_i32(0, HEAD_LENGTH as i32)
        .ok()
        .and_then(|head| bytes(reader, &head));
    let Some(head) = head else {
        return failed(index, file);
    };
    match classify(&name, &head) {
        FileKind::Dicom => match bytes(reader, file) {
            Some(content) => read_file(index, &name, &content),
            None => failed(index, file),
        },
        // TODO(2.4.c): ZIP archives and DICOMDIR. TODO(4.9): graphic sidecars.
        FileKind::Zip | FileKind::Dicomdir | FileKind::Sidecar | FileKind::Other => ImportedFile {
            file: index,
            ..ImportedFile::default()
        },
    }
}

fn bytes(reader: &FileReaderSync, blob: &Blob) -> Option<Vec<u8>> {
    let buffer = reader.read_as_array_buffer(blob).ok()?;
    Some(Uint8Array::new(&buffer).to_vec())
}

fn failed(index: usize, file: &File) -> ImportedFile {
    ImportedFile {
        file: index,
        errors: vec![format!("{}: cannot read the file", file.name())],
        ..ImportedFile::default()
    }
}

fn post(event: &ImportEvent) {
    match serde_json::to_string(event) {
        Ok(json) => {
            if let Err(error) = scope().post_message(&json.into()) {
                web_sys::console::error_1(&error);
            }
        }
        Err(error) => web_sys::console::error_1(&error.to_string().into()),
    }
}
