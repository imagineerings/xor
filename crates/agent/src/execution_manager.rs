use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};

/// Unique identifier for a tracked task.
pub type TaskId = u64;

/// Metadata describing a tracked task.
#[derive(Clone, Debug)]
pub struct TaskMetadata {
    /// Human-readable name (e.g. "code-execution", "recipe-run").
    pub name: String,
    /// Longer description of what the task does.
    pub description: String,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
}

impl TaskMetadata {
    /// Create new task metadata with the current timestamp.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            created_at: Utc::now(),
        }
    }
}

/// The current status of a tracked task.
#[derive(Clone, Debug)]
pub enum TaskStatus {
    /// The task is still running.
    Running,
    /// The task finished with a result (Ok or Err).
    Completed(Arc<Result<()>>),
    /// The task was cancelled before completion.
    Cancelled,
}

impl TaskStatus {
    /// Returns `true` if the task is still running.
    pub fn is_running(&self) -> bool {
        matches!(self, TaskStatus::Running)
    }

    /// Returns `true` if the task completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, TaskStatus::Completed(r) if r.is_ok())
    }

    /// Returns `true` if the task completed with an error.
    pub fn is_error(&self) -> bool {
        matches!(self, TaskStatus::Completed(r) if r.is_err())
    }

    /// Returns `true` if the task was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, TaskStatus::Cancelled)
    }
}

/// A task being tracked by the execution manager.
#[derive(Clone, Debug)]
pub struct TrackedTask {
    /// Unique identifier assigned at registration time.
    pub id: TaskId,
    /// Metadata provided when the task was registered.
    pub metadata: TaskMetadata,
    /// Current status of the task.
    pub status: TaskStatus,
    /// When the task was registered (started).
    pub started_at: DateTime<Utc>,
    /// When the task reached a terminal state (completed or cancelled).
    pub completed_at: Option<DateTime<Utc>>,
}

/// Summary information about a task, suitable for listing.
#[derive(Clone, Debug)]
pub struct TaskInfo {
    /// Unique task identifier.
    pub id: TaskId,
    /// Metadata provided at registration.
    pub metadata: TaskMetadata,
    /// Current status.
    pub status: TaskStatus,
    /// How long the task ran (if it reached a terminal state).
    pub duration: Option<Duration>,
}

/// Manages the lifecycle and tracking of running tasks.
///
/// The execution manager records when tasks start, tracks their status,
/// and provides visibility into what the agent is currently doing.
/// It does **not** execute the tasks itself — callers are responsible
/// for running futures and reporting completion via [`complete`](Self::complete).
#[derive(Clone, Debug, Default)]
pub struct ExecutionManager {
    tasks: HashMap<TaskId, TrackedTask>,
    next_id: TaskId,
}

impl ExecutionManager {
    /// Create a new empty execution manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new task and return its assigned [`TaskId`].
    ///
    /// The task is recorded as [`TaskStatus::Running`]. Call
    /// [`complete`](Self::complete) when the task finishes, or
    /// [`cancel`](Self::cancel) to mark it as cancelled.
    pub fn register(&mut self, metadata: TaskMetadata) -> TaskId {
        self.next_id += 1;
        let id = self.next_id;

        self.tasks.insert(
            id,
            TrackedTask {
                id,
                metadata,
                status: TaskStatus::Running,
                started_at: Utc::now(),
                completed_at: None,
            },
        );

        id
    }

    /// Mark a running task as completed with the given result.
    ///
    /// Returns `true` if the task was found and was in a running state.
    /// Returns `false` if the task was already in a terminal state or
    /// does not exist.
    pub fn complete(&mut self, id: TaskId, result: Result<()>) -> bool {
        let Some(task) = self.tasks.get_mut(&id) else {
            return false;
        };
        if !task.status.is_running() {
            return false;
        }
        task.status = TaskStatus::Completed(Arc::new(result));
        task.completed_at = Some(Utc::now());
        true
    }

    /// Cancel a running task.
    ///
    /// Returns `true` if the task was found and was running.
    pub fn cancel(&mut self, id: TaskId) -> bool {
        let Some(task) = self.tasks.get_mut(&id) else {
            return false;
        };
        if !task.status.is_running() {
            return false;
        }
        task.status = TaskStatus::Cancelled;
        task.completed_at = Some(Utc::now());
        true
    }

    /// Get the current status of a task.
    pub fn status(&self, id: TaskId) -> Option<&TaskStatus> {
        self.tasks.get(&id).map(|t| &t.status)
    }

    /// Get a tracked task by its identifier.
    pub fn get(&self, id: TaskId) -> Option<&TrackedTask> {
        self.tasks.get(&id)
    }

    /// List all currently active (running) tasks with summary info.
    pub fn list_active(&self) -> Vec<TaskInfo> {
        self.tasks
            .values()
            .filter(|t| t.status.is_running())
            .map(task_to_info)
            .collect()
    }

    /// List all tracked tasks (running, completed, and cancelled).
    pub fn list_all(&self) -> Vec<TaskInfo> {
        self.tasks.values().map(task_to_info).collect()
    }

    /// Remove a task from tracking entirely.
    ///
    /// Returns the removed task, or `None` if it did not exist.
    pub fn remove(&mut self, id: TaskId) -> Option<TrackedTask> {
        self.tasks.remove(&id)
    }

    /// Returns the number of currently running tasks.
    pub fn active_count(&self) -> usize {
        self.tasks
            .values()
            .filter(|t| t.status.is_running())
            .count()
    }

    /// Returns the total number of tracked tasks (including completed).
    pub fn total_count(&self) -> usize {
        self.tasks.len()
    }

    /// Returns `true` if there are no running tasks.
    pub fn is_idle(&self) -> bool {
        self.active_count() == 0
    }
}

fn task_to_info(task: &TrackedTask) -> TaskInfo {
    let duration = task
        .completed_at
        .and_then(|end| (end - task.started_at).to_std().ok());
    TaskInfo {
        id: task.id,
        metadata: task.metadata.clone(),
        status: task.status.clone(),
        duration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_status() {
        let mut manager = ExecutionManager::new();
        let id = manager.register(TaskMetadata::new("test", "A test task"));
        assert_eq!(manager.active_count(), 1);
        assert_eq!(manager.total_count(), 1);
        assert!(manager.status(id).unwrap().is_running());
    }

    #[test]
    fn test_complete_with_success() {
        let mut manager = ExecutionManager::new();
        let id = manager.register(TaskMetadata::new("op", "An operation"));
        assert!(manager.complete(id, Ok(())));
        assert!(manager.status(id).unwrap().is_success());
        assert!(!manager.status(id).unwrap().is_running());
        assert!(manager.get(id).unwrap().completed_at.is_some());
    }

    #[test]
    fn test_complete_with_error() {
        let mut manager = ExecutionManager::new();
        let id = manager.register(TaskMetadata::new("op", "Failing operation"));
        assert!(manager.complete(id, Err(anyhow::anyhow!("something went wrong"))));
        assert!(manager.status(id).unwrap().is_error());
    }

    #[test]
    fn test_cancel() {
        let mut manager = ExecutionManager::new();
        let id = manager.register(TaskMetadata::new("op", "Cancel me"));
        assert!(manager.cancel(id));
        assert!(manager.status(id).unwrap().is_cancelled());
        // Cancelling again should fail
        assert!(!manager.cancel(id));
    }

    #[test]
    fn test_list_active() {
        let mut manager = ExecutionManager::new();
        let id1 = manager.register(TaskMetadata::new("a", "Task A"));
        let id2 = manager.register(TaskMetadata::new("b", "Task B"));
        manager.complete(id1, Ok(()));

        let active = manager.list_active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, id2);
    }

    #[test]
    fn test_list_all() {
        let mut manager = ExecutionManager::new();
        let id1 = manager.register(TaskMetadata::new("a", "Task A"));
        let id2 = manager.register(TaskMetadata::new("b", "Task B"));
        manager.complete(id1, Ok(()));
        manager.cancel(id2);

        let all = manager.list_all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut manager = ExecutionManager::new();
        let id = manager.register(TaskMetadata::new("temp", "Temporary"));
        let removed = manager.remove(id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, id);
        assert!(manager.get(id).is_none());
        assert_eq!(manager.total_count(), 0);
    }

    #[test]
    fn test_is_idle() {
        let mut manager = ExecutionManager::new();
        assert!(manager.is_idle());
        let id = manager.register(TaskMetadata::new("op", "An op"));
        assert!(!manager.is_idle());
        manager.complete(id, Ok(()));
        assert!(manager.is_idle());
    }

    #[test]
    fn test_complete_nonexistent() {
        let mut manager = ExecutionManager::new();
        assert!(!manager.complete(999, Ok(())));
    }

    #[test]
    fn test_cancel_already_completed() {
        let mut manager = ExecutionManager::new();
        let id = manager.register(TaskMetadata::new("op", "Quick op"));
        manager.complete(id, Ok(())).then_some(());
        assert!(!manager.cancel(id));
    }

    #[test]
    fn test_task_info_duration() {
        let mut manager = ExecutionManager::new();
        let id = manager.register(TaskMetadata::new("op", "Duration test"));
        std::thread::sleep(std::time::Duration::from_millis(10));
        manager.complete(id, Ok(())).then_some(());

        let info = manager.list_all();
        let task_info = info.iter().find(|t| t.id == id).unwrap();
        assert!(task_info.duration.is_some());
        assert!(task_info.duration.unwrap().as_millis() >= 5);
    }
}
