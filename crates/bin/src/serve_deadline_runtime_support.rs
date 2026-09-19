use super::{run, DeadlineRuntimeConfig, RuntimeControl};
use application::{
    deadline_dispatch::*, deadline_worker::*, deadlines::DeadlineError, ApplicationError,
    PortFailureKind,
};
use domain::typed_participants::Uuid;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

#[path = "serve_deadline_runtime_values.rs"]
mod values;

pub const POLL: Duration = Duration::from_millis(7);
pub const WATCHDOG: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Call {
    Dispatch(DeadlineDispatchStream, u32),
    Worker,
    Wait(Duration),
}

#[derive(Debug, Clone, Copy)]
pub enum Failure {
    Port,
    Classified(PortFailureKind),
    Integrity,
    Configuration,
}

impl Failure {
    fn error(self) -> ApplicationError {
        match self {
            Self::Port => ApplicationError::Port("scripted port failure".into()),
            Self::Classified(kind) => ApplicationError::ClassifiedPort {
                kind,
                message: "scripted classified failure".into(),
            },
            Self::Integrity => DeadlineError::StoredInconsistent("scripted evidence".into()).into(),
            Self::Configuration => {
                ApplicationError::InvalidConfiguration("scripted inventory".into())
            }
        }
    }
}

pub enum DispatchStep {
    Page,
    Empty,
    Fail(Failure),
}

pub enum WorkerStep {
    Completed,
    Deferred(bool),
    Idle,
    Fail(Failure),
}

pub struct State {
    calls: Mutex<Vec<Call>>,
    stopped: AtomicBool,
    waits: AtomicUsize,
    stop_after_waits: usize,
    dispatches: AtomicUsize,
    workers: AtomicUsize,
    stop_dispatch: AtomicUsize,
    stop_worker: AtomicUsize,
    active: AtomicUsize,
    maximum: AtomicUsize,
}

impl State {
    fn new(stop_after_waits: usize) -> Self {
        assert!(stop_after_waits > 0);
        Self {
            calls: Mutex::new(Vec::new()),
            stopped: AtomicBool::new(false),
            waits: AtomicUsize::new(0),
            stop_after_waits,
            dispatches: AtomicUsize::new(0),
            workers: AtomicUsize::new(0),
            stop_dispatch: AtomicUsize::new(0),
            stop_worker: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
        }
    }

    pub fn stop(&self) {
        self.stopped.store(true, SeqCst);
    }

    pub fn stop_after_dispatch(&self, count: usize) {
        self.stop_dispatch.store(count, SeqCst);
    }

    pub fn stop_after_worker(&self, count: usize) {
        self.stop_worker.store(count, SeqCst);
    }

    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    pub fn active(&self) -> usize {
        self.active.load(SeqCst)
    }

    pub fn maximum(&self) -> usize {
        self.maximum.load(SeqCst)
    }

    fn push(&self, call: Call) {
        let mut calls = self.calls.lock().unwrap();
        assert!(
            calls.len() < 1024,
            "consumer ignored its bounded cycle or stop"
        );
        calls.push(call);
    }

    fn enter(&self, call: Call) -> Active<'_> {
        let active = self.active.fetch_add(1, SeqCst) + 1;
        self.maximum.fetch_max(active, SeqCst);
        self.push(call);
        let (count, stop) = match call {
            Call::Dispatch(..) => (&self.dispatches, &self.stop_dispatch),
            Call::Worker => (&self.workers, &self.stop_worker),
            Call::Wait(_) => unreachable!(),
        };
        if count.fetch_add(1, SeqCst) + 1 == stop.load(SeqCst) {
            self.stop();
        }
        Active(self)
    }
}

struct Active<'a>(&'a State);

impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, SeqCst);
    }
}

impl RuntimeControl for State {
    fn is_stopped(&self) -> bool {
        self.stopped.load(SeqCst)
    }

    fn wait(&self, duration: Duration) {
        assert_eq!(
            self.active(),
            0,
            "consumer waited before a port call finished"
        );
        self.push(Call::Wait(duration));
        if self.waits.fetch_add(1, SeqCst) + 1 >= self.stop_after_waits {
            self.stop();
        }
    }
}

pub struct Gate {
    first: AtomicBool,
    entered: mpsc::SyncSender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Gate {
    pub fn new() -> (Arc<Self>, mpsc::Receiver<()>, mpsc::SyncSender<()>) {
        let (entered_tx, entered) = mpsc::sync_channel(1);
        let (release, release_rx) = mpsc::sync_channel(1);
        (
            Arc::new(Self {
                first: AtomicBool::new(true),
                entered: entered_tx,
                release: Mutex::new(release_rx),
            }),
            entered,
            release,
        )
    }

    fn hold(&self) {
        if self.first.swap(false, SeqCst) {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(WATCHDOG)
                .expect("gate release watchdog");
        }
    }
}

pub struct Dispatcher {
    state: Arc<State>,
    script: Mutex<VecDeque<DispatchStep>>,
    pub gate: Option<Arc<Gate>>,
}

impl DeadlineDispatchStore for Dispatcher {
    fn dispatch(
        &self,
        request: DeadlineDispatchRequest,
    ) -> Result<DeadlineDispatchBatch, ApplicationError> {
        let _active = self
            .state
            .enter(Call::Dispatch(request.stream, request.limit.get()));
        if let Some(gate) = &self.gate {
            gate.hold();
        }
        let selected = match self
            .script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(DispatchStep::Page)
        {
            DispatchStep::Page => request.limit.get(),
            DispatchStep::Empty => 0,
            DispatchStep::Fail(error) => return Err(error.error()),
        };
        Ok(values::batch(request, selected))
    }
}

pub struct Worker {
    state: Arc<State>,
    script: Mutex<VecDeque<WorkerStep>>,
    pub gate: Option<Arc<Gate>>,
}

impl DeadlineWorkerStore for Worker {
    fn run_next(&self) -> Result<DeadlineWorkerRun, ApplicationError> {
        let _active = self.state.enter(Call::Worker);
        if let Some(gate) = &self.gate {
            gate.hold();
        }
        match self
            .script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(WorkerStep::Completed)
        {
            WorkerStep::Completed => Ok(values::completed()),
            WorkerStep::Deferred(inconsistent) => Ok(values::deferred(inconsistent)),
            WorkerStep::Idle => Ok(DeadlineWorkerRun::Idle),
            WorkerStep::Fail(error) => Err(error.error()),
        }
    }

    fn result(&self, _: Uuid) -> Result<Option<DeadlineWorkerResult>, ApplicationError> {
        panic!("scheduler must not query job results")
    }

    fn latest_attempt(&self, _: Uuid) -> Result<Option<DeadlineWorkerAttempt>, ApplicationError> {
        panic!("scheduler must not query job attempts")
    }
}

pub struct Harness {
    pub state: Arc<State>,
    pub dispatch: Dispatcher,
    pub worker: Worker,
}

impl Harness {
    pub fn new(waits: usize, dispatch: Vec<DispatchStep>, worker: Vec<WorkerStep>) -> Self {
        let state = Arc::new(State::new(waits));
        Self {
            state: Arc::clone(&state),
            dispatch: Dispatcher {
                state: Arc::clone(&state),
                script: Mutex::new(dispatch.into()),
                gate: None,
            },
            worker: Worker {
                state,
                script: Mutex::new(worker.into()),
                gate: None,
            },
        }
    }

    pub fn run(&self, limit: u32) -> Result<(), ApplicationError> {
        let config =
            DeadlineRuntimeConfig::new(DeadlineDispatchLimit::new(limit).unwrap(), POLL).unwrap();
        run(&self.dispatch, &self.worker, config, self.state.as_ref())
    }
}

pub fn cycle(stream: DeadlineDispatchStream, limit: u32, worker_calls: usize) -> Vec<Call> {
    let mut calls = vec![Call::Dispatch(stream, limit)];
    calls.extend(std::iter::repeat_n(Call::Worker, worker_calls));
    calls.push(Call::Wait(POLL));
    calls
}
