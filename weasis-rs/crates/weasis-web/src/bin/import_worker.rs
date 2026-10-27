//! The import worker: reads the files of an import request away from the interface thread
//! (decision D3) and posts, as JSON, an `ImportEvent::Read` per file then `Finished`.
//!
//! It posts `Ready` once started. A request is an array: an `ImportRequest` as JSON, then the
//! `File`s; requests are handled one after the other.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/LoadLocalDicom.java`
//! (the reading threads of `Importer`), `imp/DicomDirLoader.java` and `imp/DicomZipMediaIO.java`.

use js_sys::{Array, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use weasis_core::explorer::{
    FileKind, HEAD_LENGTH, ImportEvent, ImportRequest, ImportedFile, classify, dicomdir_selection,
    read_archive, read_file,
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

fn import(message: &Array) {
    let request = message
        .get(0)
        .as_string()
        .and_then(|json| serde_json::from_str(&json).ok());
    let files: Vec<File> = message
        .iter()
        .skip(1)
        .filter_map(|value| value.dyn_into().ok())
        .collect();
    match (FileReaderSync::new().ok(), request) {
        (Some(reader), Some(ImportRequest::Files { password })) => {
            for (index, file) in files.iter().enumerate() {
                post(&ImportEvent::Read(read(
                    &reader,
                    index,
                    file,
                    password.as_deref(),
                )));
            }
        }
        (Some(reader), Some(ImportRequest::Dicomdir { paths })) => {
            let selection = dicomdir_selection(&paths, |index| bytes(&reader, files.get(index)?));
            match selection {
                Some(selected) => {
                    for index in selected {
                        let (Some(file), Some(path)) = (files.get(index), paths.get(index)) else {
                            continue;
                        };
                        let read = match bytes(&reader, file) {
                            Some(content) => read_file(index, path, &content),
                            None => failed(index, file),
                        };
                        post(&ImportEvent::Read(read));
                    }
                }
                None => post(&ImportEvent::DicomdirEmpty),
            }
        }
        _ => {
            for (index, file) in files.iter().enumerate() {
                post(&ImportEvent::Read(failed(index, file)));
            }
        }
    }
    post(&ImportEvent::Finished);
}

fn read(
    reader: &FileReaderSync,
    index: usize,
    file: &File,
    password: Option<&str>,
) -> ImportedFile {
    let name = file.name();
    let head = file
        .slice_with_i32_and_i32(0, HEAD_LENGTH as i32)
        .ok()
        .and_then(|head| bytes(reader, &head));
    let Some(head) = head else {
        return failed(index, file);
    };
    let kind = classify(&name, &head);
    if !matches!(kind, FileKind::Dicom | FileKind::Zip) {
        // TODO(4.9): graphic sidecars. A DICOMDIR is imported from the DICOMDIR page.
        return ImportedFile {
            file: index,
            ..ImportedFile::default()
        };
    }
    let Some(content) = bytes(reader, file) else {
        return failed(index, file);
    };
    match kind {
        FileKind::Zip => read_archive(index, &name, &content, password.map(str::as_bytes)),
        _ => read_file(index, &name, &content),
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
