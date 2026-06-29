use anyhow::Result;
use collections::HashMap;
use std::sync::Arc;

pub type SubprocessId = u64;

#[derive(Clone, Default)]
pub struct SubprocessManager {
    state: Arc<parking_lot::Mutex<SubprocessManagerState>>,
}

#[derive(Default)]
struct SubprocessManagerState {
    next_id: SubprocessId,
    children: HashMap<SubprocessId, crate::process::Child>,
}

impl SubprocessManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, child: crate::process::Child) -> SubprocessId {
        let mut state = self.state.lock();
        state.next_id = state.next_id.saturating_add(1);
        let id = state.next_id;
        state.children.insert(id, child);
        id
    }

    pub fn remove(&self, id: SubprocessId) -> Option<crate::process::Child> {
        self.state.lock().children.remove(&id)
    }

    pub fn contains(&self, id: SubprocessId) -> bool {
        self.state.lock().children.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.state.lock().children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn kill(&self, id: SubprocessId) -> Result<bool> {
        let Some(mut child) = self.remove(id) else {
            return Ok(false);
        };
        child.kill()?;
        Ok(true)
    }

    pub fn kill_all(&self) -> Vec<(SubprocessId, anyhow::Error)> {
        let children = {
            let mut state = self.state.lock();
            std::mem::take(&mut state.children)
        };

        let mut errors = Vec::new();
        for (id, mut child) in children {
            if let Err(error) = child.kill() {
                errors.push((id, error));
            }
        }
        errors
    }
}

impl Drop for SubprocessManager {
    fn drop(&mut self) {
        if Arc::strong_count(&self.state) == 1 {
            for (id, error) in self.kill_all() {
                log::warn!("failed to kill subprocess {id}: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[cfg(not(windows))]
    fn sleeping_child() -> crate::process::Child {
        let mut command = std::process::Command::new("/bin/sh");
        command.args(["-c", "sleep 30"]);
        crate::process::Child::spawn(command, Stdio::null(), Stdio::null(), Stdio::null()).unwrap()
    }

    #[cfg(not(windows))]
    #[test]
    fn test_subprocess_manager_tracks_and_kills_child() {
        let manager = SubprocessManager::new();
        let id = manager.insert(sleeping_child());

        assert!(manager.contains(id));
        assert_eq!(manager.len(), 1);
        assert!(manager.kill(id).unwrap());
        assert!(manager.is_empty());
        assert!(!manager.kill(id).unwrap());
    }

    #[test]
    fn test_subprocess_manager_kill_all_handles_empty_manager() {
        let manager = SubprocessManager::new();

        assert!(manager.kill_all().is_empty());
    }
}
