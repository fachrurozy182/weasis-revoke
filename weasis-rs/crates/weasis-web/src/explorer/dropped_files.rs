//! Files dropped on the window, with the content of dropped folders and their subfolders.
//!
//! Java reference: `weasis-base/weasis-base-ui/src/main/java/org/weasis/base/ui/gui/WeasisWin.java`
//! (the drop handler of the main area calls `importFiles(files, true)`).

use js_sys::{Array, Promise};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{DataTransfer, File, FileSystemDirectoryEntry, FileSystemEntry, FileSystemFileEntry};

/// Whether a drag carries files from the system, rather than a tab of the window.
pub fn carries_files(data: &DataTransfer) -> bool {
    data.types()
        .iter()
        .any(|t| t.as_string().as_deref() == Some("Files"))
}

/// A dropped item: an entry of the file system (a file or a folder), or a file alone when the
/// drop has no file system behind it.
pub enum Dropped {
    /// File or folder.
    Entry(FileSystemEntry),
    /// File without entry.
    File(File),
}

/// The items of a drop; they must be taken while the drop event is handled.
pub fn dropped_entries(data: &DataTransfer) -> Vec<Dropped> {
    let items = data.items();
    (0..items.length())
        .filter_map(|i| items.get(i))
        .filter_map(|item| match item.webkit_get_as_entry().ok().flatten() {
            Some(entry) => Some(Dropped::Entry(entry)),
            None => item.get_as_file().ok().flatten().map(Dropped::File),
        })
        .collect()
}

/// The files of the dropped items, folders read recursively; unreadable entries are left out.
pub async fn files_of(dropped: Vec<Dropped>) -> Vec<File> {
    let mut files = Vec::new();
    let mut pending = Vec::new();
    for item in dropped {
        match item {
            Dropped::Entry(entry) => pending.push(entry),
            Dropped::File(file) => files.push(file),
        }
    }
    while let Some(entry) = pending.pop() {
        if entry.is_directory() {
            pending.extend(directory_content(entry.unchecked_into()).await);
        } else if let Some(file) = file_of(entry.unchecked_into()).await {
            files.push(file);
        }
    }
    files
}

async fn file_of(entry: FileSystemFileEntry) -> Option<File> {
    let promise = Promise::new(&mut |resolve, reject| {
        entry.file_with_callback_and_callback(&resolve, &reject);
    });
    JsFuture::from(promise).await.ok()?.dyn_into().ok()
}

/// Every entry of a folder: the reader gives them by batches until an empty one.
async fn directory_content(directory: FileSystemDirectoryEntry) -> Vec<FileSystemEntry> {
    let reader = directory.create_reader();
    let mut entries = Vec::new();
    loop {
        let promise = Promise::new(&mut |resolve, reject| {
            if let Err(error) = reader.read_entries_with_callback_and_callback(&resolve, &reject) {
                let _ = reject.call1(&wasm_bindgen::JsValue::NULL, &error);
            }
        });
        let Ok(batch) = JsFuture::from(promise).await else {
            break;
        };
        let batch = Array::from(&batch);
        if batch.length() == 0 {
            break;
        }
        entries.extend(batch.iter().filter_map(|e| e.dyn_into().ok()));
    }
    entries
}
