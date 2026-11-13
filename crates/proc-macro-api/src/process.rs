//! Handle process life-time and message passing for proc-macro client

use std::{
    fmt::Debug,
    io::{self, BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU8, AtomicU32, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use paths::AbsPath;
use semver::Version;
use span::Span;
use stdx::JodChild;

use crate::{
    ProcMacro, ProcMacroKind, ProtocolFormat, ServerError,
    bidirectional_protocol::{
        self, SubCallback,
        msg::{BidirectionalMessage, SubResponse},
        reject_subrequests,
    },
    legacy_protocol::{self, SpanMode},
    version,
};

/// Represents a process handling proc-macro communication.
pub(crate) struct ProcMacroServerProcess {
    /// The state of the proc-macro server process, the protocol is currently strictly sequential
    /// hence the lock on the state.
    state: Mutex<ProcessSrvState>,
    /// The process handle and its health, shared with the watchdog thread so that it can kill
    /// the process when an expansion times out.
    control: Arc<ProcessControl>,
    /// When set, each expansion is raced against the timeout by the watchdog, which kills the
    /// process if it does not respond in time.
    watchdog: Option<(ProcessWatchdog, Duration)>,
    version: u32,
    protocol: Protocol,
    active: AtomicU32,
}

impl std::fmt::Debug for ProcMacroServerProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcMacroServerProcess")
            .field("version", &self.version)
            .field("protocol", &self.protocol)
            .field("exited", &self.control.exited)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Protocol {
    LegacyJson { mode: SpanMode },
    BidirectionalPostcardPrototype { mode: SpanMode },
}

pub trait ProcessExit: Send + Sync {
    fn exit_err(&mut self) -> Option<ServerError>;
    fn kill(&mut self) -> io::Result<()>;
}

/// Health of a server process with respect to expansion timeouts.
///
/// The watchdog moves a process from `Healthy` to `TimedOut` when an expansion misses its
/// deadline. The pool claims a timed out process (`TimedOut` -> `BeingReplaced`) before spawning
/// its replacement, releasing the claim (back to `TimedOut`) if that fails so that a later
/// expansion can retry the replacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum ProcessStatus {
    Healthy = 0,
    TimedOut = 1,
    BeingReplaced = 2,
}

/// The part of a server process that is shared with the watchdog thread.
struct ProcessControl {
    process: Mutex<Box<dyn ProcessExit>>,
    /// Populated when the server exits, whether on its own or killed by the watchdog.
    exited: OnceLock<ServerError>,
    /// A [`ProcessStatus`].
    status: AtomicU8,
}

impl ProcessControl {
    fn new(process: Box<dyn ProcessExit>) -> Arc<ProcessControl> {
        Arc::new(ProcessControl {
            process: Mutex::new(process),
            exited: OnceLock::new(),
            status: AtomicU8::new(ProcessStatus::Healthy as u8),
        })
    }

    fn time_out(&self, macro_name: &str, timeout: Duration) {
        let error = ServerError {
            message: format!("proc-macro `{macro_name}` expansion timed out after {timeout:?}"),
            io: None,
        };
        if self.exited.set(error).is_ok() {
            self.status.store(ProcessStatus::TimedOut as u8, Ordering::Release);
            _ = self.process.lock().kill();
        }
    }

    fn timed_out(&self) -> bool {
        self.status.load(Ordering::Acquire) != ProcessStatus::Healthy as u8
    }

    fn claim_replacement(&self) -> bool {
        self.status
            .compare_exchange(
                ProcessStatus::TimedOut as u8,
                ProcessStatus::BeingReplaced as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn replacement_failed(&self) {
        self.status.store(ProcessStatus::TimedOut as u8, Ordering::Release);
    }
}

/// A handle to the watchdog thread which kills server processes whose expansion requests
/// exceed their deadline. The thread exits once all handles to it have been dropped.
#[derive(Clone, Debug)]
pub(crate) struct ProcessWatchdog {
    sender: mpsc::Sender<WatchdogCommand>,
}

struct WatchdogRequest {
    deadline: Instant,
    timeout: Duration,
    macro_name: Box<str>,
    control: Arc<ProcessControl>,
}

enum WatchdogCommand {
    Arm(WatchdogRequest),
    Disarm(Arc<ProcessControl>),
}

/// Disarms the pending watchdog request on drop.
struct WatchdogGuard {
    control: Arc<ProcessControl>,
    sender: mpsc::Sender<WatchdogCommand>,
}

impl Drop for WatchdogGuard {
    fn drop(&mut self) {
        _ = self.sender.send(WatchdogCommand::Disarm(self.control.clone()));
    }
}

impl ProcessWatchdog {
    pub(crate) fn spawn() -> io::Result<ProcessWatchdog> {
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("proc-macro-watchdog".into())
            .spawn(move || run_watchdog(receiver))?;
        Ok(ProcessWatchdog { sender })
    }

    /// Arms a timeout for a single expansion; dropping the returned guard disarms it.
    ///
    /// A process has at most one request armed at a time as its I/O is strictly sequential,
    /// which makes `control` a unique key for the request.
    fn arm(
        &self,
        control: &Arc<ProcessControl>,
        macro_name: &str,
        timeout: Duration,
    ) -> WatchdogGuard {
        let guard = WatchdogGuard { control: control.clone(), sender: self.sender.clone() };
        let Some(deadline) = Instant::now().checked_add(timeout) else {
            return guard;
        };
        let request = WatchdogRequest {
            deadline,
            timeout,
            macro_name: macro_name.into(),
            control: control.clone(),
        };
        if self.sender.send(WatchdogCommand::Arm(request)).is_err() {
            stdx::never!("the proc-macro watchdog thread has died");
        }
        guard
    }
}

fn run_watchdog(receiver: mpsc::Receiver<WatchdogCommand>) {
    let mut requests = Vec::<WatchdogRequest>::new();
    loop {
        let deadline = requests.iter().map(|request| request.deadline).min();
        let command = match deadline {
            Some(deadline) => {
                match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(command) => Some(command),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            None => match receiver.recv() {
                Ok(command) => Some(command),
                Err(mpsc::RecvError) => return,
            },
        };

        match command {
            Some(WatchdogCommand::Arm(request)) => requests.push(request),
            Some(WatchdogCommand::Disarm(control)) => {
                requests.retain(|request| !Arc::ptr_eq(&request.control, &control));
            }
            None => {
                let now = Instant::now();
                for request in requests.extract_if(.., |request| request.deadline <= now) {
                    request.control.time_out(&request.macro_name, request.timeout);
                }
            }
        }
    }
}

impl ProcessExit for Process {
    fn exit_err(&mut self) -> Option<ServerError> {
        match self.child.try_wait() {
            Ok(None) | Err(_) => None,
            Ok(Some(status)) => {
                let mut msg = String::new();
                if !status.success()
                    && let Some(stderr) = self.child.stderr.as_mut()
                {
                    _ = stderr.read_to_string(&mut msg);
                }
                Some(ServerError {
                    message: format!(
                        "proc-macro server exited with {status}{}{msg}",
                        if msg.is_empty() { "" } else { ": " }
                    ),
                    io: None,
                })
            }
        }
    }

    fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }
}

/// Maintains the state of the proc-macro server process.
pub(crate) struct ProcessSrvState {
    stdin: Box<dyn Write + Send + Sync>,
    stdout: Box<dyn BufRead + Send + Sync>,
}

impl ProcMacroServerProcess {
    /// Starts the proc-macro server and performs a version check
    pub(crate) fn spawn<'a>(
        process_path: &AbsPath,
        env: impl IntoIterator<
            Item = (impl AsRef<std::ffi::OsStr>, &'a Option<impl 'a + AsRef<std::ffi::OsStr>>),
        > + Clone,
        version: Option<&Version>,
        watchdog: Option<(ProcessWatchdog, Duration)>,
    ) -> io::Result<ProcMacroServerProcess> {
        Self::run(
            |format| {
                let mut process = Process::run(
                    process_path,
                    env.clone(),
                    format.map(|format| format.to_string()).as_deref(),
                )?;
                let (stdin, stdout) = process.stdio().expect("couldn't access child stdio");

                Ok((Box::new(process), Box::new(stdin), Box::new(stdout)))
            },
            version,
            || {
                #[expect(clippy::disallowed_methods)]
                Command::new(process_path)
                    .arg("--version")
                    .output()
                    .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                    .unwrap_or_else(|_| "unknown version".to_owned())
            },
            watchdog,
        )
    }

    /// Invokes `spawn` and performs a version check.
    pub(crate) fn run(
        spawn: impl Fn(
            Option<ProtocolFormat>,
        ) -> io::Result<(
            Box<dyn ProcessExit>,
            Box<dyn Write + Send + Sync>,
            Box<dyn BufRead + Send + Sync>,
        )>,
        version: Option<&Version>,
        binary_server_version: impl Fn() -> String,
        watchdog: Option<(ProcessWatchdog, Duration)>,
    ) -> io::Result<ProcMacroServerProcess> {
        const VERSION: Version = Version::new(1, 93, 0);
        // we do `>` for nightly as this started working in the middle of the 1.93 nightly release, so we dont want to break on half of the nightlies
        let has_working_format_flag = version.map_or(false, |v| {
            if v.pre.as_str() == "nightly" { *v > VERSION } else { *v >= VERSION }
        });

        let formats: &[_] = if std::env::var_os("RUST_ANALYZER_USE_POSTCARD").is_some()
            && has_working_format_flag
        {
            &[
                Some(ProtocolFormat::BidirectionalPostcardPrototype),
                Some(ProtocolFormat::JsonLegacy),
            ]
        } else {
            &[None]
        };

        let mut err = None;
        for &format in formats {
            let create_srv = || {
                let (process, stdin, stdout) = spawn(format)?;

                io::Result::Ok(ProcMacroServerProcess {
                    state: Mutex::new(ProcessSrvState { stdin, stdout }),
                    control: ProcessControl::new(process),
                    watchdog: watchdog.clone(),
                    version: 0,
                    protocol: match format {
                        Some(ProtocolFormat::BidirectionalPostcardPrototype) => {
                            Protocol::BidirectionalPostcardPrototype { mode: SpanMode::Id }
                        }
                        Some(ProtocolFormat::JsonLegacy) | None => {
                            Protocol::LegacyJson { mode: SpanMode::Id }
                        }
                    },
                    active: AtomicU32::new(0),
                })
            };
            let mut srv = create_srv()?;
            tracing::info!("sending proc-macro server version check");
            match srv.version_check(Some(&reject_subrequests)) {
                Ok(v) if v > version::CURRENT_API_VERSION => {
                    let process_version = binary_server_version();
                    err = Some(io::Error::other(format!(
                        "Your installed proc-macro server is too new for your rust-analyzer. API version: {}, server version: {process_version}. \
                        This will prevent proc-macro expansion from working. Please consider updating your rust-analyzer to ensure compatibility with your current toolchain.",
                        version::CURRENT_API_VERSION
                    )));
                }
                Ok(v) => {
                    tracing::info!("Proc-macro server version: {v}");
                    srv.version = v;
                    if srv.version >= version::RUST_ANALYZER_SPAN_SUPPORT
                        && let Ok(new_mode) =
                            srv.enable_rust_analyzer_spans(Some(&reject_subrequests))
                    {
                        match &mut srv.protocol {
                            Protocol::LegacyJson { mode }
                            | Protocol::BidirectionalPostcardPrototype { mode } => *mode = new_mode,
                        }
                    }
                    tracing::info!("Proc-macro server protocol: {:?}", srv.protocol);
                    return Ok(srv);
                }
                Err(e) => {
                    tracing::info!(%e, "proc-macro version check failed");
                    err = Some(io::Error::other(format!(
                        "proc-macro server version check failed: {e}"
                    )))
                }
            }
        }
        Err(err.unwrap())
    }

    /// Finds proc-macros in a given dynamic library.
    pub(crate) fn find_proc_macros(
        &self,
        dylib_path: &AbsPath,
    ) -> Result<Result<Vec<(String, ProcMacroKind)>, String>, ServerError> {
        match self.protocol {
            Protocol::LegacyJson { .. } => legacy_protocol::find_proc_macros(self, dylib_path),

            Protocol::BidirectionalPostcardPrototype { .. } => {
                bidirectional_protocol::find_proc_macros(self, dylib_path, &|_| {
                    Ok(SubResponse::Cancel {
                        reason: String::from(
                            "Server should not do a sub request when loading proc-macros",
                        ),
                    })
                })
            }
        }
    }

    /// Returns the server error if the process has exited.
    pub(crate) fn exited(&self) -> Option<&ServerError> {
        self.control.exited.get()
    }

    /// Whether the process was killed because an expansion timed out.
    pub(crate) fn timed_out(&self) -> bool {
        self.control.timed_out()
    }

    pub(crate) fn claim_replacement(&self) -> bool {
        self.control.claim_replacement()
    }

    pub(crate) fn replacement_failed(&self) {
        self.control.replacement_failed()
    }

    /// Retrieves the API version of the proc-macro server.
    pub(crate) fn version(&self) -> u32 {
        self.version
    }

    /// Enable support for rust-analyzer span mode if the server supports it.
    pub(crate) fn rust_analyzer_spans(&self) -> bool {
        match self.protocol {
            Protocol::LegacyJson { mode } | Protocol::BidirectionalPostcardPrototype { mode } => {
                mode == SpanMode::RustAnalyzer
            }
        }
    }

    /// Checks the API version of the running proc-macro server.
    fn version_check(&self, callback: Option<SubCallback<'_>>) -> Result<u32, ServerError> {
        match self.protocol {
            Protocol::LegacyJson { .. } => legacy_protocol::version_check(self),
            Protocol::BidirectionalPostcardPrototype { .. } => {
                let cb = callback.expect("callback required for bidirectional protocol");
                bidirectional_protocol::version_check(self, cb)
            }
        }
    }

    /// Enable support for rust-analyzer span mode if the server supports it.
    fn enable_rust_analyzer_spans(
        &self,
        callback: Option<SubCallback<'_>>,
    ) -> Result<SpanMode, ServerError> {
        match self.protocol {
            Protocol::LegacyJson { .. } => legacy_protocol::enable_rust_analyzer_spans(self),
            Protocol::BidirectionalPostcardPrototype { .. } => {
                let cb = callback.expect("callback required for bidirectional protocol");
                bidirectional_protocol::enable_rust_analyzer_spans(self, cb)
            }
        }
    }

    pub(crate) fn expand(
        &self,
        proc_macro: &ProcMacro,
        subtree: tt::SubtreeView<'_>,
        attr: Option<tt::SubtreeView<'_>>,
        env: Vec<(String, String)>,
        def_site: Span,
        call_site: Span,
        mixed_site: Span,
        current_dir: String,
        callback: Option<SubCallback<'_>>,
    ) -> Result<Result<tt::TopSubtree, String>, ServerError> {
        self.active.fetch_add(1, Ordering::AcqRel);
        let result = match self.protocol {
            Protocol::LegacyJson { .. } => legacy_protocol::expand(
                proc_macro,
                self,
                subtree,
                attr,
                env,
                def_site,
                call_site,
                mixed_site,
                current_dir,
            ),
            Protocol::BidirectionalPostcardPrototype { .. } => bidirectional_protocol::expand(
                proc_macro,
                self,
                subtree,
                attr,
                env,
                def_site,
                call_site,
                mixed_site,
                current_dir,
                callback.expect("callback required for bidirectional protocol"),
            ),
        };

        self.active.fetch_sub(1, Ordering::AcqRel);
        result
    }

    pub(crate) fn send_task_legacy<Request, Response>(
        &self,
        send: impl FnOnce(
            &mut dyn Write,
            &mut dyn BufRead,
            Request,
            &mut String,
        ) -> Result<Option<Response>, ServerError>,
        req: Request,
        macro_name: Option<&str>,
    ) -> Result<Response, ServerError> {
        self.with_locked_io(String::new(), macro_name, |writer, reader, buf| {
            send(writer, reader, req, buf).and_then(|res| {
                res.ok_or_else(|| {
                    let message = "proc-macro server did not respond with data".to_owned();
                    ServerError {
                        io: Some(Arc::new(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            message.clone(),
                        ))),
                        message,
                    }
                })
            })
        })
    }

    fn with_locked_io<R, B>(
        &self,
        mut buf: B,
        macro_name: Option<&str>,
        f: impl FnOnce(&mut dyn Write, &mut dyn BufRead, &mut B) -> Result<R, ServerError>,
    ) -> Result<R, ServerError> {
        let state = &mut *self.state.lock();
        let watchdog = match (&self.watchdog, macro_name) {
            (Some((watchdog, timeout)), Some(macro_name)) => {
                Some(watchdog.arm(&self.control, macro_name, *timeout))
            }
            (Some(_), None) | (None, Some(_)) | (None, None) => None,
        };
        let result = f(&mut state.stdin, &mut state.stdout, &mut buf);
        drop(watchdog);

        if self.timed_out() {
            return Err(self.exited().unwrap().clone());
        }

        result.map_err(|e| {
            let process_exited = matches!(
                e.io.as_ref().map(|it| it.kind()),
                Some(io::ErrorKind::BrokenPipe | io::ErrorKind::UnexpectedEof)
            );
            if !process_exited {
                return e;
            }

            match self.control.process.lock().exit_err() {
                None => e,
                Some(server_error) => self.control.exited.get_or_init(|| server_error).clone(),
            }
        })
    }

    pub(crate) fn run_bidirectional(
        &self,
        initial: BidirectionalMessage,
        callback: SubCallback<'_>,
        macro_name: Option<&str>,
    ) -> Result<BidirectionalMessage, ServerError> {
        self.with_locked_io(Vec::new(), macro_name, |writer, reader, buf| {
            bidirectional_protocol::run_conversation(writer, reader, buf, initial, callback)
        })
    }

    pub(crate) fn number_of_active_req(&self) -> u32 {
        self.active.load(Ordering::Acquire)
    }
}

/// Manages the execution of the proc-macro server process.
#[derive(Debug)]
struct Process {
    child: JodChild,
}

impl Process {
    /// Runs a new proc-macro server process with the specified environment variables.
    fn run<'a>(
        path: &AbsPath,
        env: impl IntoIterator<
            Item = (impl AsRef<std::ffi::OsStr>, &'a Option<impl 'a + AsRef<std::ffi::OsStr>>),
        >,
        format: Option<&str>,
    ) -> io::Result<Process> {
        let child = JodChild(mk_child(path, env, format)?);
        Ok(Process { child })
    }

    /// Retrieves stdin and stdout handles for the process.
    fn stdio(&mut self) -> Option<(ChildStdin, BufReader<ChildStdout>)> {
        let stdin = self.child.stdin.take()?;
        let stdout = self.child.stdout.take()?;
        let read = BufReader::new(stdout);

        Some((stdin, read))
    }
}

/// Creates and configures a new child process for the proc-macro server.
fn mk_child<'a>(
    path: &AbsPath,
    extra_env: impl IntoIterator<
        Item = (impl AsRef<std::ffi::OsStr>, &'a Option<impl 'a + AsRef<std::ffi::OsStr>>),
    >,
    format: Option<&str>,
) -> io::Result<Child> {
    #[allow(clippy::disallowed_methods)]
    let mut cmd = Command::new(path);
    for env in extra_env {
        match env {
            (key, Some(val)) => cmd.env(key, val),
            (key, None) => cmd.env_remove(key),
        };
    }
    if let Some(format) = format {
        cmd.arg("--format");
        cmd.arg(format);
    }
    cmd.env("RUST_ANALYZER_INTERNALS_DO_NOT_USE", "this is unstable")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if cfg!(windows) {
        let mut path_var = std::ffi::OsString::new();
        path_var.push(path.parent().unwrap().parent().unwrap());
        path_var.push("\\bin;");
        path_var.push(std::env::var_os("PATH").unwrap_or_default());
        cmd.env("PATH", path_var);
    }
    cmd.spawn()
}

#[cfg(test)]
mod tests {
    use std::{
        io::Read,
        sync::atomic::{AtomicBool, AtomicUsize},
    };

    use parking_lot::Condvar;

    use crate::pool::{ProcMacroServerPool, ProcessFactory};

    use super::*;

    struct FakeProcess {
        killed: Arc<AtomicBool>,
        wake: Arc<(Mutex<bool>, Condvar)>,
    }

    impl ProcessExit for FakeProcess {
        fn exit_err(&mut self) -> Option<ServerError> {
            None
        }

        fn kill(&mut self) -> io::Result<()> {
            self.killed.store(true, Ordering::Release);
            let (lock, wake) = &*self.wake;
            *lock.lock() = true;
            wake.notify_all();
            Ok(())
        }
    }

    struct BlockingReader {
        wake: Arc<(Mutex<bool>, Condvar)>,
    }

    impl Read for BlockingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            let (lock, wake) = &*self.wake;
            let mut killed = lock.lock();
            while !*killed {
                wake.wait(&mut killed);
            }
            Ok(0)
        }
    }

    impl BufRead for BlockingReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            let mut byte = [0];
            _ = self.read(&mut byte)?;
            Ok(&[])
        }

        fn consume(&mut self, _amt: usize) {}
    }

    fn fake_process(
        watchdog: Option<(ProcessWatchdog, Duration)>,
        killed: Arc<AtomicBool>,
        wake: Arc<(Mutex<bool>, Condvar)>,
    ) -> ProcMacroServerProcess {
        ProcMacroServerProcess {
            state: Mutex::new(ProcessSrvState {
                stdin: Box::new(io::sink()),
                stdout: Box::new(BlockingReader { wake: wake.clone() }),
            }),
            control: ProcessControl::new(Box::new(FakeProcess { killed, wake })),
            watchdog,
            version: 0,
            protocol: Protocol::LegacyJson { mode: SpanMode::Id },
            active: AtomicU32::new(0),
        }
    }

    #[test]
    fn expansion_timeout_kills_process() {
        let killed = Arc::new(AtomicBool::new(false));
        let wake = Arc::new((Mutex::new(false), Condvar::new()));
        let watchdog = Some((ProcessWatchdog::spawn().unwrap(), Duration::from_millis(1000)));
        let process = fake_process(watchdog, killed.clone(), wake);

        let result: Result<(), ServerError> = process.send_task_legacy(
            |_writer, reader, (), _buf| {
                let mut byte = [0];
                _ = reader.read(&mut byte).map_err(|error| ServerError {
                    message: "failed to read response".into(),
                    io: Some(Arc::new(error)),
                })?;
                Ok(None)
            },
            (),
            Some("hang"),
        );

        let error = result.unwrap_err();
        assert!(killed.load(Ordering::Acquire));
        assert!(process.timed_out());
        assert_eq!(error.message, "proc-macro `hang` expansion timed out after 10ms");
    }

    #[test]
    fn completed_expansion_is_disarmed() {
        let killed = Arc::new(AtomicBool::new(false));
        let watchdog = Some((ProcessWatchdog::spawn().unwrap(), Duration::from_millis(10)));
        let process =
            fake_process(watchdog, killed.clone(), Arc::new((Mutex::new(false), Condvar::new())));

        let result: Result<(), ServerError> =
            process.send_task_legacy(|_writer, _reader, (), _buf| Ok(Some(())), (), Some("fast"));

        result.unwrap();
        std::thread::sleep(Duration::from_millis(30));
        assert!(!killed.load(Ordering::Acquire));
        assert!(!process.timed_out());
    }

    #[test]
    fn watchdog_times_out_multiple_processes() {
        let watchdog = ProcessWatchdog::spawn().unwrap();
        let first_killed = Arc::new(AtomicBool::new(false));
        let first =
            fake_process(None, first_killed.clone(), Arc::new((Mutex::new(false), Condvar::new())));
        let second_killed = Arc::new(AtomicBool::new(false));
        let second = fake_process(
            None,
            second_killed.clone(),
            Arc::new((Mutex::new(false), Condvar::new())),
        );

        let first_guard = watchdog.arm(&first.control, "first", Duration::from_millis(10));
        let second_guard = watchdog.arm(&second.control, "second", Duration::from_millis(10));
        std::thread::sleep(Duration::from_millis(30));

        assert!(first_killed.load(Ordering::Acquire));
        assert!(second_killed.load(Ordering::Acquire));
        drop((first_guard, second_guard));
    }

    #[test]
    fn timed_out_process_is_replaced_in_background() {
        let spawn_count = Arc::new(AtomicUsize::new(0));
        let spawn: ProcessFactory = Box::new({
            let spawn_count = spawn_count.clone();
            move || {
                spawn_count.fetch_add(1, Ordering::AcqRel);
                Ok(fake_process(
                    None,
                    Arc::new(AtomicBool::new(false)),
                    Arc::new((Mutex::new(false), Condvar::new())),
                ))
            }
        });
        let pool = Arc::new(ProcMacroServerPool::new(vec![spawn().unwrap()], spawn));
        let process = pool.pick_process().unwrap();

        process.control.time_out("hang", Duration::from_millis(10));
        assert!(process.timed_out());
        pool.replace_timed_out_process_in_background(process.clone());
        pool.replace_timed_out_process_in_background(process.clone());

        let deadline = Instant::now() + Duration::from_secs(1);
        let replacement = loop {
            if let Ok(replacement) = pool.pick_process()
                && !Arc::ptr_eq(&process, &replacement)
            {
                break replacement;
            }
            assert!(Instant::now() < deadline, "timed out waiting for replacement process");
            std::thread::yield_now();
        };
        assert!(!Arc::ptr_eq(&process, &replacement));
        assert_eq!(spawn_count.load(Ordering::Acquire), 2);
    }
}
