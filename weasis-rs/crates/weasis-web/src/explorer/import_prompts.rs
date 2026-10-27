//! Questions asked during an import: the password of an encrypted ZIP archive, and whether to
//! import a folder file by file when its DICOMDIR gave nothing.
//!
//! NOTE: the desktop application then shows the local device page with the folder filled in;
//! here the folder is imported at once, which is what that page would do.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/imp/DicomZipMediaIO.java`
//! (`loadDicomZip`) and `DicomDirImport.java` (`importDICOM`).

use leptos::prelude::*;

use super::Importer;
use crate::i18n::tr;
use crate::widgets::{Button, ButtonKind, Dialog};

/// Asks the password of each encrypted archive in turn; Cancel skips the archive.
#[component]
pub fn ZipPasswordDialog(importer: Importer) -> impl IntoView {
    let prompts = importer.password_prompts;
    let open = RwSignal::new(false);
    Effect::new(move |_| open.set(prompts.with(|p| !p.is_empty())));
    let password = RwSignal::new(String::new());
    let answer = move |ok: bool| {
        let archive = prompts.try_update(|p| p.pop_front()).flatten();
        if let (true, Some(archive)) = (ok, archive) {
            importer.import_with_password(vec![archive], password.get_untracked());
        }
        password.set(String::new());
    };
    // Closing by the title bar skips the archive too.
    Effect::new(move |previous: Option<bool>| {
        let is_open = open.get();
        if previous == Some(true) && !is_open && prompts.with_untracked(|p| !p.is_empty()) {
            answer(false);
        }
        is_open
    });
    view! {
        <Dialog title=tr("DicomZipImport.title") open=open>
            <label class="w-password">
                {tr("enter.password")}
                <input
                    type="password"
                    autofocus
                    prop:value=password
                    on:input=move |e| password.set(event_target_value(&e))
                />
            </label>
            <div class="w-dialog-buttons">
                <Button label="OK" kind=ButtonKind::Primary on_click=Callback::new(move |()| answer(true)) />
                <Button label="Cancel" on_click=Callback::new(move |()| answer(false)) />
            </div>
        </Dialog>
    }
}

/// Offers to import the files of a folder whose DICOMDIR gave nothing.
#[component]
pub fn ManualImportDialog(importer: Importer) -> impl IntoView {
    let files = importer.manual_import;
    let open = RwSignal::new(false);
    Effect::new(move |_| open.set(files.with(Option::is_some)));
    let answer = move |yes: bool| {
        if let (true, Some(files)) = (yes, files.try_update(Option::take).flatten()) {
            importer.import(files);
        }
        files.set(None);
    };
    // Closing by the title bar is a No.
    Effect::new(move |previous: Option<bool>| {
        let is_open = open.get();
        if previous == Some(true) && !is_open {
            files.set(None);
        }
        is_open
    });
    view! {
        <Dialog title=tr("DicomDirImport.dicomdir") open=open>
            <p class="w-report-message">{tr("DicomExplorer.mes_import_manual")}</p>
            <div class="w-dialog-buttons">
                <Button label="Yes" kind=ButtonKind::Primary on_click=Callback::new(move |()| answer(true)) />
                <Button label="No" on_click=Callback::new(move |()| answer(false)) />
            </div>
        </Dialog>
    }
}
