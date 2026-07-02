use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use futures::future::FutureExt;
use gpui::{BackgroundExecutor, Task};
use uuid::Uuid;

/// Unique identifier for a tracked task.
pub type TaskId = Uuid;

/// Metadata describing a tracked task.
#[derive(Clone, Debug)]
pub struct TaskMetadata {
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

/// The status of a tracked task.
#[derive(Debug)]
pub enum TaskStatus {
    /// The task is currently running.
    Running,
    /// The task has completed with the given result.
    Completed(std::result::Result<(), anyhow::Error>),
    /// The task was cancelled before completion.
    Cancelled,
}

impl PartialEq for TaskStatus {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Running, Self::Running) => true,
            (Self::Cancelled, Self::Cancelled) => true,
            (Self::Completed(Ok(())), Self::Completed(Ok(()))) => true,
            (Self::Completed(Err(_)), Self::Completed(Err(_))) => true,
            _ => false,
        }
    }
}

impl Clone for TaskStatus {
    fn clone(&self) -> Self {
        match self {
            Self::Running => Self::Running,
            Self::Cancelled => Self::Cancelled,
            Self::Completed(Ok(())) => Self::Completed(Ok(())),
            Self::Completed(Err(e)) => Self::Completed(Err(anyhow::anyhow!("{e}"))),
        }
    }
}

/// Information about a tracked task.
#[derive(Clone, Debug)]
pub struct TaskInfo {
    pub id: TaskId,
    pub metadata: TaskMetadata,
    pub status: TaskStatus,
    pub duration: Option<Duration>,
}

/// Tracks and manages spawned tasks, providing lifecycle management
/// (spawn, cancel, status) and post-completion bookkeeping.
///
/// Tasks are spawned on a background executor and tracked by their
/// [`TaskId`]. Completed tasks are retained for at least 5 seconds
/// before being evicted to allow callers to inspect results.
///
/// # Correctness
///
/// After a task completes, the manager records the final status for
/// at least 5 seconds before cleanup. This allows callers to poll for
/// results even after the underlying future has finished.
pub struct ExecutionManager {
    running_tasks: HashMap<TaskId, TrackedTask>,
    executor: BackgroundExecutor,
}

struct TrackedTask {
    metadata: TaskMetadata,
    status: Arc<Mutex<TaskStatus>>,
    started_at: Instant,
    completed_at: Option<Instant>,
    /// The spawned helper task that awaits the original `gpui::Task` and
    /// records its completion status. Kept alive so that dropping this
    /// handle cancels the underlying work.
    _helper: Option<Task<()>>,
}

impl ExecutionManager {
    /// Creates a new execution manager using the given background executor.
    pub fn new(executor: BackgroundExecutor) -> Self {
        Self {
            running_tasks: HashMap::new(),
            executor,
        }
    }

    /// Spawn a new task and track it.
    ///
    /// The task is immediately started on the background executor.
    /// Once it completes (or panics), its status is recorded and kept
    /// for at least 5 seconds before eviction.
    pub fn spawn_task(&mut self, task: Task<anyhow::Result<()>>, metadata: TaskMetadata) -> TaskId {
        let id = TaskId::new_v4();
        let status = Arc::new(Mutex::new(TaskStatus::Running));
        let status_clone = status.clone();

        // Spawn a helper on the background executor that awaits the
        // gpui::Task and records its completion status. We use
        // AssertUnwindSafe to catch panics from the inner task so that
        // panics are reported as errors rather than crashing the process.
        let helper = self.executor.spawn(async move {
            let result = std::panic::AssertUnwindSafe(task).catch_unwind().await;
            let status_value = match result {
                Ok(Ok(())) => TaskStatus::Completed(Ok(())),
                Ok(Err(err)) => TaskStatus::Completed(Err(err)),
                Err(panic_info) => {
                    let msg = panic_info
                        .downcast_ref::<&str>()
                        .copied()
                        .or_else(|| panic_info.downcast_ref::<String>().map(|s| s.as_str()))
                        .unwrap_or("task panicked");
                    TaskStatus::Completed(Err(anyhow::anyhow!("{msg}")))
                }
            };
            *status_clone.lock().unwrap() = status_value;
        });

        self.running_tasks.insert(
            id,
            TrackedTask {
                metadata,
                status,
                started_at: Instant::now(),
                completed_at: None,
                _helper: Some(helper),
            },
        );

        id
    }

    /// Cancel a running task by its ID.
    ///
    /// Dropping the task handle stops the task. The status is updated
    /// to [`TaskStatus::Cancelled`] only if the task was still running.
    /// If the task already completed, this is a no-op.
    pub fn cancel(&mut self, id: TaskId) -> Result<(), anyhow::Error> {
        let tracked = self
            .running_tasks
            .get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("task {id} not found"))?;

        // Drop the helper task handle to cancel the underlying work.
        tracked._helper.take();

        let mut guard = tracked.status.lock().unwrap();
        if matches!(*guard, TaskStatus::Running) {
            *guard = TaskStatus::Cancelled;
        }
        tracked.completed_at = Some(Instant::now());

        Ok(())
    }

    /// Returns the current status of a task.
    ///
    /// Returns `None` if the task ID is unknown or has been evicted.
    pub fn status(&self, id: TaskId) -> Option<TaskStatus> {
        let tracked = self.running_tasks.get(&id)?;
        let guard = tracked.status.lock().ok()?;
        Some(guard.clone())
    }

    /// Returns information about all active (non-evicted) tasks.
    pub fn list_active(&mut self) -> Vec<TaskInfo> {
        self.evict_completed();
        self.running_tasks
            .iter()
            .map(|(id, tracked)| {
                let status = tracked.status.lock().unwrap().clone();
                let duration = tracked
                    .completed_at
                    .map(|end| end.duration_since(tracked.started_at));
                TaskInfo {
                    id: *id,
                    metadata: tracked.metadata.clone(),
                    status,
                    duration,
                }
            })
            .collect()
    }

    /// Remove tasks that completed more than 5 seconds ago.
    fn evict_completed(&mut self) {
        let now = Instant::now();
        self.running_tasks
            .retain(|_, tracked| match tracked.completed_at {
                Some(t) if now.duration_since(t) >= Duration::from_secs(5) => false,
                _ => true,
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::oneshot;

    use gpui::TestDispatcher;

    fn test_metadata(name: &str) -> TaskMetadata {
        TaskMetadata {
            name: name.to_string(),
            description: String::new(),
            created_at: Utc::now(),
        }
    }

    /// Create a `BackgroundExecutor` backed by a `TestDispatcher`.
    fn test_executor() -> BackgroundExecutor {
        let dispatcher = Arc::new(TestDispatcher::new(0));
        BackgroundExecutor::new(dispatcher)
    }

    /// Create a `BackgroundExecutor` backed by a `TestDispatcher` and
    /// return its foreground executor for pumping the scheduler.
    fn test_executor_and_foreground() -> (BackgroundExecutor, scheduler::LocalExecutor) {
        let dispatcher = Arc::new(TestDispatcher::new(0));
        let executor = BackgroundExecutor::new(dispatcher.clone());
        let scheduler = dispatcher.scheduler().clone();
        let foreground = scheduler.foreground();
        (executor, foreground)
    }

    #[test]
    fn test_spawn_task_is_running() {
        let executor = test_executor();
        let mut manager = ExecutionManager::new(executor.clone());

        let task = executor.spawn::<anyhow::Result<()>>(async { Ok(()) });
        let id = manager.spawn_task(task, test_metadata("test"));

        let status = manager.status(id);
        assert_eq!(status, Some(TaskStatus::Running));
    }

    #[test]
    fn test_task_completes_successfully() {
        let (executor, foreground) = test_executor_and_foreground();
        let mut manager = ExecutionManager::new(executor.clone());

        let task = executor.spawn::<anyhow::Result<()>>(async { Ok(()) });
        let id = manager.spawn_task(task, test_metadata("test"));

        // Block on the foreground to pump the scheduler. This advances
        // the test scheduler's clock and runs any pending background
        // tasks to completion.
        foreground.block_on(async {
            foreground.timer(Duration::from_millis(1)).await;
        });

        let status = manager.status(id);
        assert_eq!(status, Some(TaskStatus::Completed(Ok(()))));
    }

    #[test]
    fn test_task_returns_error() {
        let (executor, foreground) = test_executor_and_foreground();
        let mut manager = ExecutionManager::new(executor.clone());

        let task = executor.spawn::<anyhow::Result<()>>(async { Err(anyhow::anyhow!("oops")) });
        let id = manager.spawn_task(task, test_metadata("failing"));

        foreground.block_on(async {
            foreground.timer(Duration::from_millis(1)).await;
        });

        let status = manager.status(id);
        match status {
            Some(TaskStatus::Completed(Err(e))) => {
                assert!(e.to_string().contains("oops"));
            }
            other => panic!("expected Completed(Err), got {other:?}"),
        }
    }

    #[test]
    fn test_cancel_running_task() {
        let executor = test_executor();
        let mut manager = ExecutionManager::new(executor.clone());

        // Create a task that doesn't complete immediately.
        let (tx, rx) = oneshot::channel::<()>();
        let task = executor.spawn::<anyhow::Result<()>>(async move {
            rx.await.ok();
            Ok(())
        });
        let id = manager.spawn_task(task, test_metadata("blocked"));

        // The task is still running (waiting on the oneshot).
        assert_eq!(manager.status(id), Some(TaskStatus::Running));

        // Cancel it.
        manager.cancel(id).unwrap();
        assert_eq!(manager.status(id), Some(TaskStatus::Cancelled));

        // Dropping the sender would cause the task to complete, but
        // since we already cancelled, the status stays Cancelled.
        drop(tx);
    }

    #[test]
    fn test_cancel_unknown_id_returns_error() {
        let executor = test_executor();
        let mut manager = ExecutionManager::new(executor);
        assert!(manager.cancel(TaskId::new_v4()).is_err());
    }

    #[test]
    fn test_status_unknown_id() {
        let executor = test_executor();
        let manager = ExecutionManager::new(executor);
        assert_eq!(manager.status(TaskId::new_v4()), None);
    }

    #[test]
    fn test_list_active_includes_all() {
        let executor = test_executor();
        let mut manager = ExecutionManager::new(executor.clone());

        let task1 = executor.spawn::<anyhow::Result<()>>(async { Ok(()) });
        let task2 = executor.spawn::<anyhow::Result<()>>(async { Ok(()) });
        let _id1 = manager.spawn_task(task1, test_metadata("a"));
        let _id2 = manager.spawn_task(task2, test_metadata("b"));

        let active = manager.list_active();
        assert_eq!(active.len(), 2);

        let names: Vec<&str> = active
            .iter()
            .map(|info| info.metadata.name.as_str())
            .collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }
}
