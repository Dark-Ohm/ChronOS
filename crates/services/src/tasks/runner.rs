//! Process runner: spawn task, stream stdout/stderr lines, cancel via kill.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::buffer::{LogBuffer, LogLine, StreamKind};
use super::config::TaskDef;

/// Lifecycle of a single run. Never invent "ok" without a completed process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStatus {
    /// No run has been started for this slot yet.
    Idle,
    Running {
        started: Instant,
    },
    Ok {
        code: i32,
        duration: Duration,
    },
    Failed {
        /// Exit code when the process exited; `None` if kill/spawn failed mid-way.
        code: Option<i32>,
        duration: Duration,
        detail: String,
    },
}

impl RunStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, RunStatus::Running { .. })
    }
}

enum WorkerMsg {
    Line(LogLine),
    Done {
        status: RunStatus,
    },
}

/// Owns at most one child process and a growing log buffer.
pub struct TaskSession {
    status: RunStatus,
    buffer: LogBuffer,
    child: Option<Arc<Mutex<Child>>>,
    /// OS pid for cancel without contending the waiter mutex.
    child_pid: Option<u32>,
    rx: Option<Receiver<WorkerMsg>>,
    /// Task id of the last/current run.
    active_task_id: Option<String>,
}

impl Default for TaskSession {
    fn default() -> Self {
        Self::new(super::buffer::DEFAULT_LOG_CAP)
    }
}

impl TaskSession {
    pub fn new(cap: usize) -> Self {
        Self {
            status: RunStatus::Idle,
            buffer: LogBuffer::new(cap),
            child: None,
            child_pid: None,
            rx: None,
            active_task_id: None,
        }
    }

    pub fn status(&self) -> &RunStatus {
        &self.status
    }

    pub fn buffer(&self) -> &LogBuffer {
        &self.buffer
    }

    pub fn active_task_id(&self) -> Option<&str> {
        self.active_task_id.as_deref()
    }

    /// Drain worker messages into the buffer/status. Call from UI poll.
    pub fn poll(&mut self) {
        let Some(rx) = self.rx.as_ref() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(WorkerMsg::Line(line)) => self.buffer.push(line),
                Ok(WorkerMsg::Done { status }) => {
                    self.status = status;
                    self.child = None;
                    self.child_pid = None;
                    self.rx = None;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.status.is_running() {
                        self.status = RunStatus::Failed {
                            code: None,
                            duration: Duration::ZERO,
                            detail: "task worker disconnected".into(),
                        };
                    }
                    self.child = None;
                    self.rx = None;
                    break;
                }
            }
        }
    }

    /// Start `task` in `cwd`. Refuses if already running.
    pub fn start(&mut self, task: &TaskDef, cwd: &Path) -> Result<(), String> {
        if self.status.is_running() {
            return Err("a task is already running".into());
        }

        self.buffer.clear();
        self.active_task_id = Some(task.id.clone());
        let started = Instant::now();
        self.status = RunStatus::Running { started };

        let mut cmd = Command::new(&task.command);
        cmd.args(&task.args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let detail = if e.kind() == std::io::ErrorKind::NotFound {
                    format!(
                        "executable not found in PATH: '{}' ({e})",
                        task.command
                    )
                } else {
                    format!("failed to spawn '{}': {e}", task.command)
                };
                self.buffer
                    .push_str(StreamKind::System, detail.clone());
                self.status = RunStatus::Failed {
                    code: None,
                    duration: started.elapsed(),
                    detail: detail.clone(),
                };
                self.child = None;
                self.child_pid = None;
                self.rx = None;
                return Err(detail);
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let child_pid = child.id();
        let child = Arc::new(Mutex::new(child));
        self.child = Some(Arc::clone(&child));
        self.child_pid = Some(child_pid);

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        // stdout reader
        if let Some(out) = stdout {
            let tx = tx.clone();
            thread::Builder::new()
                .name("chronos-task-stdout".into())
                .spawn(move || read_pipe(out, StreamKind::Stdout, &tx))
                .map_err(|e| e.to_string())?;
        }
        // stderr reader
        if let Some(err) = stderr {
            let tx = tx.clone();
            thread::Builder::new()
                .name("chronos-task-stderr".into())
                .spawn(move || read_pipe(err, StreamKind::Stderr, &tx))
                .map_err(|e| e.to_string())?;
        }

        // waiter
        let child_wait = Arc::clone(&child);
        thread::Builder::new()
            .name("chronos-task-wait".into())
            .spawn(move || {
                let status = match child_wait.lock() {
                    Ok(mut guard) => match guard.wait() {
                        Ok(st) => {
                            let code = st.code().unwrap_or(-1);
                            let duration = started.elapsed();
                            if st.success() {
                                RunStatus::Ok { code, duration }
                            } else {
                                RunStatus::Failed {
                                    code: Some(code),
                                    duration,
                                    detail: format!("exited with code {code}"),
                                }
                            }
                        }
                        Err(e) => RunStatus::Failed {
                            code: None,
                            duration: started.elapsed(),
                            detail: format!("wait failed: {e}"),
                        },
                    },
                    Err(_) => RunStatus::Failed {
                        code: None,
                        duration: started.elapsed(),
                        detail: "child lock poisoned".into(),
                    },
                };
                let _ = tx.send(WorkerMsg::Done { status });
            })
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Kill the running process (not just stop reading).
    pub fn cancel(&mut self) {
        if !self.status.is_running() {
            return;
        }
        let started = match &self.status {
            RunStatus::Running { started } => *started,
            _ => Instant::now(),
        };
        // Kill by pid: the waiter thread may hold the Child mutex on `wait()`.
        if let Some(pid) = self.child_pid {
            let _ = std::process::Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status();
        }
        self.child_pid = None;
        self.buffer
            .push_str(StreamKind::System, "cancelled by user");
        self.status = RunStatus::Failed {
            code: None,
            duration: started.elapsed(),
            detail: "cancelled by user".into(),
        };
        self.child = None;
        self.child_pid = None;
        // Drain remaining messages without overwriting cancel status.
        if let Some(rx) = self.rx.take() {
            while let Ok(msg) = rx.try_recv() {
                if let WorkerMsg::Line(line) = msg {
                    self.buffer.push(line);
                }
            }
        }
    }
}

fn read_pipe<R: std::io::Read>(reader: R, kind: StreamKind, tx: &Sender<WorkerMsg>) {
    let mut lines = BufReader::new(reader).lines();
    while let Some(item) = lines.next() {
        match item {
            Ok(text) => {
                if tx
                    .send(WorkerMsg::Line(LogLine {
                        stream: kind,
                        text,
                    }))
                    .is_err()
                {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn echo_task(msg: &str) -> TaskDef {
        TaskDef {
            id: "echo".into(),
            label: "echo".into(),
            command: "echo".into(),
            args: vec![msg.into()],
        }
    }

    fn true_task() -> TaskDef {
        TaskDef {
            id: "true".into(),
            label: "true".into(),
            command: "true".into(),
            args: vec![],
        }
    }

    fn false_task() -> TaskDef {
        TaskDef {
            id: "false".into(),
            label: "false".into(),
            command: "false".into(),
            args: vec![],
        }
    }

    fn wait_done(session: &mut TaskSession, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            session.poll();
            if !session.status().is_running() {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        session.poll();
    }

    #[test]
    fn echo_produces_stdout_and_ok() {
        let mut s = TaskSession::new(100);
        let cwd = std::env::temp_dir();
        s.start(&echo_task("hello-t178"), &cwd).unwrap();
        wait_done(&mut s, Duration::from_secs(3));
        match s.status() {
            RunStatus::Ok { code, .. } => assert_eq!(*code, 0),
            other => panic!("expected Ok, got {other:?}"),
        }
        let joined: String = s
            .buffer()
            .lines()
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("hello-t178"), "got: {joined}");
        assert!(
            s.buffer()
                .lines()
                .iter()
                .any(|l| l.stream == StreamKind::Stdout)
        );
    }

    #[test]
    fn false_is_failed_with_code() {
        let mut s = TaskSession::new(100);
        let cwd = std::env::temp_dir();
        s.start(&false_task(), &cwd).unwrap();
        wait_done(&mut s, Duration::from_secs(3));
        match s.status() {
            RunStatus::Failed {
                code: Some(c), ..
            } => assert_ne!(*c, 0),
            other => panic!("expected Failed with code, got {other:?}"),
        }
    }

    #[test]
    fn true_is_ok() {
        let mut s = TaskSession::new(100);
        s.start(&true_task(), &std::env::temp_dir()).unwrap();
        wait_done(&mut s, Duration::from_secs(3));
        assert!(matches!(s.status(), RunStatus::Ok { code: 0, .. }));
    }

    #[test]
    fn missing_binary_is_honest_error() {
        let mut s = TaskSession::new(100);
        let task = TaskDef {
            id: "nope".into(),
            label: "nope".into(),
            command: "chronos-t178-definitely-not-on-path-xyz".into(),
            args: vec![],
        };
        let err = s.start(&task, &std::env::temp_dir()).unwrap_err();
        assert!(err.contains("not found") || err.contains("No such"));
        assert!(matches!(s.status(), RunStatus::Failed { .. }));
        assert!(!matches!(s.status(), RunStatus::Ok { .. }));
    }

    #[test]
    fn cancel_kills_long_running() {
        let mut s = TaskSession::new(100);
        let task = TaskDef {
            id: "sleep".into(),
            label: "sleep".into(),
            command: "sleep".into(),
            args: vec!["30".into()],
        };
        s.start(&task, &std::env::temp_dir()).unwrap();
        thread::sleep(Duration::from_millis(50));
        s.cancel();
        assert!(matches!(
            s.status(),
            RunStatus::Failed {
                detail: d,
                ..
            } if d.contains("cancelled")
        ));
        // Ensure no orphan sleep from this test (best-effort; other sleeps may exist).
        // We only assert our session is not running.
        assert!(!s.status().is_running());
    }

    #[test]
    fn idle_never_pretends_ok() {
        let s = TaskSession::new(10);
        assert_eq!(s.status(), &RunStatus::Idle);
    }
}
