//! A pool of proc-macro server processes
use std::{io, panic::RefUnwindSafe, sync::Arc};

use parking_lot::Mutex;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{MacroDylib, ProcMacro, ServerError, process::ProcMacroServerProcess};

pub(crate) type ProcessFactory = Box<dyn Fn() -> io::Result<ProcMacroServerProcess> + Send + Sync>;

/// A fixed-size pool of proc-macro server processes.
pub(crate) struct ProcMacroServerPool {
    workers: Mutex<Box<[Arc<ProcMacroServerProcess>]>>,
    spawn: ProcessFactory,
    version: u32,
}

impl RefUnwindSafe for ProcMacroServerPool {}

impl std::fmt::Debug for ProcMacroServerPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcMacroServerPool")
            .field("workers", &self.workers)
            .field("version", &self.version)
            .finish()
    }
}

impl ProcMacroServerPool {
    pub(crate) fn new(workers: Vec<ProcMacroServerProcess>, spawn: ProcessFactory) -> Self {
        let version = workers[0].version();
        let workers = workers.into_iter().map(Arc::new).collect::<Vec<_>>().into_boxed_slice();
        Self { workers: Mutex::new(workers), spawn, version }
    }

    pub(crate) fn exited(&self) -> Option<ServerError> {
        let workers = self.workers.lock();
        if workers.iter().any(|worker| worker.exited().is_none()) {
            return None;
        }
        workers.first()?.exited().cloned()
    }

    pub(crate) fn pick_process(&self) -> Result<Arc<ProcMacroServerProcess>, ServerError> {
        let mut best: Option<Arc<ProcMacroServerProcess>> = None;
        let mut best_load = u32::MAX;

        for worker in self.workers.lock().iter() {
            if worker.exited().is_some() {
                continue;
            }

            let load = worker.number_of_active_req();
            if load == 0 {
                return Ok(worker.clone());
            }

            if load < best_load {
                best = Some(worker.clone());
                best_load = load;
            }
        }

        best.ok_or_else(|| ServerError {
            message: "all proc-macro server workers have exited".into(),
            io: None,
        })
    }

    pub(crate) fn replace_timed_out_process_in_background(
        self: &Arc<Self>,
        process: Arc<ProcMacroServerProcess>,
    ) {
        if !process.claim_replacement() {
            return;
        }

        let thread = std::thread::Builder::new().name("proc-macro-restarter".into()).spawn({
            let pool = self.clone();
            let process = process.clone();
            move || {
                if let Err(error) = pool.replace_timed_out_process(&process) {
                    process.replacement_failed();
                    tracing::error!(%error, "failed to replace timed out proc-macro server");
                }
            }
        });
        if let Err(error) = thread {
            process.replacement_failed();
            tracing::error!(%error, "failed to spawn proc-macro server restarter");
        }
    }

    fn replace_timed_out_process(
        &self,
        process: &Arc<ProcMacroServerProcess>,
    ) -> Result<(), ServerError> {
        let slot = self.workers.lock().iter().position(|worker| Arc::ptr_eq(worker, process));
        let Some(slot) = slot else {
            return Ok(());
        };

        let replacement = (self.spawn)().map_err(|error| ServerError {
            message: "failed to restart proc-macro server after expansion timeout".into(),
            io: Some(Arc::new(error)),
        })?;
        if replacement.version() != self.version {
            return Err(ServerError {
                message: format!(
                    "restarted proc-macro server changed protocol version from {} to {}",
                    self.version,
                    replacement.version()
                ),
                io: None,
            });
        }

        let mut workers = self.workers.lock();
        if Arc::ptr_eq(&workers[slot], process) {
            workers[slot] = Arc::new(replacement);
        }
        Ok(())
    }

    pub(crate) fn load_dylib(
        self: &Arc<Self>,
        dylib: &MacroDylib,
    ) -> Result<Vec<ProcMacro>, ServerError> {
        let _span = tracing::info_span!("ProcMacroServer::load_dylib").entered();

        let dylib_path = Arc::new(dylib.path.clone());
        let dylib_last_modified =
            std::fs::metadata(dylib_path.as_path()).ok().and_then(|m| m.modified().ok());
        let workers = self.workers.lock().iter().cloned().collect::<Vec<_>>();
        let (first, rest) = workers.split_first().expect("worker pool must not be empty");

        let macros = first
            .find_proc_macros(&dylib.path)?
            .map_err(|e| ServerError { message: e, io: None })?;

        rest.into_par_iter()
            .map(|worker| {
                worker
                    .find_proc_macros(&dylib.path)?
                    .map(|_| ())
                    .map_err(|e| ServerError { message: e, io: None })
            })
            .collect::<Result<(), _>>()?;

        Ok(macros
            .into_iter()
            .map(|(name, kind)| ProcMacro {
                pool: self.clone(),
                name: name.into(),
                kind,
                dylib_path: dylib_path.clone(),
                dylib_last_modified,
            })
            .collect())
    }

    pub(crate) fn version(&self) -> u32 {
        self.version
    }
}
