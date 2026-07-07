#!/usr/bin/env node

const crypto = require("node:crypto");
const childProcess = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const net = require("node:net");
const os = require("node:os");
const path = require("node:path");
const { pathToFileURL } = require("node:url");

const DEFAULT_LINEAR_ENDPOINT = "https://api.linear.app/graphql";

const DEFAULT_SETTINGS = {
  repository_path: "",
  workflow_path: "WORKFLOW.md",
  tasks_glob: ".agents/specs/**/tasks.md",
  workflow_state_path: ".agents/workflow-state.json",
  linear_endpoint: DEFAULT_LINEAR_ENDPOINT,
  linear_api_key: "$LINEAR_API_KEY",
  linear_team_id: "",
  linear_team_key: "",
  linear_project_id: "",
  linear_project_slug: "",
  linear_label_ids: [],
  linear_state_id: "",
  active_states: ["Todo", "In Progress"],
  terminal_states: ["Done", "Closed", "Cancelled", "Canceled", "Duplicate"],
  resume_existing: true,
};

function parseSettings() {
  const raw = process.env.WORKFLOW_SETTINGS || "{}";
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    throw new Error(`WORKFLOW_SETTINGS is not valid JSON: ${error.message}`);
  }

  const base = { ...DEFAULT_SETTINGS, ...parsed };
  return {
    ...DEFAULT_SETTINGS,
    ...workflowSettings(base),
    ...parsed,
  };
}

function workflowSettings(settings) {
  try {
    const workflow = parseWorkflow(workflowPath(settings));
    const tracker = workflow.config.tracker || {};
    const tasks = workflow.config.tasks || {};
    const mapped = {};

    if (tracker.endpoint) mapped.linear_endpoint = tracker.endpoint;
    if (tracker.api_key) mapped.linear_api_key = tracker.api_key;
    if (tracker.team_id) mapped.linear_team_id = tracker.team_id;
    if (tracker.team_key) mapped.linear_team_key = tracker.team_key;
    if (tracker.project_id) mapped.linear_project_id = tracker.project_id;
    if (tracker.project_slug) mapped.linear_project_slug = tracker.project_slug;
    if (tracker.state_id) mapped.linear_state_id = tracker.state_id;
    if (Array.isArray(tracker.label_ids)) mapped.linear_label_ids = tracker.label_ids;
    if (Array.isArray(tracker.active_states)) mapped.active_states = tracker.active_states;
    if (Array.isArray(tracker.terminal_states)) mapped.terminal_states = tracker.terminal_states;
    if (typeof tracker.resume_existing === "boolean") mapped.resume_existing = tracker.resume_existing;
    if (tasks.glob) mapped.tasks_glob = tasks.glob;
    if (workflow.config.workflow_state?.path) mapped.workflow_state_path = workflow.config.workflow_state.path;

    return mapped;
  } catch (_error) {
    return {};
  }
}

function repositoryRoot(settings) {
  const configured = settings.repository_path || process.cwd();
  if (configured.startsWith("~")) {
    return path.resolve(os.homedir(), configured.slice(1));
  }
  return path.resolve(configured);
}

function workflowPath(settings, override) {
  const configured = override || settings.workflow_path || "WORKFLOW.md";
  if (path.isAbsolute(configured)) return configured;
  return path.resolve(repositoryRoot(settings), configured);
}

function workflowStatePath(settings, override) {
  const configured = override || settings.workflow_state_path || DEFAULT_SETTINGS.workflow_state_path;
  if (path.isAbsolute(configured)) return configured;
  return path.resolve(repositoryRoot(settings), configured);
}

function resolveEnvReference(value, name) {
  if (typeof value !== "string") return value;
  if (!value.startsWith("$")) return value;
  const envName = value.slice(1);
  const resolved = process.env[envName] || "";
  if (!resolved) {
    throw new Error(`${name} references ${value}, but that environment variable is empty`);
  }
  return resolved;
}

function parseWorkflow(filePath) {
  let text;
  try {
    text = fs.readFileSync(filePath, "utf8");
  } catch (error) {
    const err = new Error(`missing_workflow_file: ${filePath}`);
    err.code = "missing_workflow_file";
    throw err;
  }

  let config = {};
  let body = text;
  if (text.startsWith("---\n") || text.startsWith("---\r\n")) {
    const newline = text.startsWith("---\r\n") ? "\r\n" : "\n";
    const marker = `${newline}---${newline}`;
    const end = text.indexOf(marker, 3);
    if (end === -1) {
      const err = new Error("workflow_parse_error: unterminated YAML front matter");
      err.code = "workflow_parse_error";
      throw err;
    }
    config = parseSimpleYamlMap(text.slice(3 + newline.length, end));
    body = text.slice(end + marker.length);
  }

  if (!config || Array.isArray(config) || typeof config !== "object") {
    const err = new Error("workflow_front_matter_not_a_map");
    err.code = "workflow_front_matter_not_a_map";
    throw err;
  }

  return {
    config,
    prompt_template: body.trim(),
    path: filePath,
  };
}

function parseSimpleYamlMap(yaml) {
  const lines = yaml
    .split(/\r?\n/)
    .map((raw) => ({ raw, line: raw.replace(/\s+#.*$/, "") }))
    .filter(({ line }) => line.trim());
  let index = 0;

  function indentation(line) {
    return line.match(/^ */)[0].length;
  }

  function parseNode(expectedIndent) {
    const current = lines[index]?.line || "";
    if (current.trim().startsWith("- ")) {
      return parseList(expectedIndent);
    }
    return parseMap(expectedIndent);
  }

  function parseList(expectedIndent) {
    const values = [];
    while (index < lines.length) {
      const { line } = lines[index];
      const indent = indentation(line);
      const trimmed = line.trim();
      if (indent < expectedIndent || !trimmed.startsWith("- ")) break;
      if (indent !== expectedIndent) {
        throw new Error(`workflow_parse_error: unsupported YAML indentation: ${trimmed}`);
      }
      values.push(parseYamlScalar(trimmed.slice(2).trim()));
      index += 1;
    }
    return values;
  }

  function parseMap(expectedIndent) {
    const value = {};
    while (index < lines.length) {
      const { line } = lines[index];
      const indent = indentation(line);
      const trimmed = line.trim();
      if (indent < expectedIndent || trimmed.startsWith("- ")) break;
      if (indent !== expectedIndent) {
        throw new Error(`workflow_parse_error: unsupported YAML indentation: ${trimmed}`);
      }
      const match = trimmed.match(/^([^:]+):(.*)$/);
      if (!match) {
        throw new Error(`workflow_parse_error: unsupported YAML line: ${trimmed}`);
      }
      const key = match[1].trim();
      const valueText = match[2].trim();
      index += 1;
      if (valueText === "|" || valueText === "|-") {
        value[key] = parseBlockScalar(indent);
      } else if (!valueText) {
        if (index >= lines.length || indentation(lines[index].line) <= indent) {
          value[key] = {};
        } else {
          value[key] = parseNode(indentation(lines[index].line));
        }
      } else {
        value[key] = parseYamlScalar(valueText);
      }
    }
    return value;
  }

  function parseBlockScalar(parentIndent) {
    const blockIndent = parentIndent + 2;
    const blockLines = [];
    while (index < lines.length) {
      const raw = lines[index].raw;
      const indent = indentation(raw);
      if (indent <= parentIndent) break;
      blockLines.push(raw.length >= blockIndent ? raw.slice(blockIndent) : raw.trimStart());
      index += 1;
    }
    return blockLines.join("\n");
  }

  return parseNode(0);
}

function parseYamlScalar(value) {
  if (value === "true") return true;
  if (value === "false") return false;
  if (value === "null") return null;
  if (/^-?\d+$/.test(value)) return Number.parseInt(value, 10);
  if (value.startsWith("[") && value.endsWith("]")) {
    return value
      .slice(1, -1)
      .split(",")
      .map((part) => parseYamlScalar(part.trim()))
      .filter((part) => part !== "");
  }
  return value.replace(/^["']|["']$/g, "");
}

function parseCliArgs(argv) {
  const args = {};
  const positional = [];
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (!value.startsWith("--")) {
      positional.push(value);
      continue;
    }
    const key = value.slice(2).replace(/-/g, "_");
    const next = argv[index + 1];
    if (!next || next.startsWith("--")) {
      args[key] = true;
      continue;
    }
    args[key] = parseCliValue(next);
    index += 1;
  }
  return { command: positional[0] || "next", positional: positional.slice(1), args };
}

function parseCliValue(value) {
  if (/^-?\d+$/.test(value)) return Number.parseInt(value, 10);
  if (value === "true") return true;
  if (value === "false") return false;
  if (value === "null") return null;
  return value;
}

function validateWorkflowContract(workflow) {
  const tracker = workflow.config.tracker || {};
  if (tracker.kind && tracker.kind !== "linear") {
    throw new Error(`workflow_validation_error: tracker.kind must be linear, got ${tracker.kind}`);
  }

  const template = workflow.prompt_template || "";
  if (/{%[\s\S]*?%}/.test(template)) {
    throw new Error(
      "workflow_validation_error: Liquid tag blocks are not supported; use {{ issue.field }} interpolation only",
    );
  }

  const allowedIssueFields = new Set([
    "id",
    "identifier",
    "title",
    "description",
    "priority",
    "state",
    "branch_name",
    "url",
    "labels",
    "blocked_by",
    "created_at",
    "updated_at",
    "task_file",
    "task_line",
    "task_body",
    "requirements",
    "writes",
    "linear",
  ]);

  const matches = template.matchAll(/\{\{\s*([^}]+?)\s*\}\}/g);
  for (const match of matches) {
    const expression = match[1].trim();
    if (expression.includes("|")) {
      throw new Error(`workflow_validation_error: filters are not supported: ${expression}`);
    }
    if (expression === "attempt") continue;
    if (expression.startsWith("issue.")) {
      const field = expression.slice("issue.".length).split(".")[0];
      if (allowedIssueFields.has(field)) continue;
    }
    throw new Error(`workflow_validation_error: unknown variable ${expression}`);
  }

  return workflow;
}

function listTasks(settings) {
  const root = repositoryRoot(settings);
  return findTaskFiles(root, settings.tasks_glob || DEFAULT_SETTINGS.tasks_glob)
    .flatMap((filePath) => parseTaskFile(filePath, root))
    .sort((left, right) => {
      const pathOrder = left.task_file.localeCompare(right.task_file);
      return pathOrder || left.task_line - right.task_line;
    });
}

function findTaskFiles(root, glob) {
  const normalized = glob.replace(/\\/g, "/");
  if (!normalized.includes("*")) {
    const filePath = path.resolve(root, normalized);
    return fs.existsSync(filePath) ? [filePath] : [];
  }

  const [prefix, suffixWithGlob] = normalized.split("**");
  const baseDir = path.resolve(root, prefix || ".");
  const suffix = suffixWithGlob.replace(/^\//, "").replace(/\*/g, "");
  if (!fs.existsSync(baseDir)) return [];

  const files = [];
  walkFiles(baseDir, (filePath) => {
    const relative = path.relative(baseDir, filePath).replace(/\\/g, "/");
    if (!suffix || relative.endsWith(suffix)) {
      files.push(filePath);
    }
  });
  return files;
}

function walkFiles(directory, visit) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      walkFiles(entryPath, visit);
    } else if (entry.isFile()) {
      visit(entryPath);
    }
  }
}

function parseTaskFile(filePath, root) {
  const text = fs.readFileSync(filePath, "utf8");
  const lines = text.split(/\r?\n/);
  const relativePath = path.relative(root, filePath).replace(/\\/g, "/");
  const tasks = [];
  let current = null;

  function finishCurrent() {
    if (!current) return;
    const requirements = extractMetadata(current.bodyLines, "_Requirements:");
    const writes = extractMetadata(current.bodyLines, "_writes:");
    const description = current.bodyLines.join("\n").trim();
    const taskBody = [current.originalLine, ...current.bodyLines].join("\n").trim();
    const id = stableTaskId(relativePath, current.line, current.title);

    tasks.push({
      id,
      identifier: `${relativePath}:${current.line}`,
      title: current.title,
      description,
      priority: extractPriority(current.title),
      state: markerState(current.marker),
      branch_name: null,
      url: null,
      labels: taskLabels(relativePath),
      blocked_by: [],
      created_at: null,
      updated_at: null,
      task_file: relativePath,
      task_line: current.line,
      task_body: taskBody,
      requirements,
      writes,
      linear: emptyLinearIssue(),
    });
    current = null;
  }

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const match = line.match(/^- \[([ xX~-])\]\s+(.+)$/);
    if (match) {
      finishCurrent();
      const title = match[2].trim().replace(/^\d+[\.)]\s*/, "");
      current = {
        marker: match[1],
        title,
        originalLine: line,
        line: index + 1,
        bodyLines: [],
      };
      continue;
    }

    if (current) {
      if (line.trim() && !/^\s/.test(line)) {
        finishCurrent();
        continue;
      }
      current.bodyLines.push(line);
    }
  }
  finishCurrent();

  return tasks;
}

function extractMetadata(lines, prefix) {
  const values = [];
  const normalizedPrefix = prefix.toLowerCase();
  for (const line of lines) {
    const trimmed = line.trim();
    const start = trimmed.toLowerCase().indexOf(normalizedPrefix);
    if (start === -1) continue;
    const afterPrefix = trimmed.slice(start + prefix.length).trim();
    values.push(afterPrefix.replace(/^_+|_+$/g, "").trim());
  }
  return values;
}

function extractPriority(title) {
  const match = title.match(/^(P[0-4])\.?\s*:?/);
  if (match) return match[1];
  return null;
}

function markerState(marker) {
  if (marker === "x" || marker === "X") return "Done";
  if (marker === "~" || marker === "-") return "In Progress";
  return "Todo";
}

function stableTaskId(relativePath, line, title) {
  const digest = crypto.createHash("sha1").update(`${relativePath}:${line}:${title}`).digest("hex").slice(0, 12);
  return `task:${digest}`;
}

function taskLabels(relativePath) {
  const parts = relativePath
    .split("/")
    .filter((part) => part && part !== ".agents" && part !== "specs" && part !== "tasks.md")
    .map((part) => part.replace(/[^A-Za-z0-9._-]+/g, "-").toLowerCase());
  return Array.from(new Set(["local-task", "workflow", ...parts]));
}

function activeTasks(settings, tasks) {
  const active = new Set((settings.active_states || []).map((state) => state.toLowerCase()));
  const terminal = new Set((settings.terminal_states || []).map((state) => state.toLowerCase()));
  return tasks.filter((task) => {
    const state = (task.state || "").toLowerCase();
    if (active.size > 0) return active.has(state);
    return !terminal.has(state);
  });
}

function loadWorkflowState(settings, options = {}) {
  const filePath = workflowStatePath(settings, options.workflow_state_path);
  const relativePath = path.relative(repositoryRoot(settings), filePath).replace(/\\/g, "/");
  if (!fs.existsSync(filePath)) {
    return {
      path: relativePath,
      exists: false,
      recommendation: null,
      task_notes: [],
      dependency_notes: [],
    };
  }

  let parsed;
  try {
    parsed = JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    return {
      path: relativePath,
      exists: true,
      error: `workflow_state_parse_error: ${error.message}`,
      recommendation: null,
      task_notes: [],
      dependency_notes: [],
    };
  }

  return normalizeWorkflowState(parsed, relativePath);
}

function normalizeWorkflowState(state, relativePath) {
  const taskNotes = Array.isArray(state.task_notes) ? state.task_notes : [];
  const dependencyNotes = Array.isArray(state.dependency_notes) ? state.dependency_notes : [];
  const taskActivity = Array.isArray(state.task_activity) ? state.task_activity : [];
  return {
    path: relativePath,
    exists: true,
    version: state.version || 1,
    updated_at: state.updated_at || null,
    repo_revision: state.repo_revision || null,
    recommendation: normalizeWorkflowStateRecommendation(state.recommendation),
    task_activity: taskActivity.map(normalizeTaskActivity).filter(Boolean),
    task_notes: taskNotes.filter((note) => note && typeof note === "object"),
    dependency_notes: dependencyNotes.filter((note) => note && typeof note === "object"),
    ranked_candidates: Array.isArray(state.ranked_candidates) ? state.ranked_candidates : [],
    maintenance: state.maintenance && typeof state.maintenance === "object" ? state.maintenance : null,
  };
}

function normalizeTaskActivity(activity) {
  if (!activity || typeof activity !== "object") return null;
  const status = normalizeActivityStatus(activity.status || activity.activity || activity.state);
  if (!status) return null;
  return {
    task_id: activity.task_id || null,
    task_identifier: activity.task_identifier || null,
    title: activity.title || null,
    linear_identifier: activity.linear_identifier || null,
    linear_url: activity.linear_url || null,
    status,
    owner: activity.owner || null,
    summary: activity.summary || null,
    updated_at: activity.updated_at || null,
  };
}

function normalizeActivityStatus(status) {
  const normalized = String(status || "").toLowerCase();
  if (["active", "inactive"].includes(normalized)) return normalized;
  return null;
}

function saveWorkflowState(settings, state) {
  const filePath = workflowStatePath(settings);
  const { path: _path, exists: _exists, error: _error, ...serializable } = state;
  serializable.updated_at = new Date().toISOString();
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(serializable, null, 2)}\n`, "utf8");
}

function findTaskActivity(state, task, issue = null) {
  const linearIdentifier = issue?.identifier || task.linear?.identifier || null;
  return (state.task_activity || []).find((activity) => {
    return (
      (activity.task_id && activity.task_id === task.id) ||
      (activity.task_identifier && activity.task_identifier === task.identifier) ||
      (linearIdentifier && activity.linear_identifier === linearIdentifier)
    );
  });
}

function upsertTaskActivity(settings, task, issue, status, options = {}) {
  const normalizedStatus = normalizeActivityStatus(status);
  if (!normalizedStatus) {
    throw new Error(`Unsupported task activity status: ${status}`);
  }

  const state = loadWorkflowState(settings);
  if (state.error) {
    throw new Error(state.error);
  }

  const existingIndex = (state.task_activity || []).findIndex((activity) => {
    return (
      (activity.task_id && activity.task_id === task.id) ||
      (activity.task_identifier && activity.task_identifier === task.identifier) ||
      (issue?.identifier && activity.linear_identifier === issue.identifier)
    );
  });

  const record = {
    task_id: task.id,
    task_identifier: task.identifier,
    title: task.title,
    linear_identifier: issue?.identifier || task.linear?.identifier || null,
    linear_url: issue?.url || task.linear?.url || null,
    status: normalizedStatus,
    owner: options.owner || null,
    summary: options.summary || null,
    updated_at: new Date().toISOString(),
  };

  if (existingIndex >= 0) {
    state.task_activity[existingIndex] = {
      ...state.task_activity[existingIndex],
      ...record,
    };
  } else {
    state.task_activity = [...(state.task_activity || []), record];
  }

  saveWorkflowState(settings, state);
  return record;
}

function claimedTaskStatus(settings, task, issue) {
  const linearActivity = parseLinearWorkflowActivity(issue?.description || "");
  if (linearActivity) {
    return {
      activity: linearActivity,
      source: "linear",
      is_active: linearActivity.status === "active",
      is_inactive: linearActivity.status === "inactive",
    };
  }

  const state = loadWorkflowState(settings);
  const activity = state.error ? null : findTaskActivity(state, task, issue);
  return {
    activity,
    source: activity ? "workflow_state" : "missing",
    is_active: activity?.status === "active",
    is_inactive: activity?.status === "inactive" || !activity,
  };
}

function parseLinearWorkflowActivity(description) {
  const lines = String(description || "").split(/\r?\n/);
  const values = {};
  for (const line of lines) {
    const separator = line.indexOf(":");
    if (separator === -1) continue;
    const key = line.slice(0, separator).trim();
    const value = line.slice(separator + 1).trim();
    if (key === "workflow.activity") values.status = normalizeActivityStatus(value);
    if (key === "workflow.activity_owner") values.owner = value || null;
    if (key === "workflow.activity_summary") values.summary = value || null;
    if (key === "workflow.activity_updated_at") values.updated_at = value || null;
  }
  if (!values.status) return null;
  return {
    task_id: null,
    task_identifier: null,
    title: null,
    linear_identifier: null,
    linear_url: null,
    status: values.status,
    owner: values.owner || null,
    summary: values.summary || null,
    updated_at: values.updated_at || null,
  };
}

function syncTaskActivityRecord(task, issue, activity) {
  return {
    task_id: task.id,
    task_identifier: task.identifier,
    title: task.title,
    linear_identifier: issue?.identifier || task.linear?.identifier || activity?.linear_identifier || null,
    linear_url: issue?.url || task.linear?.url || activity?.linear_url || null,
    status: activity.status,
    owner: activity.owner || null,
    summary: activity.summary || null,
    updated_at: activity.updated_at || new Date().toISOString(),
  };
}

function normalizeWorkflowStateRecommendation(recommendation) {
  if (!recommendation || typeof recommendation !== "object") return null;
  return {
    task_id: recommendation.task_id || null,
    task_identifier: recommendation.task_identifier || null,
    title: recommendation.title || null,
    rationale: recommendation.rationale || recommendation.reason || null,
    evidence: Array.isArray(recommendation.evidence) ? recommendation.evidence : [],
    stale_if_changed: Array.isArray(recommendation.stale_if_changed) ? recommendation.stale_if_changed : [],
    updated_at: recommendation.updated_at || null,
  };
}

function attachWorkflowState(settings, result, tasks) {
  const state = loadWorkflowState(settings);
  return {
    ...result,
    decision_state: summarizeWorkflowState(state, tasks),
  };
}

function summarizeWorkflowState(state, tasks) {
  const recommendation = state.recommendation;
  const recommendedTask = recommendation ? findRecommendedTask(tasks, recommendation) : null;
  const relevantTaskNotes = state.task_notes
    .filter((note) => {
      return tasks.some((task) => {
        return task.id === note.task_id || task.identifier === note.task_identifier;
      });
    })
    .slice(0, 12);

  const relevantTaskActivity = (state.task_activity || [])
    .filter((activity) => {
      return tasks.some((task) => {
        return task.id === activity.task_id || task.identifier === activity.task_identifier;
      });
    })
    .slice(0, 12);

  return {
    path: state.path,
    exists: state.exists,
    error: state.error || null,
    updated_at: state.updated_at || null,
    repo_revision: state.repo_revision || null,
    task_activity: relevantTaskActivity,
    recommendation: recommendation
      ? {
          ...recommendation,
          matches_active_task: Boolean(recommendedTask),
          matched_task_id: recommendedTask?.id || null,
          matched_task_identifier: recommendedTask?.identifier || null,
        }
      : null,
    relevant_task_notes: relevantTaskNotes,
    dependency_notes: state.dependency_notes.slice(0, 12),
  };
}

function findRecommendedTask(tasks, recommendation) {
  return (
    tasks.find((task) => {
      return task.id === recommendation.task_id || task.identifier === recommendation.task_identifier;
    }) || null
  );
}

function findTask(settings, taskId) {
  const task = listTasks(settings).find((candidate) => {
    return candidate.id === taskId || candidate.identifier === taskId;
  });
  if (!task) {
    throw new Error(`Local task not found: ${taskId}`);
  }
  return task;
}

function emptyLinearIssue() {
  return {
    id: null,
    identifier: null,
    title: null,
    url: null,
    state: null,
  };
}

function renderTemplate(template, issue, attempt) {
  const source = template || defaultPromptTemplate();
  return source.replace(/\{\{\s*([^}]+?)\s*\}\}/g, (_match, expression) => {
    if (expression.includes("|")) {
      throw new Error(`template_render_error: unknown filters are not supported: ${expression}`);
    }
    const value = resolveTemplateValue(expression.trim(), { issue, attempt });
    if (value === undefined) {
      throw new Error(`template_render_error: unknown variable ${expression.trim()}`);
    }
    if (value === null) return "";
    if (typeof value === "object") return JSON.stringify(value, null, 2);
    return String(value);
  });
}

function defaultPromptTemplate() {
  return [
    "You are working on a local Sim spec task.",
    "",
    "Task: {{ issue.title }}",
    "Source: {{ issue.task_file }}:{{ issue.task_line }}",
    "",
    "Task body:",
    "{{ issue.task_body }}",
  ].join("\n");
}

function resolveTemplateValue(expression, scope) {
  const parts = expression.split(".");
  let value = scope[parts.shift()];
  for (const part of parts) {
    if (value == null || !Object.prototype.hasOwnProperty.call(value, part)) {
      return undefined;
    }
    value = value[part];
  }
  return value;
}

async function populateLinear(settings, task, preliminaryPrompt) {
  validateLinearSettings(settings);
  const teamId = await resolveLinearTeamId(settings);
  const projectId = await resolveLinearProjectId(settings);
  const description = linearIssueDescription(task, preliminaryPrompt);
  const input = {
    teamId,
    title: task.title,
    description,
  };

  if (projectId) input.projectId = projectId;
  if (settings.linear_state_id) input.stateId = settings.linear_state_id;
  if (Array.isArray(settings.linear_label_ids) && settings.linear_label_ids.length > 0) {
    input.labelIds = settings.linear_label_ids;
  }

  const mutation = `
    mutation($input: IssueCreateInput!) {
      issueCreate(input: $input) {
        success
        issue {
          id
          identifier
          title
          url
          branchName
          state { name }
        }
      }
    }`;
  const data = await linear(settings, mutation, { input });
  const created = data.issueCreate;
  if (!created?.success || !created.issue) {
    throw new Error("Linear issueCreate did not return a created issue");
  }

  return {
    id: created.issue.id,
    identifier: created.issue.identifier,
    title: created.issue.title,
    url: created.issue.url,
    branch_name: created.issue.branchName || null,
    state: created.issue.state?.name || null,
  };
}

function validateLinearSettings(settings) {
  if (!settings.linear_api_key) throw new Error("Missing required setting: linear_api_key");
  if (!settings.linear_team_id && !settings.linear_team_key) {
    throw new Error("Missing required setting: linear_team_id or linear_team_key");
  }
}

async function resolveLinearTeamId(settings) {
  if (settings.linear_team_id) return settings.linear_team_id;
  const teamKey = resolveEnvReference(settings.linear_team_key, "linear_team_key");
  const query = `
    query($key: String!) {
      teams(filter: { key: { eq: $key } }) {
        nodes { id key name }
      }
    }`;
  const data = await linear(settings, query, { key: teamKey });
  const team = data.teams?.nodes?.[0];
  if (!team) {
    throw new Error(`Linear team not found for key ${teamKey}`);
  }
  return team.id;
}

async function resolveLinearProjectId(settings) {
  if (settings.linear_project_id) return settings.linear_project_id;
  if (!settings.linear_project_slug) return null;
  const projectSlug = resolveEnvReference(settings.linear_project_slug, "linear_project_slug");
  const query = `
    query($slug: String!) {
      projects(filter: { slugId: { eq: $slug } }) {
        nodes { id name slugId }
      }
    }`;
  const data = await linear(settings, query, { slug: projectSlug });
  const project = data.projects?.nodes?.[0];
  if (!project) {
    throw new Error(`Linear project not found for slug ${projectSlug}`);
  }
  return project.id;
}

async function linear(settings, query, variables = {}) {
  const token = resolveEnvReference(settings.linear_api_key, "linear_api_key");
  const response = await fetch(settings.linear_endpoint || DEFAULT_LINEAR_ENDPOINT, {
    method: "POST",
    headers: {
      authorization: token,
      "content-type": "application/json",
      "user-agent": "sim-workflow-skill",
    },
    body: JSON.stringify({ query, variables }),
  });
  const payload = await response.json();
  if (!response.ok || payload.errors) {
    const details = payload.errors ? JSON.stringify(payload.errors) : response.statusText;
    throw new Error(`Linear GraphQL request failed: ${details}`);
  }
  return payload.data;
}

function linearIssueDescription(task, _prompt) {
  return [
    "Workflow picked this local Sim spec task.",
    "",
    workflowTaskMarker(task),
    workflowSourceMarker(task),
    "workflow.activity:active",
    `workflow.activity_updated_at:${new Date().toISOString()}`,
    "",
    `Task: ${task.title}`,
    `Source: ${task.task_file}:${task.task_line}`,
    "",
    "Summary:",
    task.description || task.title,
    "",
    "Requirements:",
    formatList(task.requirements),
    "",
    "Expected writes:",
    formatList(task.writes),
  ].join("\n");
}

function formatList(values) {
  if (!values || values.length === 0) return "- N/A";
  return values.map((value) => `- ${value}`).join("\n");
}

function withLinearActivityMarkers(description, activity) {
  const markers = {
    "workflow.activity": activity.status,
    "workflow.activity_updated_at": activity.updated_at || new Date().toISOString(),
    "workflow.activity_owner": activity.owner || null,
    "workflow.activity_summary": activity.summary || null,
  };

  let lines = String(description || "")
    .split(/\r?\n/)
    .filter((line) => {
      return !Object.keys(markers).some((key) => line.startsWith(`${key}:`));
    });

  const sourceIndex = lines.findIndex((line) => line.startsWith("workflow.local_task_source:"));
  const insertIndex = sourceIndex >= 0 ? sourceIndex + 1 : lines.findIndex((line) => line.trim() === "");
  const markerLines = Object.entries(markers)
    .filter(([_key, value]) => value)
    .map(([key, value]) => `${key}:${value}`);

  if (insertIndex >= 0) {
    lines.splice(insertIndex, 0, ...markerLines);
  } else {
    lines = [...markerLines, "", ...lines];
  }

  return lines.join("\n");
}

async function updateLinearIssueActivity(settings, issue, activity) {
  const query = `
    query($id: String!) {
      issue(id: $id) {
        id
        identifier
        description
      }
    }`;
  const fetched = await linear(settings, query, { id: issue.identifier || issue.id });
  const currentDescription = fetched.issue?.description || "";
  const description = withLinearActivityMarkers(currentDescription, activity);
  const mutation = `
    mutation($id: String!, $input: IssueUpdateInput!) {
      issueUpdate(id: $id, input: $input) {
        success
        issue {
          id
          identifier
          title
          url
          branchName
          description
          state { name }
        }
      }
    }`;
  const data = await linear(settings, mutation, {
    id: issue.identifier || issue.id,
    input: { description },
  });
  const updated = data.issueUpdate;
  if (!updated?.success || !updated.issue) {
    throw new Error("Linear issueUpdate did not return an updated issue");
  }
  return updated.issue;
}

function groupLabel(dir) {
  const name = dir.split("/").pop() || dir;
  return name
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function groupReadinessLabel(groupInfo) {
  if (groupInfo.previousDone) return "ready (previous tasks done)";
  if (groupInfo.previousNone) return "ready (no dependencies)";
  if (groupInfo.previousPartial)
    return `waiting on ${groupInfo.totalPrevious - groupInfo.donePrevious} previous task${groupInfo.totalPrevious - groupInfo.donePrevious !== 1 ? "s" : ""}`;
  return "unknown";
}

function rankCandidates(tasks, settings) {
  if (tasks.length <= 1) return tasks;

  const state = loadWorkflowState(settings);
  const allTasks = listTasks(settings);
  const priorityOrder = { P0: 0, P1: 1, P2: 2, P3: 3, P4: 4 };

  const groups = {};
  for (const task of tasks) {
    const dir = path.dirname(task.task_file);
    if (!groups[dir]) groups[dir] = [];
    groups[dir].push(task);
  }

  const groupInfo = {};
  for (const [dir, groupTasks] of Object.entries(groups)) {
    const allInDir = allTasks
      .filter((t) => path.dirname(t.task_file) === dir)
      .sort((a, b) => a.task_line - b.task_line);

    const firstLine = Math.min(...groupTasks.map((t) => t.task_line));
    const previousTasks = allInDir.filter((t) => t.task_line < firstLine);
    const doneCount = previousTasks.filter((t) => (t.state || "").toLowerCase() === "done").length;

    let depNote = null;
    if (state.dependency_notes) {
      for (const note of state.dependency_notes) {
        if (note.scope) {
          const normalizedScope = note.scope.replace(/^\.[\\/]/, "");
          if (dir === normalizedScope || dir.endsWith("/" + normalizedScope) || dir.includes(normalizedScope)) {
            depNote = note;
            break;
          }
        }
      }
    }

    groupInfo[dir] = {
      previousDone: previousTasks.length > 0 && doneCount === previousTasks.length,
      previousPartial: doneCount > 0 && doneCount < previousTasks.length,
      previousNone: previousTasks.length === 0,
      totalPrevious: previousTasks.length,
      donePrevious: doneCount,
      depNote,
    };
  }

  function groupScore(dir) {
    const info = groupInfo[dir];
    if (info.previousDone) return 0;
    if (info.previousNone) return 20;
    if (info.previousPartial) return 50;
    return 100;
  }

  const sortedDirs = Object.keys(groups).sort((a, b) => {
    const scoreA = groupScore(a);
    const scoreB = groupScore(b);
    if (scoreA !== scoreB) return scoreA - scoreB;

    const bestPrioA = Math.min(...groups[a].map((t) => priorityOrder[t.priority] ?? 5));
    const bestPrioB = Math.min(...groups[b].map((t) => priorityOrder[t.priority] ?? 5));
    if (bestPrioA !== bestPrioB) return bestPrioA - bestPrioB;

    return Math.min(...groups[a].map((t) => t.task_line)) - Math.min(...groups[b].map((t) => t.task_line));
  });

  const ranked = [];
  let globalRank = 0;
  for (const dir of sortedDirs) {
    const dirTasks = [...groups[dir]].sort((a, b) => {
      const aPrio = priorityOrder[a.priority] ?? 5;
      const bPrio = priorityOrder[b.priority] ?? 5;
      if (aPrio !== bPrio) return aPrio - bPrio;
      return a.task_line - b.task_line;
    });

    for (const task of dirTasks) {
      globalRank++;
      ranked.push({
        ...task,
        rank: globalRank,
        group: dir,
        group_info: groupInfo[dir],
      });
    }
  }

  return ranked;
}

async function pickNextTask(settings, workflow, tasks, attempt) {
  if (tasks.length === 0) {
    return {
      task: null,
      prompt: null,
      action: "none",
      duplicate_prevented: false,
      skipped_claimed_tasks: [],
      reason: "No active local tasks found.",
    };
  }

  const ranked = rankCandidates(tasks, settings);
  const candidates = [];
  const skippedClaimedTasks = [];

  for (const task of ranked) {
    const existing = await findExistingLinearIssue(settings, task);
    if (!existing) {
      candidates.push(task);
      continue;
    }

    const claimed = claimedTaskStatus(settings, task, existing);
    if (claimed.is_active) {
      skippedClaimedTasks.push(skippedClaimedTask(task, existing, claimed.activity));
      continue;
    }

    const picked = await pickTask(settings, workflow, task, attempt, {
      existingIssue: existing,
      existingChecked: true,
      markActive: true,
    });
    return {
      task: picked.task,
      prompt: picked.prompt,
      action: "resumed_inactive",
      duplicate_prevented: skippedClaimedTasks.length > 0,
      candidates: null,
      skipped_claimed_tasks: skippedClaimedTasks,
      reason: "Resumed an inactive claimed task before starting new work.",
    };
  }

  if (candidates.length === 1) {
    const picked = await pickTask(settings, workflow, candidates[0], attempt, {
      existingChecked: true,
      markActive: true,
    });
    return {
      task: picked.task,
      prompt: picked.prompt,
      action: picked.action,
      duplicate_prevented: skippedClaimedTasks.length > 0,
      candidates: null,
      skipped_claimed_tasks: skippedClaimedTasks,
    };
  }

  if (candidates.length === 0) {
    return {
      task: null,
      prompt: null,
      action: "none",
      duplicate_prevented: skippedClaimedTasks.length > 0,
      skipped_claimed_tasks: skippedClaimedTasks,
      reason: "All active local tasks are already claimed and marked active in workflow state.",
    };
  }

  return {
    task: null,
    prompt: null,
    action: "evaluate",
    duplicate_prevented: skippedClaimedTasks.length > 0,
    candidates,
    skipped_claimed_tasks: skippedClaimedTasks,
    reason:
      `${candidates.length} unclaimed tasks available. ` +
      "Top candidates ranked by priority and dependency order. " +
      "Pick one with: node .agents/skills/workflow/scripts/workflow.js pick <task-id>",
  };
}

function skippedClaimedTask(task, issue, activity = null) {
  return {
    task_id: task.id,
    task_identifier: task.identifier,
    task_title: task.title,
    linear_identifier: issue.identifier || null,
    linear_url: issue.url || null,
    linear_state: typeof issue.state === "string" ? issue.state : issue.state?.name || null,
    activity_status: activity?.status || null,
    activity_owner: activity?.owner || null,
    activity_updated_at: activity?.updated_at || null,
  };
}

async function pickNextTasks(settings, workflow, tasks, attempt, count) {
  const ranked = rankCandidates(tasks, settings);
  const pickedTasks = [];
  const skippedClaimedTasks = [];

  for (const task of ranked) {
    if (pickedTasks.length >= count) break;
    const existing = await findExistingLinearIssue(settings, task);
    const claimed = existing ? claimedTaskStatus(settings, task, existing) : null;
    if (existing && claimed?.is_active) {
      skippedClaimedTasks.push(skippedClaimedTask(task, existing, claimed.activity));
      continue;
    }

    const picked = await pickTask(settings, workflow, task, attempt, {
      existingIssue: existing,
      existingChecked: true,
      markActive: true,
    });
    pickedTasks.push({
      task: picked.task,
      prompt: picked.prompt,
      action: picked.action,
      duplicate_prevented: skippedClaimedTasks.length > 0,
    });
  }

  return {
    tasks: pickedTasks.map((picked) => picked.task),
    prompts: pickedTasks.map((picked) => picked.prompt),
    picked: pickedTasks,
    task: pickedTasks[0]?.task || null,
    prompt: pickedTasks[0]?.prompt || null,
    action: pickedTasks.length > 0 ? "created_batch" : "none",
    duplicate_prevented: skippedClaimedTasks.length > 0,
    skipped_claimed_tasks: skippedClaimedTasks,
    reason: pickedTasks.length > 0 ? null : "All active local tasks already have non-terminal Linear issues.",
  };
}

async function pickTask(settings, workflow, task, attempt, options = {}) {
  const existing =
    options.existingIssue ||
    (settings.resume_existing !== false && !options.existingChecked
      ? await findExistingLinearIssue(settings, task)
      : null);

  if (existing) {
    const claimed = claimedTaskStatus(settings, task, existing);
    if (claimed.is_active && !options.force) {
      return {
        task: null,
        prompt: null,
        action: "claimed_active",
        duplicate_prevented: true,
        reason:
          `Task ${task.identifier} is already claimed by ${existing.identifier || "a Linear issue"} ` +
          "and marked active in workflow state. Pass --force to take it over.",
        skipped_claimed_tasks: [skippedClaimedTask(task, existing, claimed.activity)],
      };
    }

    attachLinearIssue(task, existing);
    if (options.markActive !== false) {
      const activity = upsertTaskActivity(settings, task, existing, "active", {
        owner: options.owner || null,
        summary: options.summary || null,
      });
      await updateLinearIssueActivity(settings, existing, activity);
    }
    return {
      task,
      prompt: renderTemplate(workflow.prompt_template, task, attempt),
      action: claimed.is_inactive ? "resumed_inactive" : "resumed",
      duplicate_prevented: true,
    };
  }

  const preliminaryPrompt = renderTemplate(workflow.prompt_template, task, attempt);
  const issue = await populateLinear(settings, task, preliminaryPrompt);
  attachLinearIssue(task, issue);
  if (options.markActive !== false) {
    const activity = upsertTaskActivity(settings, task, issue, "active", {
      owner: options.owner || null,
      summary: options.summary || null,
    });
    await updateLinearIssueActivity(settings, issue, activity);
  }
  return {
    task,
    prompt: renderTemplate(workflow.prompt_template, task, attempt),
    action: "created",
    duplicate_prevented: false,
  };
}

function attachLinearIssue(task, issue) {
  const state = typeof issue.state === "string" ? issue.state : issue.state?.name;
  task.linear = {
    id: issue.id || null,
    identifier: issue.identifier || null,
    title: issue.title || null,
    url: issue.url || null,
    state: state || null,
  };
  task.branch_name = issue.branch_name || issue.branchName || task.branch_name || null;
  task.url = issue.url || task.url;
}

async function findExistingLinearIssue(settings, task) {
  validateLinearSettings(settings);
  const query = `
    query($term: String!) {
      searchIssues(term: $term, first: 10) {
        nodes {
          id
          identifier
          title
          url
          branchName
          description
          state { name }
        }
      }
    }`;
  const issues = [];
  const seen = new Set();
  for (const term of [task.id, task.identifier]) {
    const data = await linear(settings, query, { term });
    for (const issue of data.searchIssues?.nodes || []) {
      if (seen.has(issue.id)) continue;
      seen.add(issue.id);
      issues.push(issue);
    }
  }

  const terminal = new Set((settings.terminal_states || []).map((state) => state.toLowerCase()));
  const sourceMarker = workflowSourceMarker(task);

  return (
    issues.find((issue) => {
      const description = issue.description || "";
      const state = (issue.state?.name || "").toLowerCase();
      if (terminal.has(state)) return false;
      return description.includes(workflowTaskMarker(task)) || description.includes(sourceMarker);
    }) || null
  );
}

function workflowTaskMarker(task) {
  return `workflow.local_task_id:${task.id}`;
}

function workflowSourceMarker(task) {
  return `workflow.local_task_source:${task.identifier}`;
}

async function moveLinearIssue(settings, args) {
  const issue = await resolveLinearIssueForMove(settings, args);
  const stateId = args.state_id || (await resolveLinearStateId(settings, args.state || args.state_name));
  if (!stateId) {
    throw new Error("Missing target Linear state. Pass --state-id or --state-name.");
  }

  const mutation = `
    mutation($id: String!, $input: IssueUpdateInput!) {
      issueUpdate(id: $id, input: $input) {
        success
        issue {
          id
          identifier
          title
          url
          branchName
          state { name }
        }
      }
    }`;
  const data = await linear(settings, mutation, {
    id: issue.id,
    input: { stateId },
  });
  const updated = data.issueUpdate;
  if (!updated?.success || !updated.issue) {
    throw new Error("Linear issueUpdate did not return an updated issue");
  }
  return {
    action: "moved",
    issue: {
      id: updated.issue.id,
      identifier: updated.issue.identifier,
      title: updated.issue.title,
      url: updated.issue.url,
      branch_name: updated.issue.branchName || null,
      state: updated.issue.state?.name || null,
    },
  };
}

async function resolveLinearIssueForMove(settings, args) {
  const key = args.issue || args.issue_id || args.task_id;
  if (!key) {
    throw new Error("Missing issue or task identifier. Pass move <task-id-or-linear-id>.");
  }

  const task = listTasks(settings).find((candidate) => {
    return candidate.id === key || candidate.identifier === key;
  });
  if (task) {
    const issue = await findExistingLinearIssue(settings, task);
    if (!issue) {
      throw new Error(`No non-terminal Linear issue found for local task ${key}`);
    }
    return issue;
  }

  const query = `
    query($term: String!) {
      searchIssues(term: $term, first: 10) {
        nodes {
          id
          identifier
          title
          url
          branchName
          state { name }
        }
      }
    }`;
  const data = await linear(settings, query, { term: key });
  const issue =
    (data.searchIssues?.nodes || []).find((candidate) => {
      return candidate.id === key || candidate.identifier === key || candidate.url === key;
    }) || data.searchIssues?.nodes?.[0];
  if (!issue) {
    throw new Error(`Linear issue not found: ${key}`);
  }
  return issue;
}

async function resolveLinearStateId(settings, stateName) {
  if (!stateName) return null;
  const teamId = await resolveLinearTeamId(settings);
  const query = `
    query($teamId: ID!, $name: String!) {
      workflowStates(filter: { team: { id: { eq: $teamId } }, name: { eq: $name } }) {
        nodes { id name }
      }
    }`;
  const data = await linear(settings, query, { teamId, name: stateName });
  const state = data.workflowStates?.nodes?.[0];
  if (!state) {
    throw new Error(`Linear workflow state not found: ${stateName}`);
  }
  return state.id;
}

function checkMergedToMain(settings) {
  const root = repositoryRoot(settings);

  try {
    const currentBranch = childProcess
      .execSync("git rev-parse --abbrev-ref HEAD", { cwd: root, encoding: "utf8", stdio: "pipe" })
      .trim();
    if (currentBranch === "main") return true;
  } catch {
    return null;
  }

  try {
    childProcess.execSync("git rev-parse --verify origin/main", {
      cwd: root,
      encoding: "utf8",
      stdio: "pipe",
    });
  } catch {
    return null;
  }

  try {
    childProcess.execSync("git merge-base --is-ancestor HEAD origin/main", {
      cwd: root,
      stdio: "pipe",
    });
    return true;
  } catch {
    return false;
  }
}

async function updateTaskActivity(settings, args) {
  const key = args.task_id || args.issue || args.issue_id || args.task || args.id;
  if (!key) {
    throw new Error(
      "Missing task or Linear issue identifier. Pass activity <task-id-or-linear-id> --status active|inactive.",
    );
  }

  const status = normalizeActivityStatus(args.status || args.state || args.activity);
  if (!status) {
    throw new Error("Missing or unsupported activity status. Use --status active or --status inactive.");
  }

  const tasks = listTasks(settings);
  let task = tasks.find((candidate) => candidate.id === key || candidate.identifier === key);
  let issue = null;

  if (task) {
    issue = await findExistingLinearIssue(settings, task);
    if (!issue) {
      throw new Error(`No non-terminal Linear issue found for local task ${key}`);
    }
  } else {
    issue = await resolveLinearIssueForMove(settings, { issue: key });
    task = await findTaskForLinearIssue(settings, issue, tasks);
    if (!task) {
      throw new Error(`Could not match ${key} to a local workflow task.`);
    }
  }

  attachLinearIssue(task, issue);
  const activity = upsertTaskActivity(settings, task, issue, status, {
    owner: args.owner || null,
    summary: args.summary || null,
  });
  const updatedIssue = await updateLinearIssueActivity(settings, issue, activity);
  attachLinearIssue(task, updatedIssue);

  return {
    action: "activity_updated",
    task,
    issue: task.linear,
    activity,
  };
}

async function findTaskForLinearIssue(settings, issue, tasks = listTasks(settings)) {
  const query = `
    query($id: String!) {
      issue(id: $id) {
        id
        identifier
        description
      }
    }`;
  const data = await linear(settings, query, { id: issue.identifier || issue.id });
  const description = data.issue?.description || "";
  return tasks.find((task) => {
    return description.includes(workflowTaskMarker(task)) || description.includes(workflowSourceMarker(task));
  });
}

async function completeTask(settings, args) {
  const taskId = args.task_id || args.issue || args.issue_id;
  if (!taskId) {
    throw new Error("Missing task identifier. Pass complete <task-id>.");
  }

  const task = findTask(settings, taskId);

  if (!args.force) {
    const merged = checkMergedToMain(settings);
    if (merged === false) {
      throw new Error(
        "Task cannot be completed: the current branch has not been merged into main. " +
          "Complete the PR process first (commit → push → open PR → land/merge to main), " +
          "then run complete again. To override this check, pass --force.",
      );
    }
    if (merged === null) {
      console.warn(
        "Warning: could not verify merge status (not a git repository or no origin/main remote). " +
          "Proceeding with completion. Pass --force to silence this warning.",
      );
    }
  }

  let linear = null;
  if (!args.local_only) {
    linear = await moveLinearIssue(settings, {
      ...args,
      issue: task.id,
      state_name: args.state_name || args.state || "Done",
    });
  }

  const local = args.no_local
    ? { updated: false, reason: "Local checkbox update skipped by --no-local." }
    : updateLocalTaskCheckbox(settings, task, "Done");

  let activity = null;
  if (linear?.issue) {
    activity = upsertTaskActivity(settings, task, linear.issue, "inactive", {
      summary: "Completed by workflow command.",
    });
    await updateLinearIssueActivity(settings, linear.issue, activity);
  }

  return {
    action: "completed",
    task: findTask(settings, task.id),
    local,
    linear,
    activity,
  };
}

function updateLocalTaskCheckbox(settings, task, stateName) {
  const root = repositoryRoot(settings);
  const filePath = path.resolve(root, task.task_file);
  const text = fs.readFileSync(filePath, "utf8");
  const newline = text.includes("\r\n") ? "\r\n" : "\n";
  const lines = text.split(/\r?\n/);
  const lineIndex = task.task_line - 1;
  const line = lines[lineIndex];
  if (line === undefined) {
    throw new Error(`Task source line not found: ${task.identifier}`);
  }

  const marker = localMarkerForState(stateName);
  const updatedLine = line.replace(/^- \[[ xX~-]\]/, `- [${marker}]`);
  if (updatedLine === line) {
    throw new Error(`Task source line is not a top-level checkbox: ${task.identifier}`);
  }

  lines[lineIndex] = updatedLine;
  fs.writeFileSync(filePath, lines.join(newline), "utf8");
  return {
    updated: true,
    state: stateName,
    task_file: task.task_file,
    task_line: task.task_line,
    before: line,
    after: updatedLine,
  };
}

function localMarkerForState(stateName) {
  const normalized = String(stateName || "").toLowerCase();
  if (["done", "closed", "complete", "completed"].includes(normalized)) return "x";
  if (["todo", "backlog", "open"].includes(normalized)) return " ";
  return "~";
}

async function callTool(name, args) {
  const settings = parseSettings();
  if (name === "workflow_launch_ui") {
    return launchWorkflowUi(settings, args);
  }

  if (name === "workflow_serve_ui") {
    return serveWorkflowUi(settings, args);
  }

  if (name === "workflow_validate_workflow") {
    const workflow = validateWorkflowContract(parseWorkflow(workflowPath(settings, args.workflow_path)));
    return {
      workflow,
      validation: { valid: true },
    };
  }

  if (name === "workflow_list_tasks") {
    const tasks = listTasks(settings);
    const filtered = args.active_only === false ? tasks : activeTasks(settings, tasks);
    return {
      tasks: filtered.slice(0, args.limit || 50),
      total: filtered.length,
    };
  }

  if (name === "workflow_render_prompt") {
    const task = findTask(settings, args.task_id);
    const workflow = validateWorkflowContract(parseWorkflow(workflowPath(settings)));
    return {
      task,
      prompt: renderTemplate(workflow.prompt_template, task, args.attempt ?? null),
      workflow: { path: workflow.path, config: workflow.config },
    };
  }

  if (name === "workflow_pick_task") {
    const task = findTask(settings, args.task_id);
    const workflow = validateWorkflowContract(parseWorkflow(workflowPath(settings)));
    const picked = await pickTask(settings, workflow, task, args.attempt ?? null, {
      force: Boolean(args.force),
      owner: args.owner || null,
      summary: args.summary || null,
    });
    return {
      task: picked.task,
      prompt: picked.prompt,
      duplicate_prevented: picked.duplicate_prevented,
      action: picked.action,
      reason: picked.reason || null,
      skipped_claimed_tasks: picked.skipped_claimed_tasks || [],
      workflow: { path: workflow.path, config: workflow.config },
    };
  }

  if (name === "workflow_next_task") {
    const workflow = validateWorkflowContract(parseWorkflow(workflowPath(settings)));
    const tasks = activeTasks(settings, listTasks(settings)).slice(0, args.limit || 200);
    const count = Math.max(1, args.count || 1);
    const picked =
      count === 1
        ? await pickNextTask(settings, workflow, tasks, args.attempt ?? null)
        : await pickNextTasks(settings, workflow, tasks, args.attempt ?? null, count);
    return {
      ...attachWorkflowState(settings, picked, tasks),
      workflow: { path: workflow.path, config: workflow.config },
    };
  }

  if (name === "workflow_move_linear") {
    return moveLinearIssue(settings, args);
  }

  if (name === "workflow_complete_task") {
    return completeTask(settings, args);
  }

  if (name === "workflow_update_activity") {
    return updateTaskActivity(settings, args);
  }

  throw new Error(`Unknown tool: ${name}`);
}

async function launchWorkflowUi(settings, args = {}) {
  const htmlPath = args.ui_path
    ? path.resolve(String(args.ui_path))
    : path.resolve(__dirname, "..", "assets", "ui", "index.html");
  if (!fs.existsSync(htmlPath)) {
    throw new Error(`Workflow UI not found: ${htmlPath}`);
  }

  if (args.static || args.file || args.print) {
    const url = pathToFileURL(htmlPath).href;
    return {
      action: "resolved",
      launched: false,
      served: false,
      ui: { path: htmlPath, url },
    };
  }

  const host = String(args.host || "127.0.0.1");
  const port = await resolveUiPort(host, args.port);
  const dataSource = String(args.data || args.source || "list");
  const serverArgs = {
    host,
    port,
    data: dataSource,
    limit: args.limit,
    count: args.count,
    active_only: args.active_only,
    task_id: args.task_id,
    attempt: args.attempt,
    ttl_ms: args.ttl_ms,
    ui_path: htmlPath,
  };
  const serverProcess = childProcess.spawn(
    process.execPath,
    [__filename, "ui-server", ...cliArgsFromObject(serverArgs)],
    {
      cwd: repositoryRoot(settings),
      detached: true,
      env: process.env,
      stdio: "ignore",
    },
  );
  serverProcess.unref();

  const url = `http://${host}:${port}/?data=/workflow-data.json`;
  await waitForUiServer(url);

  const shouldOpen = args.open !== false && !args.no_open && !args.json;
  if (!shouldOpen) {
    return {
      action: "served",
      launched: false,
      served: true,
      server: { host, port, pid: serverProcess.pid, data_source: dataSource },
      ui: { path: htmlPath, url, data_url: `http://${host}:${port}/workflow-data.json` },
    };
  }

  const launcher = platformLauncher(url);
  const child = childProcess.spawn(launcher.command, launcher.args, {
    detached: true,
    stdio: "ignore",
  });
  child.unref();

  return {
    action: "opened",
    launched: true,
    served: true,
    server: { host, port, pid: serverProcess.pid, data_source: dataSource },
    ui: { path: htmlPath, url, data_url: `http://${host}:${port}/workflow-data.json` },
  };
}

async function serveWorkflowUi(settings, args = {}) {
  const htmlPath = args.ui_path
    ? path.resolve(String(args.ui_path))
    : path.resolve(__dirname, "..", "assets", "ui", "index.html");
  if (!fs.existsSync(htmlPath)) {
    throw new Error(`Workflow UI not found: ${htmlPath}`);
  }

  const host = String(args.host || "127.0.0.1");
  const port = Number(args.port || 0);
  const ttlMs = Number(args.ttl_ms || 4 * 60 * 60 * 1000);
  const payload = await buildUiPayload(settings, args);
  const payloadText = JSON.stringify(payload, null, 2);

  const server = http.createServer((request, response) => {
    const requestUrl = new URL(request.url || "/", `http://${host}:${port || 80}`);
    if (requestUrl.pathname === "/" || requestUrl.pathname === "/index.html") {
      respond(response, 200, "text/html; charset=utf-8", fs.readFileSync(htmlPath, "utf8"));
    } else if (requestUrl.pathname === "/workflow-data.json") {
      respond(response, 200, "application/json; charset=utf-8", payloadText);
    } else if (requestUrl.pathname === "/health") {
      respond(response, 200, "application/json; charset=utf-8", JSON.stringify({ ok: true }));
    } else {
      respond(response, 404, "text/plain; charset=utf-8", "Not found");
    }
  });

  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, host, resolve);
  });

  const address = server.address();
  const actualPort = typeof address === "object" && address ? address.port : port;
  process.stdout.write(
    JSON.stringify(
      {
        action: "serving",
        server: { host, port: actualPort, data_source: payload.ui?.data_source || "list" },
        ui: {
          path: htmlPath,
          url: `http://${host}:${actualPort}/?data=/workflow-data.json`,
          data_url: `http://${host}:${actualPort}/workflow-data.json`,
        },
      },
      null,
      2,
    ),
  );
  process.stdout.write("\n");

  if (ttlMs > 0) {
    setTimeout(() => {
      server.close(() => process.exit(0));
    }, ttlMs).unref();
  }
}

async function buildUiPayload(settings, args = {}) {
  const dataSource = String(args.data || args.source || "list");
  if (dataSource === "none") {
    return {
      tasks: [],
      total: 0,
      ui: { data_source: dataSource, generated_at: new Date().toISOString() },
    };
  }

  if (dataSource === "next") {
    const workflow = validateWorkflowContract(parseWorkflow(workflowPath(settings)));
    const tasks = activeTasks(settings, listTasks(settings)).slice(0, args.limit || 200);
    const count = Math.max(1, args.count || 1);
    const picked =
      count === 1
        ? await pickNextTask(settings, workflow, tasks, args.attempt ?? null)
        : await pickNextTasks(settings, workflow, tasks, args.attempt ?? null, count);
    return {
      ...attachWorkflowState(settings, picked, tasks),
      ui: { data_source: dataSource, generated_at: new Date().toISOString() },
      workflow: { path: workflow.path, config: workflow.config },
    };
  }

  if (dataSource === "pick") {
    const taskId = args.task_id;
    if (!taskId) throw new Error("workflow ui --data pick requires --task-id <task-id>");
    const task = findTask(settings, taskId);
    const workflow = validateWorkflowContract(parseWorkflow(workflowPath(settings)));
    const picked = await pickTask(settings, workflow, task, args.attempt ?? null);
    return {
      task: picked.task,
      prompt: picked.prompt,
      duplicate_prevented: picked.duplicate_prevented,
      action: picked.action,
      ui: { data_source: dataSource, generated_at: new Date().toISOString() },
      workflow: { path: workflow.path, config: workflow.config },
    };
  }

  if (dataSource !== "list") {
    throw new Error(`Unsupported UI data source: ${dataSource}`);
  }

  const tasks = listTasks(settings);
  const filtered = args.active_only === false ? tasks : activeTasks(settings, tasks);
  return {
    tasks: filtered.slice(0, args.limit || 500),
    total: filtered.length,
    ui: { data_source: dataSource, generated_at: new Date().toISOString() },
  };
}

function respond(response, status, contentType, body) {
  response.writeHead(status, {
    "content-type": contentType,
    "cache-control": "no-store",
  });
  response.end(body);
}

async function resolveUiPort(host, requestedPort) {
  if (requestedPort) return Number(requestedPort);
  for (let port = 47631; port < 47731; port += 1) {
    if (await isPortAvailable(host, port)) return port;
  }
  throw new Error("No available workflow UI port found in range 47631-47730");
}

function isPortAvailable(host, port) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.once("error", () => resolve(false));
    server.once("listening", () => {
      server.close(() => resolve(true));
    });
    server.listen(port, host);
  });
}

async function waitForUiServer(url) {
  const healthUrl = new URL("/health", url).href;
  const startedAt = Date.now();
  while (Date.now() - startedAt < 5000) {
    try {
      const response = await fetch(healthUrl);
      if (response.ok) return;
    } catch (_error) {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  throw new Error(`Workflow UI server did not become ready: ${healthUrl}`);
}

function cliArgsFromObject(values) {
  const args = [];
  for (const [key, value] of Object.entries(values)) {
    if (value === undefined || value === null || value === false) continue;
    const cliKey = `--${key.replace(/_/g, "-")}`;
    if (value === true) {
      args.push(cliKey);
    } else {
      args.push(cliKey, String(value));
    }
  }
  return args;
}

function platformLauncher(url) {
  if (process.platform === "darwin") {
    return { command: "open", args: [url] };
  }
  if (process.platform === "win32") {
    return { command: "cmd", args: ["/c", "start", "", url] };
  }
  return { command: "xdg-open", args: [url] };
}

async function runCli(argv) {
  const { command, positional, args } = parseCliArgs(argv);
  let result;

  if (command === "next") {
    result = await callTool("workflow_next_task", args);
  } else if (command === "move") {
    result = await callTool("workflow_move_linear", {
      ...args,
      issue: args.issue || args.issue_id || args.task_id || positional[0],
    });
  } else if (command === "complete") {
    result = await callTool("workflow_complete_task", {
      ...args,
      task_id: args.task_id || args.issue || args.issue_id || positional[0],
    });
  } else if (command === "activity") {
    result = await callTool("workflow_update_activity", {
      ...args,
      task_id: args.task_id || args.issue || args.issue_id || args.task || positional[0],
    });
  } else if (command === "list") {
    result = await callTool("workflow_list_tasks", args);
  } else if (command === "pick") {
    result = await callTool("workflow_pick_task", {
      ...args,
      task_id: args.task_id || positional[0],
    });
  } else if (command === "render") {
    result = await callTool("workflow_render_prompt", {
      ...args,
      task_id: args.task_id || positional[0],
    });
  } else if (command === "validate") {
    result = await callTool("workflow_validate_workflow", args);
  } else if (command === "ui" || command === "open-ui") {
    result = await callTool("workflow_launch_ui", args);
  } else if (command === "begin-ui") {
    result = await callTool("workflow_launch_ui", {
      ...args,
      data: args.data || "next",
    });
  } else if (command === "ui-server") {
    await callTool("workflow_serve_ui", args);
    return;
  } else if (command === "help" || command === "--help" || command === "-h") {
    process.stdout.write(`${usage()}\n`);
    return;
  } else {
    throw new Error(`Unknown workflow command: ${command}\n\n${usage()}`);
  }

  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }

  process.stdout.write(formatCliResult(command, result));
}

function formatCliResult(command, result) {
  if (command === "move") {
    return [
      `Action: ${result.action}`,
      `Issue: ${result.issue.identifier}`,
      `State: ${result.issue.state}`,
      `URL: ${result.issue.url}`,
      "",
    ].join("\n");
  }

  if (command === "activity") {
    return [
      `Action: ${result.action}`,
      `Task: ${result.task.title}`,
      `Source: ${result.task.identifier}`,
      `Linear: ${result.issue.identifier || "unknown"}`,
      `Activity: ${result.activity.status}`,
      result.activity.owner ? `Owner: ${result.activity.owner}` : null,
      "",
    ]
      .filter((line) => line !== null)
      .join("\n");
  }

  if (command === "complete") {
    return [
      `Action: ${result.action}`,
      `Task: ${result.task.title}`,
      `Source: ${result.task.identifier}`,
      `Local: ${result.local.updated ? result.local.state : result.local.reason}`,
      result.linear ? `Linear: ${result.linear.issue.identifier} ${result.linear.issue.state}` : "Linear: skipped",
      "",
    ].join("\n");
  }

  if (command === "next" && Array.isArray(result.picked) && result.picked.length > 1) {
    return [
      `Action: ${result.action}`,
      `Picked: ${result.picked.length}`,
      result.duplicate_prevented ? "Duplicate prevention: skipped claimed tasks." : null,
      "",
      ...result.picked.flatMap((picked, index) => [
        `## Task ${index + 1}`,
        `Task: ${picked.task.title}`,
        `Source: ${picked.task.identifier}`,
        `Linear: ${picked.task.linear?.url || "not populated"}`,
        "",
        picked.prompt || "",
        "",
      ]),
    ]
      .filter((line) => line !== null)
      .join("\n");
  }

  if (command === "next" && result.action === "evaluate" && Array.isArray(result.candidates)) {
    const skipped = result.duplicate_prevented ? `\nSkipped claimed tasks: ${result.skipped_claimed_tasks.length}` : "";
    const stateLines = formatDecisionState(result.decision_state);
    const displayLimit = 15;
    const candidates = result.candidates;
    const totalCandidates = candidates.length;
    const shown = candidates.slice(0, displayLimit);
    const lines = [
      `Action: evaluate`,
      `${totalCandidates} unclaimed tasks available. Top candidates ranked by priority and dependency order.`,
      `Pick one with: node .agents/skills/workflow/scripts/workflow.js pick <task-id>`,
      totalCandidates > displayLimit ? `Showing top ${displayLimit} of ${totalCandidates} candidates.` : null,
      skipped,
      "",
      ...stateLines,
      stateLines.length > 0 ? "" : null,
      "---",
      "",
    ];

    let currentGroup = null;
    for (const candidate of shown) {
      const group = candidate.group || path.dirname(candidate.task_file);
      if (group !== currentGroup) {
        currentGroup = group;
        const label = groupLabel(group);
        const readiness = candidate.group_info ? groupReadinessLabel(candidate.group_info) : "";
        lines.push(`### Group: ${label} ${readiness ? `— ${readiness}` : ""}`);
      }

      const priority = (candidate.priority || "").toUpperCase();
      const prefix = priority ? `[${priority}] ` : "";
      lines.push(`Rank ${candidate.rank} | ${prefix}${candidate.title}`);
      lines.push(`        ID: ${candidate.id}`);
      lines.push(`        Source: ${candidate.identifier}`);
      if (candidate.requirements && candidate.requirements.length > 0) {
        lines.push(`        Requirements: ${candidate.requirements.join(", ")}`);
      }
      if (candidate.writes && candidate.writes.length > 0) {
        lines.push(`        Writes: ${candidate.writes.join(", ")}`);
      }
      lines.push("");
    }

    if (totalCandidates > displayLimit) {
      lines.push(`... and ${totalCandidates - displayLimit} more candidates.`);
      lines.push("");
    }

    return lines.join("\n");
  }

  if (command === "next" || command === "pick" || command === "render") {
    if (!result.task) {
      return `${result.reason || "No task selected."}\n${JSON.stringify(result, null, 2)}\n`;
    }
    return [
      `Action: ${result.action || "rendered"}`,
      `Task: ${result.task.title}`,
      `Source: ${result.task.identifier}`,
      `Linear: ${result.task.linear?.url || "not populated"}`,
      result.duplicate_prevented ? "Duplicate prevention: skipped claimed tasks." : null,
      "",
      result.prompt || "",
    ]
      .filter((line) => line !== null)
      .join("\n");
  }

  if (command === "list") {
    return `${JSON.stringify(result, null, 2)}\n`;
  }

  if (command === "ui" || command === "open-ui" || command === "begin-ui") {
    return [
      `Action: ${result.action}`,
      `Launched: ${result.launched ? "yes" : "no"}`,
      `Served: ${result.served ? "yes" : "no"}`,
      `Path: ${result.ui.path}`,
      `URL: ${result.ui.url}`,
      result.ui.data_url ? `Data: ${result.ui.data_url}` : null,
      result.server ? `Server: ${result.server.host}:${result.server.port} pid ${result.server.pid}` : null,
      "",
    ]
      .filter((line) => line !== null)
      .join("\n");
  }

  return `${JSON.stringify(result, null, 2)}\n`;
}

function formatDecisionState(state) {
  if (!state) return [];
  const lines = [`Decision state: ${state.path}${state.exists ? "" : " (missing)"}`];
  if (state.error) {
    lines.push(`State error: ${state.error}`);
    return lines;
  }
  if (state.recommendation) {
    const recommendation = state.recommendation;
    const match = recommendation.matches_active_task ? "active candidate" : "not in active candidates";
    lines.push(
      `Cached recommendation: ${recommendation.task_identifier || recommendation.task_id || "unspecified"} (${match})`,
    );
    if (recommendation.title) lines.push(`Title: ${recommendation.title}`);
    if (recommendation.rationale) lines.push(`Rationale: ${recommendation.rationale}`);
    if (recommendation.evidence.length > 0) {
      lines.push("Evidence:");
      for (const item of recommendation.evidence.slice(0, 5)) {
        lines.push(`- ${item}`);
      }
    }
  } else {
    lines.push("Cached recommendation: none");
  }
  if (state.task_activity && state.task_activity.length > 0) {
    lines.push("Task activity:");
    for (const activity of state.task_activity.slice(0, 5)) {
      const key = activity.task_identifier || activity.task_id || activity.linear_identifier || "unknown task";
      const owner = activity.owner ? ` by ${activity.owner}` : "";
      lines.push(`- ${key}: ${activity.status}${owner}`);
    }
  }
  if (state.relevant_task_notes.length > 0) {
    lines.push("Relevant task notes:");
    for (const note of state.relevant_task_notes.slice(0, 5)) {
      const key = note.task_identifier || note.task_id || "unknown task";
      const status = note.status ? ` (${note.status})` : "";
      const summary = note.summary || note.rationale || "";
      lines.push(`- ${key}${status}: ${summary}`);
    }
  }
  if (state.dependency_notes.length > 0) {
    lines.push("Dependency notes:");
    for (const note of state.dependency_notes.slice(0, 5)) {
      lines.push(`- ${note.summary || note.note || JSON.stringify(note)}`);
    }
  }
  return lines;
}

function usage() {
  return [
    "Usage: workflow.js [command] [options]",
    "",
    "Commands:",
    "  next                 Start the next unclaimed local task (default)",
    "  list                 List active local tasks",
    "  pick <task-id>       Pick or resume a specific task",
    "  render <task-id>     Render a prompt for a specific task",
    "  move <id>            Move a Linear issue or task's issue to another state",
    "  activity <id>        Mark claimed work active or inactive locally",
    "  complete <task-id>   Mark a validated task complete locally and in Linear",
    "  ui                   Open the populated local workflow task board",
    "  begin-ui             Opt-in mode: begin work and open the task board",
    "  validate             Validate WORKFLOW.md",
    "",
    "Options:",
    "  --json               Print machine-readable JSON",
    "  --limit <n>          Limit local tasks scanned",
    "  --count <n>          Pick multiple unclaimed tasks for parallel work",
    "  --attempt <n>        Render with an attempt number",
    "  --status <state>     Activity status for activity command: active|inactive",
    "  --owner <name>       Optional owner for activity records",
    "  --force              Take over an active claimed task when picking",
    "  --state-name <name>  Target Linear state for move",
    "  --state-id <id>      Target Linear state ID for move",
    "  --local-only         Complete only the local task checkbox",
    "  --no-local           Move Linear without editing the local task checkbox",
    "  --data <source>      UI data source: list, next, pick, or none",
    "  --host <host>        Host for the local UI server",
    "  --port <port>        Port for the local UI server",
    "  --task-id <id>       Task ID for --data pick",
    "  --no-open            Serve UI and print URL without opening a browser",
    "  --static             Resolve static UI file URL without serving data",
  ].join("\n");
}

if (require.main === module) {
  runCli(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error.message || String(error)}\n`);
    process.exit(1);
  });
}

module.exports = {
  activeTasks,
  callTool,
  findTaskFiles,
  buildUiPayload,
  completeTask,
  launchWorkflowUi,
  listTasks,
  parseTaskFile,
  parseWorkflow,
  runCli,
  renderTemplate,
  validateWorkflowContract,
};
