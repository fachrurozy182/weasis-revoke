//! File ▸ Import ▸ DICOM: a dialog listing the import pages on the left (local device, then
//! DICOMDIR) with the selected page on the right.
//!
//! NOTE: browsers never tell the path of a chosen file, so the fields show the name of the file
//! or folder, and the last location cannot be remembered; a folder is chosen one at a time. The
//! DICOMDIR page takes the folder holding the DICOMDIR, since its files must be chosen too; the
//! CD-ROM detection and the copy into a cache of the desktop application have no use there.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/imp/DicomImport.java`,
//! `LocalImport.java` and `DicomDirImport.java`.

use leptos::prelude::*;
use web_sys::{File, HtmlInputElement};

use super::Importer;
use crate::help::online_help_url;
use crate::i18n::tr;
use crate::storage;
use crate::widgets::{Button, ButtonKind, Dialog};

/// Preference: search the subfolders of a chosen folder.
const LAST_RECURSIVE_MODE: &str = "last.recursive.mode";
/// Preference: the page shown when the dialog opens.
const LAST_PAGE: &str = "last.dicom.import.page";

/// A page of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Page {
    Local,
    Dicomdir,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Self::Local => tr("LocalImport.local_dev"),
            Self::Dicomdir => tr("DicomDirImport.dicomdir"),
        }
    }
}

/// The dialog, shown while `open` is true.
// TODO(2.6): the viewer opening option of the pages.
#[component]
pub fn ImportDialog(open: RwSignal<bool>, importer: Importer) -> impl IntoView {
    let files = RwSignal::new_local(Vec::<File>::new());
    let folder = RwSignal::new_local(Vec::<File>::new());
    let dicomdir_folder = RwSignal::new_local(Vec::<File>::new());
    let page = RwSignal::new(storage::load(LAST_PAGE).unwrap_or(Page::Local));
    Effect::new(move |_| storage::save(LAST_PAGE, &page.get()));
    let recursive = RwSignal::new(storage::load(LAST_RECURSIVE_MODE).unwrap_or(true));
    Effect::new(move |_| storage::save(LAST_RECURSIVE_MODE, &recursive.get()));
    let import = move || match page.get_untracked() {
        Page::Local => {
            let mut chosen = files.get_untracked();
            chosen.extend(folder_files(
                &folder.get_untracked(),
                recursive.get_untracked(),
            ));
            importer.import(chosen);
            files.set(Vec::new());
            folder.set(Vec::new());
        }
        Page::Dicomdir => {
            let chosen = dicomdir_folder.get_untracked();
            let paths = chosen.iter().map(relative_path).collect();
            importer.import_dicomdir(chosen, paths);
            dicomdir_folder.set(Vec::new());
        }
    };
    let pages = move || {
        [Page::Local, Page::Dicomdir].map(|p| {
            view! {
                <li
                    role="option"
                    aria-selected=move || (page.get() == p).to_string()
                    on:click=move |_| page.set(p)
                >
                    {p.title()}
                </li>
            }
        })
    };
    view! {
        <Dialog title=tr("DicomImport.imp_dicom") open=open>
            <div class="w-item-dialog">
                <ul class="w-item-pages" role="listbox" aria-label=tr("DicomImport.imp_dicom")>
                    {pages}
                </ul>
                <div class="w-item-page" hidden=move || page.get() != Page::Local>
                    <fieldset>
                        <legend>{tr("LocalImport.files")}</legend>
                        <FileField label=tr("LocalImport.files") files=files directory=false />
                    </fieldset>
                    <fieldset>
                        <legend>{tr("LocalImport.folders")}</legend>
                        <FileField label=tr("LocalImport.folders") files=folder directory=true />
                        <label class="w-check">
                            <input
                                type="checkbox"
                                prop:checked=recursive
                                on:change=move |e| recursive.set(event_target_checked(&e))
                            />
                            {tr("LocalImport.recursive")}
                        </label>
                    </fieldset>
                </div>
                <div class="w-item-page" hidden=move || page.get() != Page::Dicomdir>
                    <fieldset>
                        <legend>{tr("DicomDirImport.dicomdir")}</legend>
                        <FileField label=tr("DicomDirImport.path") files=dicomdir_folder directory=true />
                    </fieldset>
                </div>
            </div>
            <div class="w-dialog-buttons">
                <a class="w-help-link" href=online_help_url("dicom-import") target="_blank">"?"</a>
                <Button label=tr("DicomImport.imp") on_click=Callback::new(move |()| import()) />
                <Button
                    label=tr("DicomImport.impAndClose0")
                    kind=ButtonKind::Primary
                    on_click=Callback::new(move |()| {
                        import();
                        open.set(false);
                    })
                />
                <Button label=tr("WeasisAboutBox.close") on_click=Callback::new(move |()| open.set(false)) />
            </div>
        </Dialog>
    }
}

/// A read-only field naming the chosen files, with a button opening the file chooser.
#[component]
fn FileField(
    label: &'static str,
    files: RwSignal<Vec<File>, LocalStorage>,
    directory: bool,
) -> impl IntoView {
    let input = NodeRef::<leptos::html::Input>::new();
    let text = move || files.with(|f| field_text(f, directory));
    view! {
        <div class="w-file-field">
            <input type="text" readonly aria-label=label prop:value=text />
            <button type="button" class="w-browse" title=label on:click=move |_| {
                if let Some(input) = input.get() {
                    input.click();
                }
            }>"…"</button>
            <input
                node_ref=input
                type="file"
                multiple=!directory
                prop:webkitdirectory=directory
                hidden
                on:change=move |event| {
                    let input: HtmlInputElement = event_target(&event);
                    files.set(chosen_files(&input));
                    input.set_value("");
                }
            />
        </div>
    }
}

/// The file name, the folder name, or "Multi-selection".
fn field_text(files: &[File], directory: bool) -> String {
    match files {
        [] => String::new(),
        [file] if !directory => file.name(),
        [file, ..] if directory => {
            let path = relative_path(file);
            path.split('/').next().unwrap_or_default().to_owned()
        }
        _ => tr("LocalImport.multi_dir").to_owned(),
    }
}

/// Files of a chosen folder: all of them, or only those directly in it.
fn folder_files(files: &[File], recursive: bool) -> Vec<File> {
    files
        .iter()
        .filter(|f| recursive || is_directly_in_folder(&relative_path(f)))
        .cloned()
        .collect()
}

/// Path of a file of a chosen folder, from that folder ("folder/sub/file").
fn relative_path(file: &File) -> String {
    // `webkitRelativePath` is not exposed by web-sys.
    js_sys::Reflect::get(file, &"webkitRelativePath".into())
        .ok()
        .and_then(|path| path.as_string())
        .unwrap_or_default()
}

/// "folder/file" rather than "folder/sub/file".
fn is_directly_in_folder(relative_path: &str) -> bool {
    relative_path.matches('/').count() <= 1
}

/// Files chosen in a file input.
pub fn chosen_files(input: &HtmlInputElement) -> Vec<File> {
    let Some(list) = input.files() else {
        return Vec::new();
    };
    (0..list.length()).filter_map(|i| list.get(i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_files_directly_in_the_folder_without_recursion() {
        assert!(is_directly_in_folder("study/IM1"));
        assert!(!is_directly_in_folder("study/series/IM1"));
    }
}
