use anyhow::{Context as _, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use uuid::Uuid;

const LIST_TUTORIALS: &str = "list_tutorials";
const START_TUTORIAL: &str = "start_tutorial";
const CURRENT_STEP: &str = "current_step";
const COMPLETE_STEP: &str = "complete_step";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Tutorial {
    pub id: String,
    pub title: String,
    pub path: PathBuf,
    pub steps: Vec<TutorialStep>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TutorialStep {
    pub index: usize,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TutorialSummary {
    pub id: String,
    pub title: String,
    pub step_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TutorialProgress {
    pub session_id: String,
    pub tutorial_id: String,
    pub current_step: usize,
    pub completed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Deserialize)]
pub struct StartTutorialRequest {
    pub tutorial_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionRequest {
    pub session_id: String,
}

pub struct TutorialCatalog {
    tutorials: HashMap<String, Tutorial>,
}

impl TutorialCatalog {
    pub fn load_from_dir(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let mut tutorials = HashMap::new();
        for path in markdown_files(root)? {
            let tutorial = Tutorial::load(root, &path)?;
            if tutorials.insert(tutorial.id.clone(), tutorial).is_some() {
                bail!("duplicate tutorial id for {}", path.display());
            }
        }

        Ok(Self { tutorials })
    }

    pub fn from_tutorials(tutorials: Vec<Tutorial>) -> Result<Self> {
        let mut by_id = HashMap::new();
        for tutorial in tutorials {
            if tutorial.steps.is_empty() {
                bail!("tutorial `{}` must contain at least one step", tutorial.id);
            }
            if by_id.insert(tutorial.id.clone(), tutorial).is_some() {
                bail!("duplicate tutorial id");
            }
        }
        Ok(Self { tutorials: by_id })
    }

    pub fn list(&self) -> Vec<TutorialSummary> {
        let mut summaries = self
            .tutorials
            .values()
            .map(|tutorial| TutorialSummary {
                id: tutorial.id.clone(),
                title: tutorial.title.clone(),
                step_count: tutorial.steps.len(),
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| left.id.cmp(&right.id));
        summaries
    }

    pub fn get(&self, tutorial_id: &str) -> Option<&Tutorial> {
        self.tutorials.get(tutorial_id)
    }
}

impl Tutorial {
    pub fn load(root: &Path, path: &Path) -> Result<Self> {
        let markdown = fs::read_to_string(path)
            .with_context(|| format!("reading tutorial {}", path.display()))?;
        let id = tutorial_id(root, path)?;
        let (title, steps) = parse_tutorial_markdown(&id, &markdown)?;
        Ok(Self {
            id,
            title,
            path: path.to_path_buf(),
            steps,
        })
    }
}

pub struct TutorialServer {
    catalog: TutorialCatalog,
    sessions: Mutex<HashMap<String, TutorialProgress>>,
}

impl TutorialServer {
    pub fn new(catalog: TutorialCatalog) -> Self {
        Self {
            catalog,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn capabilities(&self) -> Value {
        json!({
            "tools": {
                "listChanged": false
            }
        })
    }

    pub fn tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor {
                name: LIST_TUTORIALS.to_string(),
                description: "List available markdown tutorials.".to_string(),
                input_schema: empty_schema(),
            },
            ToolDescriptor {
                name: START_TUTORIAL.to_string(),
                description: "Start an interactive tutorial session.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "tutorial_id": { "type": "string" }
                    },
                    "required": ["tutorial_id"]
                }),
            },
            ToolDescriptor {
                name: CURRENT_STEP.to_string(),
                description: "Read the current step for a tutorial session.".to_string(),
                input_schema: session_schema(),
            },
            ToolDescriptor {
                name: COMPLETE_STEP.to_string(),
                description:
                    "Mark the current tutorial step complete and advance to the next step."
                        .to_string(),
                input_schema: session_schema(),
            },
        ]
    }

    pub fn handle_tool_call(&self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            LIST_TUTORIALS => Ok(json!({ "tutorials": self.catalog.list() })),
            START_TUTORIAL => {
                let request: StartTutorialRequest = serde_json::from_value(arguments)
                    .context("parsing start_tutorial arguments")?;
                let progress = self.start_tutorial(&request.tutorial_id)?;
                let step = self.step_for_progress(&progress)?;
                Ok(json!({ "progress": progress, "step": step }))
            }
            CURRENT_STEP => {
                let request: SessionRequest =
                    serde_json::from_value(arguments).context("parsing current_step arguments")?;
                let progress = self.progress(&request.session_id)?;
                let step = self.step_for_progress(&progress)?;
                Ok(json!({ "progress": progress, "step": step }))
            }
            COMPLETE_STEP => {
                let request: SessionRequest =
                    serde_json::from_value(arguments).context("parsing complete_step arguments")?;
                let progress = self.complete_step(&request.session_id)?;
                let step = if progress.completed {
                    None
                } else {
                    Some(self.step_for_progress(&progress)?)
                };
                Ok(json!({ "progress": progress, "step": step }))
            }
            _ => bail!("unknown tutorial tool `{name}`"),
        }
    }

    pub fn start_tutorial(&self, tutorial_id: &str) -> Result<TutorialProgress> {
        self.catalog
            .get(tutorial_id)
            .ok_or_else(|| anyhow!("unknown tutorial `{tutorial_id}`"))?;
        let progress = TutorialProgress {
            session_id: Uuid::new_v4().to_string(),
            tutorial_id: tutorial_id.to_string(),
            current_step: 0,
            completed: false,
        };
        self.lock_sessions()?
            .insert(progress.session_id.clone(), progress.clone());
        Ok(progress)
    }

    pub fn progress(&self, session_id: &str) -> Result<TutorialProgress> {
        self.lock_sessions()?
            .get(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown tutorial session `{session_id}`"))
    }

    pub fn complete_step(&self, session_id: &str) -> Result<TutorialProgress> {
        let mut sessions = self.lock_sessions()?;
        let progress = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("unknown tutorial session `{session_id}`"))?;
        if progress.completed {
            return Ok(progress.clone());
        }

        let tutorial = self
            .catalog
            .get(&progress.tutorial_id)
            .ok_or_else(|| anyhow!("unknown tutorial `{}`", progress.tutorial_id))?;
        if progress.current_step + 1 >= tutorial.steps.len() {
            progress.completed = true;
        } else {
            progress.current_step += 1;
        }
        Ok(progress.clone())
    }

    fn step_for_progress(&self, progress: &TutorialProgress) -> Result<TutorialStep> {
        let tutorial = self
            .catalog
            .get(&progress.tutorial_id)
            .ok_or_else(|| anyhow!("unknown tutorial `{}`", progress.tutorial_id))?;
        tutorial
            .steps
            .get(progress.current_step)
            .cloned()
            .ok_or_else(|| anyhow!("tutorial `{}` has no current step", progress.tutorial_id))
    }

    fn lock_sessions(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<String, TutorialProgress>>> {
        self.sessions
            .lock()
            .map_err(|_| anyhow!("tutorial session lock poisoned"))
    }
}

fn markdown_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_markdown_files(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_markdown_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(path).with_context(|| format!("reading {}", path.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
    Ok(())
}

fn tutorial_id(root: &Path, path: &Path) -> Result<String> {
    let relative_path = path.strip_prefix(root).unwrap_or(path);
    let path_without_extension = relative_path.with_extension("");
    let mut id = String::new();
    for component in path_without_extension.components() {
        if !id.is_empty() {
            id.push('/');
        }
        let component = component.as_os_str().to_string_lossy();
        id.push_str(&component);
    }
    if id.is_empty() {
        bail!("tutorial path {} produced an empty id", path.display());
    }
    Ok(id)
}

fn parse_tutorial_markdown(id: &str, markdown: &str) -> Result<(String, Vec<TutorialStep>)> {
    let mut title = None;
    let mut steps = Vec::new();
    let mut current_title = None;
    let mut current_body = Vec::new();

    for line in markdown.lines() {
        if let Some(heading) = line.strip_prefix("# ") {
            if title.is_none() {
                title = Some(heading.trim().to_string());
                continue;
            }
        }

        if let Some(heading) = line.strip_prefix("## ") {
            push_step(&mut steps, current_title.take(), &mut current_body);
            current_title = Some(heading.trim().to_string());
        } else {
            current_body.push(line.to_string());
        }
    }
    push_step(&mut steps, current_title.take(), &mut current_body);

    let title = title
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| id.to_string());
    if steps.is_empty() {
        bail!("tutorial `{id}` does not contain any content");
    }

    Ok((title, steps))
}

fn push_step(steps: &mut Vec<TutorialStep>, title: Option<String>, body: &mut Vec<String>) {
    let body_text = body.join("\n").trim().to_string();
    body.clear();
    if title.is_none() && body_text.is_empty() {
        return;
    }

    let title = title.unwrap_or_else(|| "Introduction".to_string());
    steps.push(TutorialStep {
        index: steps.len(),
        title,
        body: body_text,
    });
}

fn empty_schema() -> Value {
    json!({
        "type": "object",
        "properties": {}
    })
}

fn session_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "session_id": { "type": "string" }
        },
        "required": ["session_id"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_tutorial(id: &str) -> Tutorial {
        Tutorial {
            id: id.to_string(),
            title: "Sample".to_string(),
            path: PathBuf::from(format!("{id}.md")),
            steps: vec![
                TutorialStep {
                    index: 0,
                    title: "One".to_string(),
                    body: "First step".to_string(),
                },
                TutorialStep {
                    index: 1,
                    title: "Two".to_string(),
                    body: "Second step".to_string(),
                },
            ],
        }
    }

    #[test]
    fn loads_tutorials_from_markdown_files() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let tutorial_path = temp_dir.path().join("getting-started.md");
        fs::write(
            &tutorial_path,
            "# Getting Started\n\nWelcome.\n\n## Install\nRun setup.\n\n## Use\nStart the app.\n",
        )
        .expect("write tutorial");

        let catalog = TutorialCatalog::load_from_dir(temp_dir.path()).expect("load catalog");
        let tutorial = catalog.get("getting-started").expect("get tutorial");

        assert_eq!(tutorial.title, "Getting Started");
        assert_eq!(tutorial.steps.len(), 3);
        assert_eq!(tutorial.steps[0].title, "Introduction");
        assert_eq!(tutorial.steps[1].title, "Install");
        assert_eq!(tutorial.steps[2].body, "Start the app.");
    }

    #[test]
    fn advances_tutorial_session_until_complete() {
        let catalog = TutorialCatalog::from_tutorials(vec![sample_tutorial("sample")])
            .expect("create catalog");
        let server = TutorialServer::new(catalog);

        let progress = server.start_tutorial("sample").expect("start tutorial");
        assert_eq!(progress.current_step, 0);
        assert!(!progress.completed);

        let progress = server
            .complete_step(&progress.session_id)
            .expect("complete first step");
        assert_eq!(progress.current_step, 1);
        assert!(!progress.completed);

        let progress = server
            .complete_step(&progress.session_id)
            .expect("complete final step");
        assert_eq!(progress.current_step, 1);
        assert!(progress.completed);
    }

    #[test]
    fn handles_tool_calls() {
        let catalog = TutorialCatalog::from_tutorials(vec![sample_tutorial("sample")])
            .expect("create catalog");
        let server = TutorialServer::new(catalog);

        let list = server
            .handle_tool_call(LIST_TUTORIALS, json!({}))
            .expect("list tutorials");
        assert_eq!(list["tutorials"][0]["id"], "sample");

        let started = server
            .handle_tool_call(START_TUTORIAL, json!({ "tutorial_id": "sample" }))
            .expect("start tutorial");
        let session_id = started["progress"]["session_id"]
            .as_str()
            .expect("session id");

        let next = server
            .handle_tool_call(COMPLETE_STEP, json!({ "session_id": session_id }))
            .expect("complete step");
        assert_eq!(next["step"]["title"], "Two");
    }

    #[test]
    fn rejects_unknown_tutorial() {
        let catalog = TutorialCatalog::from_tutorials(vec![sample_tutorial("sample")])
            .expect("create catalog");
        let server = TutorialServer::new(catalog);

        let error = server
            .start_tutorial("missing")
            .expect_err("missing tutorial should fail");

        assert!(error.to_string().contains("unknown tutorial"));
    }
}
