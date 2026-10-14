//! 用真实管道验证协议损坏、超时和进程回收；不依赖 MLX。
#![cfg(unix)]
use super::SolScorer;
use std::process::Command;
use std::time::{Duration, Instant};

fn worker(script: &str) -> SolScorer {
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg(script);
    SolScorer::start(command, Duration::from_secs(2)).unwrap()
}
#[test]
fn scores_keep_batch_order_and_malformed_response_disables_worker() {
    let mut worker = worker(
        "printf '%s\\n' '{\"ready\":true}'; read x; printf '%s\\n' '{\"scores\":[0.5,3.0]}'; read x; printf '%s\\n' '{\"scores\":[9]}'",
    );
    let words = vec!["实验".into(), "食盐".into()];
    assert_eq!(
        worker.request("上文", "shiyan", &words, Duration::from_secs(1)),
        [0.5, 3.0]
    );
    assert!(
        worker
            .request("上文", "shiyan", &words, Duration::from_secs(1))
            .is_empty()
    );
    assert!(
        worker
            .request("上文", "shiyan", &words, Duration::from_secs(1))
            .is_empty()
    );
}
#[test]
fn stalled_process_is_killed_at_deadline() {
    let mut worker = worker("printf '%s\\n' '{\"ready\":true}'; read x; read y");
    let started = Instant::now();
    assert!(
        worker
            .request(
                "上文",
                "shi",
                &["是".into(), "事".into()],
                Duration::from_millis(30)
            )
            .is_empty()
    );
    assert!(started.elapsed() < Duration::from_secs(1));
}
#[test]
fn missing_ready_is_an_error() {
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg("printf '%s\\n' '{\"ready\":false}'");
    assert!(SolScorer::start(command, Duration::from_secs(1)).is_err());
}
