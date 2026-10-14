//! 私有管道通信，不监听端口；失效即结束进程，不在按键路径重启模型。
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use qingjian_core::engine::DecisionScorer;
use serde_json::{Value, json};

pub struct SolScorer {
    child: Child,

    input: ChildStdin,

    lines: Receiver<io::Result<String>>,

    failed: bool,
}

impl SolScorer {
    /// 必须在加载线程调用；目录含 venv/bin/python、worker.py 和 model/。
    pub fn load(directory: &Path) -> io::Result<Self> {
        let mut command = Command::new(directory.join("venv/bin/python"));
        command
            .arg("-I")
            .arg("-u")
            .arg(directory.join("worker.py"))
            .env("HF_HUB_OFFLINE", "1")
            .env("TRANSFORMERS_OFFLINE", "1")
            .env("TOKENIZERS_PARALLELISM", "false");
        Self::start(command, Duration::from_secs(60))
    }

    pub(super) fn start(mut command: Command, timeout: Duration) -> io::Result<Self> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut line = String::new();
                let read = reader.by_ref().take(65537).read_line(&mut line);
                match read {
                    Ok(0) => break,
                    Ok(n) if n <= 65536 && line.ends_with('\n') => {
                        if tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    _ => {
                        let _ = tx.send(Err(io::Error::other("invalid worker response")));
                        break;
                    }
                }
            }
        });
        let mut scorer = Self {
            child,
            input,
            lines,
            failed: false,
        };
        if scorer.receive(timeout)?.get("ready") != Some(&Value::Bool(true)) {
            return Err(io::Error::other("Sol worker not ready"));
        }
        Ok(scorer)
    }

    fn receive(&mut self, timeout: Duration) -> io::Result<Value> {
        let line = self
            .lines
            .recv_timeout(timeout)
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Sol worker unavailable"))??;
        serde_json::from_str(&line).map_err(io::Error::other)
    }

    pub(super) fn request(
        &mut self,
        context: &str,
        pinyin: &str,
        texts: &[String],
        timeout: Duration,
    ) -> Vec<f64> {
        if self.failed {
            return Vec::new();
        }
        let result = (|| -> io::Result<Vec<f64>> {
            let request = json!({"context": context, "pinyin": pinyin, "texts": texts});
            writeln!(self.input, "{request}")?;
            self.input.flush()?;
            let response = self.receive(timeout)?;
            let scores: Vec<f64> =
                serde_json::from_value(response["scores"].clone()).map_err(io::Error::other)?;
            if scores.len() != texts.len() || scores.iter().any(|s| !s.is_finite()) {
                return Err(io::Error::other("invalid Sol scores"));
            }
            Ok(scores)
        })();
        match result {
            Ok(scores) => scores,
            Err(_) => {
                tracing::warn!("Sol 决策进程不可用，保留基础候选；重新开启本地模型可重试");
                self.failed = true;
                let _ = self.child.kill();
                let _ = self.child.wait();
                Vec::new()
            }
        }
    }
}

impl DecisionScorer for SolScorer {
    fn decide(&mut self, context: &str, pinyin: &str, texts: &[String]) -> Vec<f64> {
        self.request(context, pinyin, texts, Duration::from_secs(2))
    }
}

impl Drop for SolScorer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
