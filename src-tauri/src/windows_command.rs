#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
pub fn hidden_command(program: &str) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

use std::cell::Cell;
use std::io::{self, Read};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

thread_local! {
    // 扫描在单个阻塞工作线程完成；预算不影响其他线程的终止身份查询。
    static QUERY_DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
}

pub(crate) struct QueryBudget {
    deadline: Instant,
    previous: Option<Instant>,
}

impl QueryBudget {
    pub(crate) fn new(timeout: Duration) -> Self {
        let deadline = Instant::now() + timeout;
        let previous = QUERY_DEADLINE.replace(Some(deadline));
        Self { deadline, previous }
    }

    pub(crate) fn check(&self) -> Result<(), String> {
        if Instant::now() >= self.deadline {
            Err("Windows 进程详情查询超过本轮扫描时限，请重试".into())
        } else {
            Ok(())
        }
    }
}

impl Drop for QueryBudget {
    fn drop(&mut self) {
        QUERY_DEADLINE.set(self.previous);
    }
}

fn query_timeout() -> io::Result<Duration> {
    let limit = Duration::from_secs(25);
    match QUERY_DEADLINE.get() {
        Some(deadline) => deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .map(|remaining| remaining.min(limit))
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "scan query budget exhausted")),
        None => Ok(limit),
    }
}

#[cfg(windows)]
pub(crate) fn powershell_output(script: &str, context: &str) -> Result<Output, String> {
    let timeout = query_timeout().map_err(|e| format!("{context}: {e}"))?;
    let output = output_with_timeout(
        hidden_command("powershell").args(["-NoProfile", "-NonInteractive", "-Command", script]),
        timeout,
    )
    .map_err(|e| format!("{context}: {e}"))?;
    if !output.status.success() {
        let detail: String = String::from_utf8_lossy(&output.stderr)
            .trim()
            .chars()
            .take(300)
            .collect();
        return Err(format!("{context}失败（{}）: {detail}", output.status));
    }
    Ok(output)
}

fn output_with_timeout(command: &mut Command, timeout: Duration) -> io::Result<Output> {
    if timeout.is_zero() {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "query budget exhausted",
        ));
    }
    let deadline = Instant::now() + timeout;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let pid = child.id();
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");

    std::thread::scope(|scope| {
        // 全量快照可能超过管道容量；等待退出的同时必须读取两个输出管道。
        let read = |mut pipe: Box<dyn Read + Send>| -> io::Result<Vec<u8>> {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes)?;
            Ok(bytes)
        };
        let stdout_reader = scope.spawn(move || read(Box::new(stdout)));
        let stderr_reader = scope.spawn(move || read(Box::new(stderr)));
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                result => {
                    // 这里只停止本工具启动的查询子进程；等待回收后才释放扫描锁。
                    let cleanup = child.kill().and_then(|_| child.wait());
                    break match cleanup {
                        Err(error) => Err(error),
                        Ok(_) => Err(match result {
                            Err(error) => error,
                            _ => io::Error::new(
                                io::ErrorKind::TimedOut,
                                format!(
                                    "query process {pid} timed out after {} ms",
                                    timeout.as_millis()
                                ),
                            ),
                        }),
                    };
                }
            }
        };
        // 无论成功还是超时，都回收读取线程，避免刷新后留下后台任务。
        let stdout = stdout_reader
            .join()
            .map_err(|_| io::Error::other("stdout reader panicked"));
        let stderr = stderr_reader
            .join()
            .map_err(|_| io::Error::other("stderr reader panicked"));
        Ok(Output {
            status: status?,
            stdout: stdout??,
            stderr: stderr??,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn child_command(mode: &str) -> std::process::Command {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        // cargo test 和单独 rustc 测试的名称都不包含 crate 名称。
        let module = module_path!().split_once("::").unwrap().1;
        command.args([
            "--exact",
            &format!("{module}::command_child"),
            "--nocapture",
        ]);
        command.env("PORT_GUARDIAN_COMMAND_TEST_MODE", mode);
        command
    }

    #[test]
    fn command_child() {
        match std::env::var("PORT_GUARDIAN_COMMAND_TEST_MODE").as_deref() {
            Ok("hang") => std::thread::sleep(Duration::from_millis(800)),
            Ok("large-output") => {
                use std::io::Write;
                let bytes = vec![b'x'; 1024 * 1024];
                std::io::stdout().write_all(&bytes).unwrap();
                std::io::stderr().write_all(&bytes).unwrap();
            }
            Ok("failure") => {
                eprintln!("query failed");
                std::process::exit(7);
            }
            _ => {}
        }
    }

    #[test]
    fn hanging_query_times_out_and_next_query_can_finish() {
        let started = Instant::now();
        let error = output_with_timeout(&mut child_command("hang"), Duration::from_millis(100))
            .expect_err("a blocked query must be stopped");
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_millis(700));
        let message = error.to_string();
        let pid = message.split_whitespace().nth(2).unwrap();
        #[cfg(unix)]
        assert!(
            !Command::new("kill")
                .args(["-0", pid])
                .output()
                .unwrap()
                .status
                .success()
        );
        #[cfg(windows)]
        {
            let output = hidden_command("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
                .output()
                .unwrap();
            assert!(!String::from_utf8_lossy(&output.stdout).contains(&format!(",\"{pid}\",")));
        }
        assert!(
            output_with_timeout(&mut child_command("ok"), Duration::from_secs(5))
                .unwrap()
                .status
                .success()
        );
    }

    #[test]
    fn drains_both_pipes_while_waiting_for_large_snapshot() {
        let output =
            output_with_timeout(&mut child_command("large-output"), Duration::from_secs(5))
                .unwrap();
        assert!(output.status.success());
        assert!(output.stdout.len() >= 1024 * 1024);
        assert_eq!(output.stderr.len(), 1024 * 1024);
    }

    #[test]
    fn retains_query_failure_status_and_stderr() {
        let output =
            output_with_timeout(&mut child_command("failure"), Duration::from_secs(5)).unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert!(String::from_utf8_lossy(&output.stderr).contains("query failed"));
    }

    #[test]
    fn exhausted_scan_budget_prevents_more_queries_and_is_restored() {
        let budget = QueryBudget::new(Duration::ZERO);
        assert!(budget.check().is_err());
        assert_eq!(query_timeout().unwrap_err().kind(), io::ErrorKind::TimedOut);
        drop(budget);
        assert_eq!(query_timeout().unwrap(), Duration::from_secs(25));
    }

    #[test]
    fn query_timeout_uses_remaining_scan_budget() {
        let _budget = QueryBudget::new(Duration::from_millis(200));
        assert!(query_timeout().unwrap() <= Duration::from_millis(200));
    }

    #[cfg(windows)]
    #[test]
    fn real_powershell_timeout_does_not_block_next_query() {
        let budget = QueryBudget::new(Duration::from_millis(200));
        let error = powershell_output("Start-Sleep -Seconds 20", "test hanging query")
            .expect_err("PowerShell must be terminated when its scan budget expires");
        assert!(error.contains("test hanging query"));
        assert!(error.contains("timed out") || error.contains("budget exhausted"));
        drop(budget);
        let output = powershell_output("Write-Output 'query recovered'", "test recovery").unwrap();
        assert!(String::from_utf8_lossy(&output.stdout).contains("query recovered"));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_failure_returns_context_instead_of_empty_snapshot() {
        let error = powershell_output("throw 'test query failure'", "test process prefetch")
            .expect_err("failed WMI-style queries cannot be accepted as empty snapshots");
        assert!(error.contains("test process prefetch"));
        assert!(error.contains("test query failure"));
    }
}
