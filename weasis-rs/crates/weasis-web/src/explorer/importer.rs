//! Imports local files into the model: the files go to the import worker one request at a time,
//! each request showing as a loading task until the worker has read all its files. Once an
//! import ends, the worker renders the thumbnails of the new series, one request each.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/LoadLocalDicom.java`,
//! `LoadDicom.java` (counting errors and unsupported files, `done`), `imp/DicomDirImport.java`
//! and `imp/DicomZipMediaIO.java`.

use std::collections::{HashSet, VecDeque};

use js_sys::{Array, Uint8Array};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use weasis_core::explorer::{ImportEvent, ImportRequest, ImportedFile, THUMBNAIL_MAX_SIZE};
use weasis_core::model::{DicomModel, SplittingRules};
use web_sys::{ErrorEvent, File, MessageEvent, Worker};

use super::import_report::ReportMessage;
use super::thumbnails::{Source, ThumbnailRequest, Thumbnails};

/// Loader of the import worker, built by Trunk (`index.html`).
const WORKER_LOADER: &str = "./import_worker_loader.js";

/// A request shown in the explorer while it runs.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadingTask {
    /// Identifier.
    pub id: u32,
    /// Stop asked, the task ends.
    pub aborting: bool,
}

/// What a request is for.
enum JobKind {
    /// An import, shown as the loading task of this id.
    Import(u32),
    /// The thumbnail of this series.
    Thumbnail(String),
}

/// Files of a request and what was found so far.
pub(super) struct Job {
    kind: JobKind,
    request: ImportRequest,
    files: Vec<File>,
    /// Files the worker has read.
    read: usize,
    errors: usize,
    unsupported: u32,
}

/// The import of local files, shared through the Leptos context.
#[derive(Clone, Copy)]
pub struct Importer {
    /// The loaded patients, studies and series.
    pub model: RwSignal<DicomModel>,
    /// Requests waiting or running, in order.
    pub tasks: RwSignal<Vec<LoadingTask>>,
    /// Messages to show once a request ends.
    pub report: RwSignal<VecDeque<ReportMessage>>,
    /// Encrypted archives waiting for their password.
    pub password_prompts: RwSignal<VecDeque<File>, LocalStorage>,
    /// Files of a folder whose DICOMDIR gave nothing, offered for a plain import.
    pub manual_import: RwSignal<Option<Vec<File>>, LocalStorage>,
    /// Thumbnails of the series.
    pub thumbnails: Thumbnails,
    // The running job is the first one; only it was sent to the worker.
    pub(super) jobs: StoredValue<VecDeque<Job>, LocalStorage>,
    worker: StoredValue<Option<Worker>, LocalStorage>,
    // The worker said it is ready: requests sent before would be lost.
    worker_ready: StoredValue<bool>,
    next_task: StoredValue<u32>,
    // Series whose phases were looked at once loaded (`LoadLocalDicom.seriesPostProcessing`).
    pub(super) post_processed: StoredValue<HashSet<String>>,
}

impl Importer {
    /// No data loaded.
    pub fn new() -> Self {
        Self {
            model: RwSignal::new(DicomModel::new()),
            tasks: RwSignal::new(Vec::new()),
            report: RwSignal::new(VecDeque::new()),
            password_prompts: RwSignal::new_local(VecDeque::new()),
            manual_import: RwSignal::new_local(None),
            thumbnails: Thumbnails::new(),
            jobs: StoredValue::new_local(VecDeque::new()),
            worker: StoredValue::new_local(None),
            worker_ready: StoredValue::new(false),
            post_processed: StoredValue::new(HashSet::new()),
            next_task: StoredValue::new(1),
        }
    }

    /// Imports files; nothing happens for an empty list.
    pub fn import(&self, files: Vec<File>) {
        self.request(ImportRequest::Files { password: None }, files);
    }

    /// Imports encrypted archives with their password.
    pub fn import_with_password(&self, files: Vec<File>, password: String) {
        let password = Some(password);
        self.request(ImportRequest::Files { password }, files);
    }

    /// Imports the files of a folder through its DICOMDIR; `paths` are the paths of the files
    /// in the folder.
    pub fn import_dicomdir(&self, files: Vec<File>, paths: Vec<String>) {
        self.request(ImportRequest::Dicomdir { paths }, files);
    }

    fn request(&self, request: ImportRequest, files: Vec<File>) {
        if files.is_empty() {
            return;
        }
        let task = self.next_task.get_value();
        self.next_task.set_value(task + 1);
        self.tasks.update(|t| {
            t.push(LoadingTask {
                id: task,
                aborting: false,
            })
        });
        let idle = self.jobs.with_value(VecDeque::is_empty);
        self.enqueue(JobKind::Import(task), request, files);
        if idle {
            self.send_next();
        }
    }

    pub(super) fn request_thumbnail(&self, thumbnail: ThumbnailRequest) {
        let request = ImportRequest::Thumbnail {
            entry: thumbnail.source.entry,
            password: thumbnail.source.password,
            frame: thumbnail.frame,
            size: THUMBNAIL_MAX_SIZE,
        };
        let files = vec![thumbnail.source.file];
        self.enqueue(JobKind::Thumbnail(thumbnail.series), request, files);
    }

    /// Queues a request; the caller sends it when the worker is idle (only the first job of the
    /// queue is ever sent, once).
    fn enqueue(&self, kind: JobKind, request: ImportRequest, files: Vec<File>) {
        let job = Job {
            kind,
            request,
            files,
            read: 0,
            errors: 0,
            unsupported: 0,
        };
        self.jobs.update_value(|jobs| jobs.push_back(job));
    }

    /// Stops a request: a waiting one is dropped, a running one ends with its worker.
    pub fn cancel(&self, task: u32) {
        self.tasks.update(|t| {
            if let Some(t) = t.iter_mut().find(|t| t.id == task) {
                t.aborting = true;
            }
        });
        let is_task = |job: &Job| matches!(job.kind, JobKind::Import(id) if id == task);
        let running = self
            .jobs
            .with_value(|jobs| jobs.front().is_some_and(is_task));
        self.jobs.update_value(|jobs| jobs.retain(|j| !is_task(j)));
        self.tasks.update(|t| t.retain(|t| t.id != task));
        if running {
            if let Some(worker) = self.worker.try_update_value(Option::take).flatten() {
                worker.terminate();
            }
            self.send_next();
        }
    }

    pub(super) fn send_next(&self) {
        let message = self.jobs.with_value(|jobs| {
            let job = jobs.front()?;
            let request = serde_json::to_string(&job.request).ok()?;
            let message = Array::of1(&request.into());
            job.files.iter().for_each(|file| _ = message.push(file));
            Some(message)
        });
        let Some(message) = message else {
            return;
        };
        let Some(worker) = self.worker() else {
            return;
        };
        if !self.worker_ready.get_value() {
            // Sent once the worker is ready.
            return;
        }
        if let Err(error) = worker.post_message(&message) {
            leptos::logging::error!("cannot send the files to the import worker: {error:?}");
        }
    }

    /// The worker, started on first use.
    fn worker(&self) -> Option<Worker> {
        if let Some(worker) = self.worker.get_value() {
            return Some(worker);
        }
        let worker = match Worker::new(WORKER_LOADER) {
            Ok(worker) => worker,
            Err(error) => {
                leptos::logging::error!("cannot start the import worker: {error:?}");
                return None;
            }
        };
        let importer = *self;
        let on_message = Closure::<dyn Fn(MessageEvent)>::new(move |event: MessageEvent| {
            // A thumbnail comes as [event, pixels], the other events alone.
            let data = event.data();
            let (header, pixels) = match data.dyn_ref::<Array>() {
                Some(parts) => (parts.get(0), Some(Uint8Array::new(&parts.get(1)).to_vec())),
                None => (data, None),
            };
            let event = header
                .as_string()
                .and_then(|json| serde_json::from_str(&json).ok());
            match event {
                Some(ImportEvent::Thumbnail { width, height }) => {
                    importer.store_thumbnail(width, height, &pixels.unwrap_or_default());
                }
                Some(ImportEvent::Ready) => {
                    importer.worker_ready.set_value(true);
                    importer.send_next();
                }
                Some(ImportEvent::Read(file)) => importer.add(file),
                Some(ImportEvent::DicomdirEmpty) => importer.offer_manual_import(),
                Some(ImportEvent::Finished) => importer.finish(),
                None => leptos::logging::error!("unexpected message of the import worker"),
            }
        });
        let on_error = Closure::<dyn Fn(ErrorEvent)>::new(move |event: ErrorEvent| {
            leptos::logging::error!("the import worker stopped: {}", event.message());
            importer.worker_failed();
        });
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        // The handlers live as long as the worker, which is never dropped while running.
        on_message.forget();
        on_error.forget();
        self.worker_ready.set_value(false);
        self.worker.set_value(Some(worker.clone()));
        Some(worker)
    }

    fn add(&self, file: ImportedFile) {
        for error in &file.errors {
            leptos::logging::warn!("{error}");
        }
        if file.password_required {
            let archive = self
                .jobs
                .with_value(|jobs| jobs.front()?.files.get(file.file).cloned());
            self.password_prompts.update(|p| p.extend(archive));
        }
        let source = self.jobs.try_update_value(|jobs| {
            let job = jobs.front_mut()?;
            job.read += 1;
            job.errors += file.errors.len();
            job.unsupported += file.unsupported;
            let password = match &job.request {
                ImportRequest::Files { password } => password.clone(),
                _ => None,
            };
            Some((job.files.get(file.file)?.clone(), password))
        });
        let Some((source_file, password)) = source.flatten() else {
            return;
        };
        if !file.instances.is_empty() {
            let rules = SplittingRules::built_in();
            self.model.update(|model| {
                for imported in file.instances {
                    let source = Source {
                        file: source_file.clone(),
                        entry: imported.entry,
                        password: password.clone(),
                    };
                    self.thumbnails
                        .record(imported.instance.sop_instance_uid.clone(), source);
                    model.add_split_instance(imported.instance, rules);
                }
            });
        }
    }

    fn offer_manual_import(&self) {
        let files = self
            .jobs
            .with_value(|jobs| jobs.front().map(|job| job.files.clone()));
        self.manual_import.set(files);
    }

    /// The worker stopped on an error: the files of the running request it had not read count
    /// as errors, and a new worker takes the next requests.
    fn worker_failed(&self) {
        if let Some(worker) = self.worker.try_update_value(Option::take).flatten() {
            worker.terminate();
        }
        self.jobs.update_value(|jobs| {
            if let Some(job) = jobs.front_mut() {
                job.errors += job.files.len() - job.read;
            }
        });
        self.finish();
    }

    fn store_thumbnail(&self, width: u32, height: u32, pixels: &[u8]) {
        let series = self.jobs.with_value(|jobs| match &jobs.front()?.kind {
            JobKind::Thumbnail(series) => Some(series.clone()),
            JobKind::Import(_) => None,
        });
        if let Some(series) = series {
            self.thumbnails.store(series, width, height, pixels);
        }
    }

    fn finish(&self) {
        let Some(job) = self.jobs.try_update_value(VecDeque::pop_front).flatten() else {
            return;
        };
        let JobKind::Import(task) = job.kind else {
            self.send_next();
            return;
        };
        self.tasks.update(|t| t.retain(|t| t.id != task));
        self.report.update(|report| {
            if job.errors > 0 {
                report.push_back(ReportMessage::Errors(job.errors));
            }
            if job.unsupported > 0 {
                report.push_back(ReportMessage::Unsupported(job.unsupported));
            }
        });
        self.separate_new_phases();
        self.queue_missing_thumbnails();
        self.send_next();
    }
}

impl Default for Importer {
    fn default() -> Self {
        Self::new()
    }
}
