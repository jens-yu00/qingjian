//! 一次稳定输入对应一次整批决策，分数不可跨候选集复用。
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use super::{DecisionScorer, request::Request};

pub(crate) struct DecisionState {
    jobs: Sender<Request>,

    results: Receiver<(Request, Vec<f64>)>,

    revision: u64,

    request: Option<Request>,

    sent: Option<Instant>,

    scores: Option<Vec<f64>>,

    frozen: bool,

    pub promoted: Option<String>,
}

impl DecisionState {
    pub fn new(mut scorer: Box<dyn DecisionScorer>) -> Self {
        let (jobs, rx) = channel::<Request>();
        let (tx, results) = channel();
        std::thread::spawn(move || {
            while let Ok(mut request) = rx.recv() {
                while let Ok(newer) = rx.try_recv() {
                    request = newer;
                }
                let scores = scorer.decide(&request.context, &request.pinyin, &request.texts);
                if tx.send((request, scores)).is_err() {
                    break;
                }
            }
        });
        Self {
            jobs,
            results,
            revision: 0,
            request: None,
            sent: None,
            scores: None,
            frozen: false,
            promoted: None,
        }
    }

    pub fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.request = None;
        self.sent = None;
        self.scores = None;
        self.frozen = false;
        self.promoted = None;
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    pub fn prepare(
        &mut self,
        context: String,
        pinyin: &str,
        texts: Vec<String>,
    ) -> Option<Vec<f64>> {
        let request = Request {
            revision: self.revision,
            context,
            pinyin: pinyin.into(),
            texts,
        };
        if self.request.as_ref() != Some(&request) {
            if self.frozen {
                return None;
            }
            self.request = Some(request);
            self.sent = None;
            self.scores = None;
            self.promoted = None;
        }
        self.scores.clone()
    }

    pub fn pending(&self) -> bool {
        !self.frozen && self.request.is_some() && self.sent.is_none()
    }

    pub fn submit(&mut self) -> bool {
        if !self.pending() {
            return false;
        }
        self.sent = Some(Instant::now());
        self.jobs.send(self.request.clone().unwrap()).is_ok()
    }

    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        while let Ok((request, scores)) = self.results.try_recv() {
            if !self.frozen
                && self.request.as_ref() == Some(&request)
                && self
                    .sent
                    .is_some_and(|time| time.elapsed() < Duration::from_secs(2))
                && scores.len() == request.texts.len()
                && scores.iter().all(|s| s.is_finite())
            {
                self.scores = Some(scores);
                updated = true;
            }
        }
        updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixed;
    impl DecisionScorer for Fixed {
        fn decide(&mut self, _: &str, _: &str, _: &[String]) -> Vec<f64> {
            vec![0.0, 4.0]
        }
    }
    #[test]
    fn completed_after_authority_deadline_cannot_change_candidates() {
        let mut state = DecisionState::new(Box::new(Fixed));
        state.prepare("上文".into(), "shi", vec!["是".into(), "事".into()]);
        state.submit();
        state.sent = Some(Instant::now() - Duration::from_secs(3));
        let (tx, rx) = channel();
        state.results = rx;
        tx.send((state.request.clone().unwrap(), vec![0.0, 4.0]))
            .unwrap();
        assert!(!state.poll());
        assert!(state.scores.is_none());
        assert!(!state.pending(), "超时不能在同一输入上不断重试");
    }
}
