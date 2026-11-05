//! Generic step tree: the control-flow skeleton shared by curator workflows and laser pipeline
//! reactions. A `Step<A>` is the declarative tree (sequence, parallel, repeat, delay, wait) with
//! domain actions `A` at the leaves; a `StepRunner<A>` is one running instance of it.
//!
//! Runners are host-driven: the host ticks them with the elapsed time, passes in the tags that
//! happened since the last tick, and executes the actions the runner hands back. Instant steps
//! chain within one tick, so `sequence: [a, b, c]` of instant actions all run in the same frame.

use serde::{Deserialize, Serialize};

pub use crate::sequencer::StepProgress;

/// Declarative step tree with domain actions `A` at the leaves.
///
/// In YAML / JSON, control steps are single-key maps (`delay: 0.3`, `sequence: [...]`); anything
/// else is parsed as an action `A`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step<A> {
    /// Wait for a duration in seconds
    Delay(f32),
    /// Wait until the host reports this tag, optionally giving up after a timeout
    Wait(WaitSpec),
    /// Run steps one after another
    Sequence(Vec<Step<A>>),
    /// Run steps together; completes when all have completed
    Parallel(Vec<Step<A>>),
    /// Run a step `count` times (0 = until the runner is dropped)
    Repeat { count: usize, step: Box<Step<A>> },
    /// A domain action, executed by the host
    #[serde(untagged)]
    Action(A),
}

/// Target of a `wait` step: a bare tag, or a tag with a timeout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WaitSpec {
    Tag(String),
    Timeout { tag: String, timeout_secs: f32 },
}

impl WaitSpec {
    pub fn tag(&self) -> &str {
        match self {
            Self::Tag(tag) | Self::Timeout { tag, .. } => tag,
        }
    }

    fn timeout(&self) -> Option<f32> {
        match self {
            Self::Tag(_) => None,
            Self::Timeout { timeout_secs, .. } => Some(*timeout_secs),
        }
    }
}

impl<A> Step<A> {
    /// Visits every action in the tree, depth first
    pub fn for_each_action<'a>(&'a self, f: &mut impl FnMut(&'a A)) {
        match self {
            Self::Delay(_) | Self::Wait(_) => {}
            Self::Sequence(steps) | Self::Parallel(steps) => steps.iter().for_each(|s| s.for_each_action(f)),
            Self::Repeat { step, .. } => step.for_each_action(f),
            Self::Action(action) => f(action),
        }
    }
}

/// Running state of one node of a `Step` tree.
#[derive(Debug, Clone)]
enum Node<A> {
    Delay { remaining: f32 },
    Wait { tag: String, remaining: Option<f32>, received: bool },
    Sequence { steps: Vec<Step<A>>, index: usize, current: Option<Box<Node<A>>> },
    Parallel { nodes: Vec<(Node<A>, bool)> },
    Repeat { count: usize, done: usize, step: Box<Step<A>>, current: Option<Box<Node<A>>> },
    Action { action: A, executed: bool },
}

impl<A: Clone> Node<A> {
    fn new(step: &Step<A>) -> Self {
        match step {
            Step::Delay(secs) => Self::Delay { remaining: *secs },
            Step::Wait(spec) => Self::Wait { tag: spec.tag().to_string(), remaining: spec.timeout(), received: false },
            Step::Sequence(steps) => Self::Sequence { steps: steps.clone(), index: 0, current: None },
            Step::Parallel(steps) => Self::Parallel { nodes: steps.iter().map(|s| (Self::new(s), false)).collect() },
            Step::Repeat { count, step } => Self::Repeat { count: *count, done: 0, step: step.clone(), current: None },
            Step::Action(action) => Self::Action { action: action.clone(), executed: false },
        }
    }

    fn notify(&mut self, tag: &str) {
        match self {
            Self::Wait { tag: wanted, received, .. } => {
                if wanted == tag {
                    *received = true;
                }
            }
            Self::Sequence { current, .. } | Self::Repeat { current, .. } => {
                if let Some(node) = current {
                    node.notify(tag);
                }
            }
            Self::Parallel { nodes } => nodes.iter_mut().filter(|(_, done)| !done).for_each(|(n, _)| n.notify(tag)),
            Self::Delay { .. } | Self::Action { .. } => {}
        }
    }

    /// Advances by `dt`. Nodes started during this tick get `dt = 0`, so instant steps chain
    /// but time is not counted twice.
    fn tick(&mut self, dt: f32, run: &mut impl FnMut(&A)) -> StepProgress {
        match self {
            Self::Delay { remaining } => {
                *remaining -= dt;
                if *remaining <= 0.0 { StepProgress::Completed } else { StepProgress::Running }
            }
            Self::Wait { received, remaining, .. } => {
                if *received {
                    return StepProgress::Completed;
                }
                match remaining {
                    Some(left) => {
                        *left -= dt;
                        if *left <= 0.0 { StepProgress::Completed } else { StepProgress::Running }
                    }
                    None => StepProgress::Running,
                }
            }
            Self::Action { action, executed } => {
                if !*executed {
                    *executed = true;
                    run(action);
                }
                StepProgress::Completed
            }
            Self::Sequence { steps, index, current } => {
                let mut dt = dt;
                loop {
                    if current.is_none() {
                        let Some(step) = steps.get(*index) else {
                            return StepProgress::Completed;
                        };
                        *current = Some(Box::new(Self::new(step)));
                    }
                    match current.as_mut().unwrap().tick(dt, run) {
                        StepProgress::Completed => {
                            *current = None;
                            *index += 1;
                            dt = 0.0;
                        }
                        other => return other,
                    }
                }
            }
            Self::Parallel { nodes } => {
                let mut all_done = true;
                for (node, done) in nodes.iter_mut().filter(|(_, done)| !*done) {
                    match node.tick(dt, run) {
                        StepProgress::Completed => *done = true,
                        StepProgress::Running => all_done = false,
                        failed @ StepProgress::Failed(_) => return failed,
                    }
                }
                if all_done { StepProgress::Completed } else { StepProgress::Running }
            }
            Self::Repeat { count, done, step, current } => {
                if *count != 0 && *done >= *count {
                    return StepProgress::Completed;
                }
                let node = current.get_or_insert_with(|| Box::new(Self::new(step)));
                match node.tick(dt, run) {
                    // At most one iteration completes per tick, so an instant body repeated
                    // forever cannot hang the frame
                    StepProgress::Completed => {
                        *current = None;
                        *done += 1;
                        if *count != 0 && *done >= *count { StepProgress::Completed } else { StepProgress::Running }
                    }
                    other => other,
                }
            }
        }
    }
}

/// One running instance of a `Step` tree.
#[derive(Debug, Clone)]
pub struct StepRunner<A> {
    root: Node<A>,
    status: StepProgress,
}

impl<A: Clone> StepRunner<A> {
    pub fn new(step: &Step<A>) -> Self {
        Self { root: Node::new(step), status: StepProgress::Running }
    }

    /// Reports a tag to every `wait` step currently waiting
    pub fn notify(&mut self, tag: &str) {
        if self.is_running() {
            self.root.notify(tag);
        }
    }

    /// Advances by `dt` seconds, calling `run` for every action reached
    pub fn tick(&mut self, dt: f32, mut run: impl FnMut(&A)) -> &StepProgress {
        if self.is_running() {
            self.status = self.root.tick(dt, &mut run);
        }
        &self.status
    }

    pub fn is_running(&self) -> bool {
        self.status == StepProgress::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum Act {
        Say(String),
    }

    fn say(s: &str) -> Step<Act> {
        Step::Action(Act::Say(s.into()))
    }

    fn run(runner: &mut StepRunner<Act>, dt: f32) -> Vec<String> {
        let mut out = Vec::new();
        runner.tick(dt, |Act::Say(s)| out.push(s.clone()));
        out
    }

    #[test]
    fn instant_sequence_runs_in_one_tick() {
        let mut r = StepRunner::new(&Step::Sequence(vec![say("a"), say("b"), say("c")]));
        assert_eq!(run(&mut r, 0.0), vec!["a", "b", "c"]);
        assert!(!r.is_running());
    }

    #[test]
    fn delay_splits_sequence_across_ticks() {
        let mut r = StepRunner::new(&Step::Sequence(vec![say("a"), Step::Delay(0.5), say("b")]));
        assert_eq!(run(&mut r, 0.1), vec!["a"]);
        assert_eq!(run(&mut r, 0.3), Vec::<String>::new());
        assert_eq!(run(&mut r, 0.2), vec!["b"]);
        assert!(!r.is_running());
    }

    #[test]
    fn newly_started_delay_does_not_consume_the_same_tick() {
        // `a` completes instantly; the delay starts in the same tick and must not count its dt
        let mut r = StepRunner::new(&Step::Sequence(vec![say("a"), Step::Delay(0.5), say("b")]));
        assert_eq!(run(&mut r, 1.0), vec!["a"]);
        assert_eq!(run(&mut r, 0.5), vec!["b"]);
    }

    #[test]
    fn wait_completes_on_tag_or_timeout() {
        let mut r = StepRunner::new(&Step::Sequence(vec![Step::Wait(WaitSpec::Tag("done".into())), say("x")]));
        assert!(run(&mut r, 10.0).is_empty());
        r.notify("other");
        assert!(run(&mut r, 0.0).is_empty());
        r.notify("done");
        assert_eq!(run(&mut r, 0.0), vec!["x"]);

        let mut t = StepRunner::new(&Step::Sequence(vec![
            Step::Wait(WaitSpec::Timeout { tag: "never".into(), timeout_secs: 0.2 }),
            say("y"),
        ]));
        assert!(run(&mut t, 0.1).is_empty());
        assert_eq!(run(&mut t, 0.1), vec!["y"]);
    }

    #[test]
    fn parallel_completes_when_all_branches_complete() {
        let mut r = StepRunner::new(&Step::Parallel(vec![
            say("a"),
            Step::Sequence(vec![Step::Delay(0.2), say("b")]),
        ]));
        assert_eq!(run(&mut r, 0.1), vec!["a"]);
        assert!(r.is_running());
        assert_eq!(run(&mut r, 0.1), vec!["b"]);
        assert!(!r.is_running());
    }

    #[test]
    fn repeat_runs_one_iteration_per_tick_at_most() {
        let mut r = StepRunner::new(&Step::Repeat { count: 3, step: Box::new(say("x")) });
        assert_eq!(run(&mut r, 0.0), vec!["x"]);
        assert_eq!(run(&mut r, 0.0), vec!["x"]);
        assert_eq!(run(&mut r, 0.0), vec!["x"]);
        assert!(!r.is_running());

        let mut forever = StepRunner::new(&Step::Repeat { count: 0, step: Box::new(say("x")) });
        for _ in 0..5 {
            assert_eq!(run(&mut forever, 0.0), vec!["x"]);
        }
        assert!(forever.is_running());
    }

    #[test]
    fn deserializes_control_steps_and_actions() {
        let json = r#"{"sequence": [
            {"parallel": [{"say": "a"}, {"delay": 0.1}]},
            {"wait": "finished"},
            {"wait": {"tag": "hit", "timeout_secs": 2.0}},
            {"repeat": {"count": 2, "step": {"say": "b"}}}
        ]}"#;
        let step: Step<Act> = serde_json::from_str(json).unwrap();
        assert_eq!(
            step,
            Step::Sequence(vec![
                Step::Parallel(vec![say("a"), Step::Delay(0.1)]),
                Step::Wait(WaitSpec::Tag("finished".into())),
                Step::Wait(WaitSpec::Timeout { tag: "hit".into(), timeout_secs: 2.0 }),
                Step::Repeat { count: 2, step: Box::new(say("b")) },
            ])
        );
    }

    #[test]
    fn for_each_action_visits_all_leaves() {
        let step = Step::Sequence(vec![say("a"), Step::Parallel(vec![say("b"), Step::Delay(1.0)])]);
        let mut seen = Vec::new();
        step.for_each_action(&mut |Act::Say(s)| seen.push(s.clone()));
        assert_eq!(seen, vec!["a", "b"]);
    }
}
