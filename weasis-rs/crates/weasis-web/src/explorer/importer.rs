//! Imports local files into the model: the files go to the import worker one request at a time,
//! each request showing as a loading task until the worker has read all its files.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/LoadLocalDicom.java`
//! and `LoadDicom.java` (counting errors and unsupported files, `done`).

use std::collections::VecDeque;

use js_sys::Array;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use weasis_core::explorer::{ImportEvent, ImportedFile};
use weasis_core::model::{DicomModel, SplittingRules};
use web_sys::{ErrorEvent, File, MessageEvent, Worker};

use super::import_report::ReportMessage;

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

/// Files of a request and what was found so far.
struct Job {
    task: u32,
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
    // The running job is the first one; only it was sent to the worker.
    jobs: StoredValue<VecDeque<Job>, LocalStorage>,
    worker: StoredValue<Option<Worker>, LocalStorage>,
    // The worker said it is ready: requests sent before would be lost.
    worker_ready: StoredValue<bool>,
    next_task: StoredValue<u32>,
}

impl Importer {
    /// No data loaded.
    pub fn new() -> Self {
        Self {
            model: RwSignal::new(DicomModel::new()),
            tasks: RwSignal::new(Vec::new()),
            report: RwSignal::new(VecDeque::new()),
            jobs: StoredValue::new_local(VecDeque::new()),
            worker: StoredValue::new_local(None),
            worker_ready: StoredValue::new(false),
            next_task: StoredValue::new(1),
        }
    }

    /// Imports files; nothing happens for an empty list.
    // TODO(3.1): keep the File of each instance to decode its pixels.
    pub fn import(&self, files: Vec<File>) {
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
        let job = Job {
            task,
            files,
            read: 0,
            errors: 0,
            unsupported: 0,
        };
        let idle = self.jobs.with_value(VecDeque::is_empty);
        self.jobs.update_value(|jobs| jobs.push_back(job));
        if idle {
            self.send_next();
        }
    }

    /// Stops a request: a waiting one is dropped, a running one ends with its worker.
    pub fn cancel(&self, task: u32) {
        self.tasks.update(|t| {
            if let Some(t) = t.iter_mut().find(|t| t.id == task) {
                t.aborting = true;
            }
        });
        let running = self.jobs.with_value(|jobs| jobs.front().map(|j| j.task)) == Some(task);
        self.jobs
            .update_value(|jobs| jobs.retain(|j| j.task != task));
        self.tasks.update(|t| t.retain(|t| t.id != task));
        if running {
            if let Some(worker) = self.worker.try_update_value(Option::take).flatten() {
                worker.terminate();
            }
            self.send_next();
        }
    }

    fn send_next(&self) {
        let files = self
            .jobs
            .with_value(|jobs| jobs.front().map(|job| job.files.iter().collect::<Array>()));
        let Some(files) = files else {
            return;
        };
        let Some(worker) = self.worker() else {
            return;
        };
        if !self.worker_ready.get_value() {
            // Sent once the worker is ready.
            return;
        }
        if let Err(error) = worker.post_message(&files) {
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
            let event = event
                .data()
                .as_string()
                .and_then(|json| serde_json::from_str(&json).ok());
            match event {
                Some(ImportEvent::Ready) => {
                    importer.worker_ready.set_value(true);
                    importer.send_next();
                }
                Some(ImportEvent::Read(file)) => importer.add(file),
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
        self.jobs.update_value(|jobs| {
            if let Some(job) = jobs.front_mut() {
                job.read += 1;
                job.errors += file.errors.len();
                job.unsupported += file.unsupported;
            }
        });
        if !file.instances.is_empty() {
            let rules = SplittingRules::built_in();
            self.model.update(|model| {
                for instance in file.instances {
                    model.add_split_instance(instance, rules);
                }
            });
        }
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

    fn finish(&self) {
        let Some(job) = self.jobs.try_update_value(VecDeque::pop_front).flatten() else {
            return;
        };
        self.tasks.update(|t| t.retain(|t| t.id != job.task));
        self.report.update(|report| {
            if job.errors > 0 {
                report.push_back(ReportMessage::Errors(job.errors));
            }
            if job.unsupported > 0 {
                report.push_back(ReportMessage::Unsupported(job.unsupported));
            }
        });
        self.send_next();
    }
}

impl Default for Importer {
    fn default() -> Self {
        Self::new()
    }
}
