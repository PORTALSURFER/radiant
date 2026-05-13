use radiant::prelude::StatusSegments;

const LOG_LIMIT: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StatusMessage {
    ActionPressed,
    AutosaveChanged(bool),
    StartWorker,
    StartBatch,
    WorkerFinished(u64),
    Frame,
    ToggleAnimation,
    Reset,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct StatusBarState {
    pub(super) autosave: bool,
    pub(super) action_count: u32,
    pub(super) jobs: Vec<WorkItem>,
    pub(super) completed_workers: u32,
    pub(super) next_worker_id: u64,
    pub(super) frame: u64,
    pub(super) animation_running: bool,
    pub(super) log: StatusLineLog,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct WorkItem {
    id: u64,
    label: String,
    progress: f32,
    done: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StatusLineLog {
    pub(super) entries: Vec<StatusLineEntry>,
    limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StatusLineEntry {
    source: String,
    message: String,
}

impl StatusLineLog {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            entries: vec![StatusLineEntry::new("system", "Ready")],
            limit,
        }
    }

    pub(super) fn publish(&mut self, source: impl Into<String>, message: impl Into<String>) {
        self.entries
            .push(StatusLineEntry::new(source.into(), message.into()));
        let overflow = self.entries.len().saturating_sub(self.limit);
        if overflow > 0 {
            self.entries.drain(0..overflow);
        }
    }

    pub(super) fn latest(&self) -> String {
        self.entries
            .last()
            .map(StatusLineEntry::line)
            .unwrap_or_else(|| "system: Ready".to_string())
    }

    pub(super) fn recent_lines(&self) -> Vec<String> {
        self.entries
            .iter()
            .rev()
            .map(StatusLineEntry::line)
            .collect()
    }
}

impl StatusLineEntry {
    fn new(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            message: message.into(),
        }
    }

    fn line(&self) -> String {
        format!("{}: {}", self.source, self.message)
    }
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            autosave: true,
            action_count: 0,
            jobs: Vec::new(),
            completed_workers: 0,
            next_worker_id: 1,
            frame: 0,
            animation_running: false,
            log: StatusLineLog::new(LOG_LIMIT),
        }
    }
}

impl StatusBarState {
    pub(super) fn record_action(&mut self) {
        self.action_count += 1;
        self.log.publish(
            "action",
            format!("button pressed {} time(s)", self.action_count),
        );
    }

    pub(super) fn set_autosave(&mut self, enabled: bool) {
        self.autosave = enabled;
        let message = if enabled {
            "autosave enabled"
        } else {
            "autosave paused"
        };
        self.log.publish("autosave", message);
    }

    pub(super) fn start_worker(&mut self, label: impl Into<String>) -> u64 {
        let id = self.next_worker_id;
        self.next_worker_id += 1;
        let label = label.into();
        self.jobs.push(WorkItem {
            id,
            label: label.clone(),
            progress: 0.0,
            done: false,
        });
        self.log.publish("worker", format!("{label} started"));
        id
    }

    pub(super) fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        for job in self.jobs.iter_mut().filter(|job| !job.done) {
            job.progress = (job.progress + 0.018).min(0.92);
        }
    }

    pub(super) fn finish_worker(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) {
            job.progress = 1.0;
            job.done = true;
            self.completed_workers += 1;
            self.log
                .publish("worker", format!("{} finished", job.label));
        }
    }

    pub(super) fn active_count(&self) -> usize {
        self.jobs.iter().filter(|job| !job.done).count()
    }

    pub(super) fn aggregate_progress(&self) -> f32 {
        if self.jobs.is_empty() {
            return 0.0;
        }
        self.jobs.iter().map(|job| job.progress).sum::<f32>() / self.jobs.len() as f32
    }

    pub(super) fn visual_revision(&self) -> u64 {
        let progress = (self.aggregate_progress() * 10_000.0).round() as u64;
        (self.frame << 16) ^ progress ^ (self.jobs.len() as u64)
    }

    pub(super) fn toggle_animation(&mut self) {
        self.animation_running = !self.animation_running;
        self.log.publish(
            "animation",
            if self.animation_running {
                "started"
            } else {
                "stopped"
            },
        );
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
        self.log.publish("system", "status reset");
    }

    pub(super) fn status_segments(&self) -> StatusSegments {
        StatusSegments::primary(self.log.latest())
            .with_center(format!(
                "Autosave {} | actions {} | workers {}",
                if self.autosave { "on" } else { "off" },
                self.action_count,
                self.completed_workers
            ))
            .with_right(if self.active_count() > 0 {
                "Busy"
            } else if self.animation_running {
                "Animating"
            } else {
                "Idle"
            })
    }
}
