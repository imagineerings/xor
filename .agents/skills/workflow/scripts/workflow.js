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
  workflow_journal_path: ".agents/workflow-operations.json",
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
  claim_lease_minutes: 120,
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
    if (Number.isInteger(tracker.claim_lease_minutes)) mapped.claim_lease_minutes = tracker.claim_lease_minutes;
    if (tasks.glob) mapped.tasks_glob = tasks.glob;
    if (workflow.config.workflow_state?.path) mapped.workflow_state_path = workflow.config.workflow_state.path;
    if (workflow.config.workflow_journal?.path) mapped.workflow_journal_path = workflow.config.workflow_journal.path;

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

function defaultWorkflowOwner() {
  return (
    process.env.WORKFLOW_OWNER ||
    process.env.CODEX_THREAD_ID ||
    process.env.GITHUB_ACTOR ||
    `${process.env.USER || "codex"}@${os.hostname()}`
  );
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
  if (argv.length === 0 || argv.includes("--help") || argv.includes("-h")) {
    return { command: "help", positional: [], args: {} };
  }
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
  return { command: positional[0] || "help", positional: positional.slice(1), args };
}

function parseCliValue(value) {
  if (/^-?\d+$/.test(value)) return Number.parseInt(value, 10);
  if (value === "true") return true;
  if (value === "false") return false;
  if (value === "null") return null;
  return value;
}

function validateCliOptions(command, args) {
  const allowed = new Set([
    "active_only", "all", "allow_remote", "attempt", "count", "data", "dry_run", "file", "force", "host", "issue", "issue_id",
    "json", "lease_id", "lease_minutes", "limit", "no_open", "online", "open", "override_merge", "override_reason",
    "override_validation", "owner", "phase", "port", "print", "source", "state", "state_id", "state_name",
    "static", "status", "strict", "summary", "takeover", "task", "task_id", "ttl_ms", "ui_path",
    "validation_evidence", "verbose", "workflow_path",
  ]);
  const unknown = Object.keys(args).filter((key) => !allowed.has(key));
  if (unknown.length > 0) throw new Error(`Unknown option(s): ${unknown.map((key) => `--${key.replace(/_/g, "-")}`).join(", ")}`);
  if (args.force && command !== "init") throw new Error("--force is supported only by workflow init; use narrowly scoped override flags elsewhere.");
  for (const field of ["count", "limit", "lease_minutes"]) {
    if (args[field] != null && (!Number.isInteger(args[field]) || args[field] <= 0)) {
      throw new Error(`--${field.replace(/_/g, "-")} must be a positive integer.`);
    }
  }
}

function validateWorkflowContract(workflow) {
  const allowedConfigKeys = new Set(["tracker", "tasks", "workflow_state", "workflow_journal"]);
  const unknownConfigKeys = Object.keys(workflow.config).filter((key) => !allowedConfigKeys.has(key));
  if (unknownConfigKeys.length > 0) {
    throw new Error(`workflow_validation_error: unknown top-level field(s): ${unknownConfigKeys.join(", ")}`);
  }
  const tracker = workflow.config.tracker || {};
  const allowedTrackerKeys = new Set([
    "kind", "endpoint", "api_key", "team_id", "team_key", "project_id", "project_slug", "state_id",
    "label_ids", "active_states", "terminal_states", "resume_existing", "claim_lease_minutes",
  ]);
  const unknownTrackerKeys = Object.keys(tracker).filter((key) => !allowedTrackerKeys.has(key));
  if (unknownTrackerKeys.length > 0) {
    throw new Error(`workflow_validation_error: unknown tracker field(s): ${unknownTrackerKeys.join(", ")}`);
  }
  if (tracker.kind && tracker.kind !== "linear") {
    throw new Error(`workflow_validation_error: tracker.kind must be linear, got ${tracker.kind}`);
  }
  if (tracker.api_key && (typeof tracker.api_key !== "string" || !/^\$[A-Za-z_][A-Za-z0-9_]*$/.test(tracker.api_key))) {
    throw new Error("workflow_validation_error: tracker.api_key must be an environment reference such as $LINEAR_API_KEY");
  }
  if (tracker.claim_lease_minutes != null && (!Number.isInteger(tracker.claim_lease_minutes) || tracker.claim_lease_minutes <= 0)) {
    throw new Error("workflow_validation_error: tracker.claim_lease_minutes must be a positive integer");
  }
  for (const field of ["active_states", "terminal_states", "label_ids"]) {
    if (tracker[field] != null && !Array.isArray(tracker[field])) {
      throw new Error(`workflow_validation_error: tracker.${field} must be a list`);
    }
  }
  const tasks = workflow.config.tasks || {};
  const unknownTaskKeys = Object.keys(tasks).filter((key) => key !== "glob");
  if (unknownTaskKeys.length > 0) {
    throw new Error(`workflow_validation_error: unknown tasks field(s): ${unknownTaskKeys.join(", ")}`);
  }
  if (tasks.glob != null && typeof tasks.glob !== "string") {
    throw new Error("workflow_validation_error: tasks.glob must be a string");
  }
  for (const section of ["workflow_state", "workflow_journal"]) {
    const config = workflow.config[section] || {};
    const unknownKeys = Object.keys(config).filter((key) => key !== "path");
    if (unknownKeys.length > 0) {
      throw new Error(`workflow_validation_error: unknown ${section} field(s): ${unknownKeys.join(", ")}`);
    }
    if (config.path != null && typeof config.path !== "string") {
      throw new Error(`workflow_validation_error: ${section}.path must be a string`);
    }
  }
  if (!workflow.prompt_template?.trim()) {
    throw new Error("workflow_validation_error: prompt template must not be empty");
  }

  const template = workflow.prompt_template || "";
  if (/{%[\s\S]*?%}/.test(template)) {
    throw new Error(
      "workflow_validation_error: Liquid tag blocks are not supported; use {{ issue.field }} interpolation only",
    );
  }

  const allowedIssueFields = new Set([
    "id",
    "explicit_id",
    "aliases",
    "identifier",
    "title",
    "description",
    "priority",
    "value",
    "wave",
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
    "reads",
    "validation",
    "validation_evidence",
    "linear",
    "activity",
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

function publicWorkflow(workflow) {
  const tracker = { ...(workflow.config.tracker || {}) };
  if (tracker.api_key) {
    tracker.api_key = typeof tracker.api_key === "string" && tracker.api_key.startsWith("$") ? tracker.api_key : "<redacted>";
  }
  return {
    ...workflow,
    config: { ...workflow.config, tracker },
  };
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
  if (!/[?*]/.test(normalized)) {
    const filePath = path.resolve(root, normalized);
    return fs.existsSync(filePath) ? [filePath] : [];
  }

  const wildcardIndex = normalized.search(/[?*]/);
  const prefixEnd = normalized.lastIndexOf("/", wildcardIndex);
  const basePrefix = prefixEnd >= 0 ? normalized.slice(0, prefixEnd) : ".";
  const baseDir = path.resolve(root, basePrefix);
  if (!fs.existsSync(baseDir)) return [];

  const files = [];
  const matcher = globToRegExp(normalized);
  walkFiles(baseDir, (filePath) => {
    const relative = path.relative(root, filePath).replace(/\\/g, "/");
    if (matcher.test(relative)) files.push(filePath);
  });
  return files;
}

function globToRegExp(glob) {
  let pattern = "^";
  for (let index = 0; index < glob.length; index += 1) {
    const character = glob[index];
    if (character === "*" && glob[index + 1] === "*") {
      if (glob[index + 2] === "/") {
        pattern += "(?:.*/)?";
        index += 2;
      } else {
        pattern += ".*";
        index += 1;
      }
    } else if (character === "*") {
      pattern += "[^/]*";
    } else if (character === "?") {
      pattern += "[^/]";
    } else {
      pattern += character.replace(/[|\\{}()[\]^$+?.]/g, "\\$&");
    }
  }
  return new RegExp(`${pattern}$`);
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
    const requirements = splitMetadataValues(extractMetadata(current.bodyLines, "_Requirements:"));
    const writes = splitMetadataValues(extractMetadata(current.bodyLines, "_writes:"));
    const reads = splitMetadataValues(extractMetadata(current.bodyLines, "_reads:"));
    const validations = extractMetadata(current.bodyLines, "_validation:");
    const validationEvidence = extractMetadata(current.bodyLines, "_validation_evidence:");
    const explicitId = extractSingleMetadata(current.bodyLines, "_id:");
    const explicitPriority = extractSingleMetadata(current.bodyLines, "_priority:");
    const blockedBy = splitMetadataValues(extractMetadata(current.bodyLines, "_blocked_by:"));
    const waveValue = extractSingleMetadata(current.bodyLines, "_wave:");
    const value = extractSingleMetadata(current.bodyLines, "_value:");
    const description = current.bodyLines.join("\n").trim();
    const taskBody = [current.originalLine, ...current.bodyLines].join("\n").trim();
    const id = stableTaskId(relativePath, current.sequence, current.title, explicitId);
    const legacyId = legacyTaskId(relativePath, current.line, current.title);

    tasks.push({
      id,
      explicit_id: Boolean(explicitId),
      aliases: legacyId === id ? [] : [legacyId],
      identifier: `${relativePath}:${current.line}`,
      title: current.title,
      description,
      priority: normalizePriority(explicitPriority) || extractPriority(current.title),
      value: normalizeValue(value),
      wave: parseOptionalInteger(waveValue),
      state: markerState(current.marker),
      branch_name: null,
      url: null,
      labels: taskLabels(relativePath),
      blocked_by: blockedBy,
      created_at: null,
      updated_at: null,
      task_file: relativePath,
      task_line: current.line,
      task_body: taskBody,
      requirements,
      writes,
      reads,
      validation: validations,
      validation_evidence: validationEvidence,
      linear: emptyLinearIssue(),
      activity: { owner: null, expires_at: null, lease_id: null, status: null },
    });
    current = null;
  }

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const match = line.match(/^- \[([ xX~-])\]\s+(.+)$/);
    if (match) {
      finishCurrent();
      const rawTitle = match[2].trim();
      const sequenceMatch = rawTitle.match(/^(\d+(?:\.\d+)*)[\.)]\s*/);
      const title = rawTitle.replace(/^\d+(?:\.\d+)*[\.)]\s*/, "");
      current = {
        marker: match[1],
        title,
        sequence: sequenceMatch?.[1] || null,
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
  const normalizedPrefix = prefix.toLowerCase().replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`^\\s*-\\s+${normalizedPrefix}`, "i");
  for (const line of lines) {
    if (!pattern.test(line)) continue;
    const trimmed = line.trim();
    const start = trimmed.toLowerCase().indexOf(prefix.toLowerCase());
    const afterPrefix = trimmed.slice(start + prefix.length).trim();
    values.push(afterPrefix.replace(/^_+|_+$/g, "").trim());
  }
  return values;
}

function extractSingleMetadata(lines, prefix) {
  return extractMetadata(lines, prefix)[0] || null;
}

function splitMetadataValues(values) {
  return values
    .flatMap((value) => value.split(","))
    .map((value) => value.trim())
    .filter(Boolean);
}

function parseOptionalInteger(value) {
  if (value == null || value === "") return null;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

function normalizePriority(value) {
  const normalized = String(value || "").trim().toUpperCase();
  return /^P[0-4]$/.test(normalized) ? normalized : null;
}

function normalizeValue(value) {
  const normalized = String(value || "").trim().toLowerCase();
  return ["high", "medium", "low"].includes(normalized) ? normalized : null;
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

function stableTaskId(relativePath, sequence, title, explicitId) {
  if (explicitId) {
    const normalized = explicitId
      .toLowerCase()
      .replace(/[^a-z0-9._-]+/g, "-")
      .replace(/^-+|-+$/g, "");
    if (!normalized) throw new Error(`Invalid empty workflow task ID in ${relativePath}: ${title}`);
    return `task:${normalized}`;
  }
  const semanticKey = sequence ? `${relativePath}:sequence:${sequence}` : `${relativePath}:title:${title}`;
  const digest = crypto.createHash("sha1").update(semanticKey).digest("hex").slice(0, 12);
  return `task:${digest}`;
}

function legacyTaskId(relativePath, line, title) {
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
      task_activity: [],
      task_notes: [],
      dependency_notes: [],
      ranked_candidates: [],
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
      task_activity: [],
      task_notes: [],
      dependency_notes: [],
      ranked_candidates: [],
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
    lease_id: activity.lease_id || null,
    expires_at: activity.expires_at || null,
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
  atomicWriteJson(filePath, serializable);
}

function atomicWriteJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  const temporaryPath = `${filePath}.${process.pid}.${Date.now()}.tmp`;
  fs.writeFileSync(temporaryPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  fs.renameSync(temporaryPath, filePath);
}

function withFileLock(filePath, callback) {
  const lockPath = `${filePath}.lock`;
  const startedAt = Date.now();
  let descriptor = null;
  while (descriptor === null) {
    try {
      fs.mkdirSync(path.dirname(lockPath), { recursive: true });
      descriptor = fs.openSync(lockPath, "wx");
      fs.writeFileSync(descriptor, `${process.pid}\n`, "utf8");
    } catch (error) {
      if (error.code !== "EEXIST") throw error;
      try {
        if (Date.now() - fs.statSync(lockPath).mtimeMs > 30_000) fs.unlinkSync(lockPath);
      } catch (statError) {
        if (statError.code !== "ENOENT") throw statError;
      }
      if (Date.now() - startedAt > 5_000) throw new Error(`workflow_lock_timeout: ${lockPath}`);
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 25);
    }
  }
  try {
    return callback();
  } finally {
    fs.closeSync(descriptor);
    try {
      fs.unlinkSync(lockPath);
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
}

function workflowJournalPath(settings) {
  const configured = settings.workflow_journal_path || DEFAULT_SETTINGS.workflow_journal_path;
  return path.isAbsolute(configured) ? configured : path.resolve(repositoryRoot(settings), configured);
}

function loadWorkflowJournal(settings) {
  const filePath = workflowJournalPath(settings);
  if (!fs.existsSync(filePath)) return { version: 1, operations: [] };
  try {
    const value = JSON.parse(fs.readFileSync(filePath, "utf8"));
    return { version: 1, operations: Array.isArray(value.operations) ? value.operations : [] };
  } catch (error) {
    throw new Error(`workflow_journal_parse_error: ${error.message}`);
  }
}

function updateWorkflowOperation(settings, operationId, patch) {
  const filePath = workflowJournalPath(settings);
  return withFileLock(filePath, () => {
    const journal = loadWorkflowJournal(settings);
    const index = journal.operations.findIndex((operation) => operation.id === operationId);
    const now = new Date().toISOString();
    if (index === -1) {
      journal.operations.push({ id: operationId, created_at: now, updated_at: now, ...patch });
    } else {
      journal.operations[index] = { ...journal.operations[index], ...patch, updated_at: now };
    }
    journal.operations = journal.operations.slice(-100);
    atomicWriteJson(filePath, journal);
    return journal.operations.find((operation) => operation.id === operationId);
  });
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

  return withFileLock(workflowStatePath(settings), () => {
    const state = loadWorkflowState(settings);
    if (state.error) throw new Error(state.error);
    const existingIndex = (state.task_activity || []).findIndex((activity) => {
      return (
        (activity.task_id && activity.task_id === task.id) ||
        (activity.task_identifier && activity.task_identifier === task.identifier) ||
        (issue?.identifier && activity.linear_identifier === issue.identifier)
      );
    });
    const existing = existingIndex >= 0 ? state.task_activity[existingIndex] : null;
    const record = buildTaskActivity(settings, task, issue, normalizedStatus, options, existing);
    if (existingIndex >= 0) state.task_activity[existingIndex] = record;
    else state.task_activity = [...(state.task_activity || []), record];
    saveWorkflowState(settings, state);
    return record;
  });
}

function buildTaskActivity(settings, task, issue, normalizedStatus, options = {}, existing = null) {
  const now = new Date().toISOString();
  return {
    task_id: task.id,
    task_identifier: task.identifier,
    title: task.title,
    linear_identifier: issue?.identifier || task.linear?.identifier || null,
    linear_url: issue?.url || task.linear?.url || null,
    status: normalizedStatus,
    owner: options.owner ?? existing?.owner ?? null,
    summary: options.summary ?? existing?.summary ?? null,
    lease_id:
      normalizedStatus === "active"
        ? options.lease_id || (options.new_lease ? null : existing?.lease_id) || crypto.randomUUID()
        : options.lease_id || null,
    expires_at:
      normalizedStatus === "active"
        ? options.expires_at || new Date(Date.now() + Math.max(1, options.lease_minutes || settings.claim_lease_minutes || 120) * 60_000).toISOString()
        : null,
    updated_at: options.updated_at || now,
  };
}

function saveTaskActivity(settings, record) {
  return withFileLock(workflowStatePath(settings), () => {
    const state = loadWorkflowState(settings);
    if (state.error) throw new Error(state.error);
    const existingIndex = (state.task_activity || []).findIndex((activity) => {
      return (
        (activity.task_id && activity.task_id === record.task_id) ||
        (activity.task_identifier && activity.task_identifier === record.task_identifier) ||
        (record.linear_identifier && activity.linear_identifier === record.linear_identifier)
      );
    });
    if (existingIndex >= 0) state.task_activity[existingIndex] = record;
    else state.task_activity = [...(state.task_activity || []), record];
    saveWorkflowState(settings, state);
    return record;
  });
}

function claimedTaskStatus(settings, task, issue) {
  const linearActivity = parseLinearWorkflowActivity(issue?.description || "");
  if (linearActivity) {
    return {
      activity: linearActivity,
      source: "linear",
      is_active: isActivityActive(linearActivity),
      is_inactive: !isActivityActive(linearActivity),
    };
  }

  const state = loadWorkflowState(settings);
  const activity = state.error ? null : findTaskActivity(state, task, issue);
  return {
    activity,
    source: activity ? "workflow_state" : "missing",
    is_active: isActivityActive(activity),
    is_inactive: !isActivityActive(activity),
  };
}

function isActivityActive(activity) {
  if (!activity || activity.status !== "active") return false;
  if (!activity.expires_at) return true;
  const expiresAt = Date.parse(activity.expires_at);
  return Number.isFinite(expiresAt) && expiresAt > Date.now();
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
    if (key === "workflow.activity_lease_id") values.lease_id = value || null;
    if (key === "workflow.activity_expires_at") values.expires_at = value || null;
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
    lease_id: values.lease_id || null,
    expires_at: values.expires_at || null,
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
    lease_id: activity.lease_id || null,
    expires_at: activity.expires_at || null,
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
    ranked_candidates: state.ranked_candidates.slice(0, 15),
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
    return candidate.id === taskId || candidate.identifier === taskId || candidate.aliases?.includes(taskId);
  });
  if (!task) {
    throw new Error(`Local task not found: ${taskId}`);
  }
  return task;
}

function checkTaskConsistency(settings, task, phase = "start", options = {}) {
  const root = repositoryRoot(settings);
  const specDirectory = path.dirname(path.resolve(root, task.task_file));
  const requirementsPath = path.join(specDirectory, "requirements.md");
  const designPath = path.join(specDirectory, "design.md");
  const allTasks = listTasks(settings);
  const errors = [];
  const warnings = [];
  const checks = [];

  function record(name, passed, detail, severity = "error") {
    checks.push({ name, passed, detail, severity });
    if (!passed) (severity === "warning" ? warnings : errors).push(detail);
  }

  record(
    "requirements_file",
    fs.existsSync(requirementsPath),
    fs.existsSync(requirementsPath) ? path.relative(root, requirementsPath) : `Missing ${path.relative(root, requirementsPath)}`,
  );
  record(
    "design_file",
    fs.existsSync(designPath),
    fs.existsSync(designPath) ? path.relative(root, designPath) : `Missing ${path.relative(root, designPath)}`,
  );
  record(
    "writes_manifest",
    task.writes.length > 0,
    task.writes.length > 0 ? `${task.writes.length} declared write path(s)` : `Task ${task.id} has no _writes metadata.`,
    "warning",
  );

  const duplicateIds = allTasks.filter((candidate) => candidate.id === task.id);
  record(
    "durable_id_unique",
    duplicateIds.length === 1,
    duplicateIds.length === 1 ? `Task ID ${task.id} is unique.` : `Task ID ${task.id} is not unique.`,
  );

  const blockers = task.blocked_by.filter((dependency) => {
    const match = findTaskByAnyId(allTasks, dependency);
    return !match || match.state !== "Done";
  });
  record("dependencies", blockers.length === 0, blockers.length === 0 ? "All dependencies are complete." : `Unmet dependencies: ${blockers.join(", ")}`);

  if (fs.existsSync(requirementsPath) && task.requirements.length > 0) {
    const requirementsText = fs.readFileSync(requirementsPath, "utf8");
    const requirementReferences = parseRequirementReferences(requirementsText);
    const missingRequirements = task.requirements.filter(
      (requirement) => !requirementReferences.has(String(requirement).trim()),
    );
    record(
      "requirement_references",
      missingRequirements.length === 0,
      missingRequirements.length === 0 ? "All requirement references resolve." : `Unknown requirement references: ${missingRequirements.join(", ")}`,
    );
  }

  const state = loadWorkflowState(settings);
  if (!state.error) {
    const activeConflicts = (state.task_activity || [])
      .filter((activity) => isActivityActive(activity))
      .map((activity) => findTaskByAnyId(allTasks, activity.task_id || activity.task_identifier))
      .filter((candidate) => candidate && candidate.id !== task.id && tasksConflict(task, candidate));
    record(
      "active_write_conflicts",
      activeConflicts.length === 0,
      activeConflicts.length === 0 ? "No active write conflicts." : `Active write conflicts: ${activeConflicts.map((candidate) => candidate.id).join(", ")}`,
    );
  } else {
    record("workflow_state", false, state.error, "warning");
  }

  if (phase === "complete") {
    const validationEvidence = options.validation_evidence || options.evidence || null;
    record(
      "validation_evidence",
      Boolean(validationEvidence),
      validationEvidence || "Completion requires --validation-evidence <summary>; _validation is only the expected plan.",
    );
    record(
      "implementation_state",
      task.state !== "Done",
      task.state !== "Done" ? `Task ${task.id} is not yet complete.` : `Task ${task.id} is already complete.`,
      "warning",
    );
  }

  return {
    action: "checked",
    phase,
    task,
    passed: errors.length === 0,
    errors,
    warnings,
    checks,
  };
}

function parseRequirementReferences(text) {
  const references = new Set();
  let currentRequirement = null;
  let inAcceptanceCriteria = false;
  for (const line of String(text || "").split(/\r?\n/)) {
    const requirement = line.match(/^#{1,6}\s+Requirement\s+([A-Za-z0-9_-]+)(?:\s*:|\s|$)/i);
    if (requirement) {
      currentRequirement = requirement[1];
      inAcceptanceCriteria = false;
      references.add(currentRequirement);
      continue;
    }
    if (/^#{1,6}\s+Acceptance Criteria\s*$/i.test(line)) {
      inAcceptanceCriteria = true;
      continue;
    }
    if (/^#{1,6}\s+/.test(line)) {
      inAcceptanceCriteria = false;
      continue;
    }
    if (currentRequirement && inAcceptanceCriteria) {
      const criterion = line.match(/^\s*(\d+)[.)]\s+/);
      if (criterion) references.add(`${currentRequirement}.${criterion[1]}`);
    }
  }
  return references;
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
    if (value === null) return "Not set";
    if (Array.isArray(value)) return value.length > 0 ? value.join(", ") : "None";
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

async function populateLinear(settings, task, preliminaryPrompt, activity) {
  validateLinearSettings(settings);
  const teamId = await resolveLinearTeamId(settings);
  const projectId = await resolveLinearProjectId(settings);
  const description = linearIssueDescription(settings, task, preliminaryPrompt, activity);
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

function linearIssueDescription(settings, task, prompt, activity = null) {
  const description = [
    "Workflow picked this local Sim spec task.",
    "",
    workflowTaskMarker(task),
    ...(task.aliases || []).map((alias) => `workflow.local_task_alias:${alias}`),
    workflowSourceMarker(task),
    workflowRepositoryMarker(settings),
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
    "",
    "Expected reads:",
    formatList(task.reads),
    "",
    "Validation:",
    formatList(task.validation),
    "",
    "Rendered prompt:",
    prompt || "N/A",
  ].join("\n");
  return withLinearActivityMarkers(
    description,
    activity || {
      status: "inactive",
      updated_at: new Date().toISOString(),
      owner: null,
      summary: "Issue created before a claim was acquired.",
      lease_id: null,
      expires_at: null,
    },
  );
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
    "workflow.activity_lease_id": activity.lease_id || null,
    "workflow.activity_expires_at": activity.expires_at || null,
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
    return `ordering penalty: ${groupInfo.totalPrevious - groupInfo.donePrevious} previous task${groupInfo.totalPrevious - groupInfo.donePrevious !== 1 ? "s" : ""} unfinished`;
  return "unknown";
}

function rankCandidates(tasks, settings) {
  const state = loadWorkflowState(settings);
  const allTasks = listTasks(settings);
  const priorityOrder = { P0: 0, P1: 1, P2: 2, P3: 3, P4: 4 };
  const recommendationFresh = isRecommendationFresh(settings, state.recommendation);
  const scored = tasks.map((task) => {
    const previousTasks = allTasks.filter(
      (candidate) => candidate.task_file === task.task_file && candidate.task_line < task.task_line,
    );
    const donePrevious = previousTasks.filter((candidate) => candidate.state === "Done").length;
    const blockers = task.blocked_by.filter((dependency) => {
      const match = findTaskByAnyId(allTasks, dependency);
      return !match || match.state !== "Done";
    });
    const scoreParts = {
      priority: (priorityOrder[task.priority] ?? 5) * 20,
      value: task.value === "high" ? -20 : task.value === "low" ? 20 : 0,
      wave: task.wave == null ? 0 : task.wave * 5,
      previous: previousTasks.length === 0 || donePrevious === previousTasks.length ? 0 : 25,
      recommendation:
        recommendationFresh && findRecommendedTask([task], state.recommendation) ? -30 : 0,
      blocked: blockers.length > 0 ? 1000 : 0,
    };
    const score = Object.values(scoreParts).reduce((sum, value) => sum + value, 0);
    return {
      ...task,
      ready: blockers.length === 0,
      blockers,
      score,
      score_parts: scoreParts,
      rank_rationale: rankingRationale(task, blockers, previousTasks, donePrevious, recommendationFresh, state),
      group: path.dirname(task.task_file),
      group_info: {
        previousDone: previousTasks.length > 0 && donePrevious === previousTasks.length,
        previousPartial: donePrevious > 0 && donePrevious < previousTasks.length,
        previousNone: previousTasks.length === 0,
        totalPrevious: previousTasks.length,
        donePrevious,
      },
    };
  });

  return scored
    .sort((left, right) => {
      if (left.ready !== right.ready) return left.ready ? -1 : 1;
      if (left.score !== right.score) return left.score - right.score;
      const pathOrder = left.task_file.localeCompare(right.task_file);
      return pathOrder || left.task_line - right.task_line;
    })
    .map((task, index) => ({ ...task, rank: index + 1 }));
}

function findTaskByAnyId(tasks, value) {
  const normalized = String(value || "");
  return (
    tasks.find((task) => {
      return (
        task.id === normalized ||
        task.identifier === normalized ||
        task.aliases?.includes(normalized) ||
        task.id === `task:${normalized}`
      );
    }) || null
  );
}

function isRecommendationFresh(settings, recommendation) {
  if (!recommendation) return false;
  const updatedAt = Date.parse(recommendation.updated_at || "");
  if (!Number.isFinite(updatedAt)) return false;
  return !(recommendation.stale_if_changed || []).some((relativePath) => {
    const filePath = path.resolve(repositoryRoot(settings), relativePath);
    try {
      return fs.statSync(filePath).mtimeMs > updatedAt;
    } catch {
      return true;
    }
  });
}

function rankingRationale(task, blockers, previousTasks, donePrevious, recommendationFresh, state) {
  const reasons = [];
  if (task.priority) reasons.push(`priority ${task.priority}`);
  if (task.value) reasons.push(`${task.value} immediate value`);
  if (task.wave != null) reasons.push(`wave ${task.wave}`);
  if (blockers.length > 0) reasons.push(`blocked by ${blockers.join(", ")}`);
  if (previousTasks.length > 0) reasons.push(`${donePrevious}/${previousTasks.length} previous tasks done`);
  if (recommendationFresh && findRecommendedTask([task], state.recommendation)) reasons.push("fresh cached recommendation");
  return reasons.length > 0 ? reasons : ["deterministic source order"];
}

function normalizeManifestPath(value) {
  const normalized = String(value || "").replace(/\\/g, "/").replace(/^\.\//, "").replace(/\/$/, "");
  const wildcardIndex = normalized.search(/[?*\[]/);
  if (wildcardIndex === -1) return normalized;
  const directoryEnd = normalized.lastIndexOf("/", wildcardIndex);
  return directoryEnd >= 0 ? normalized.slice(0, directoryEnd) : ".";
}

function manifestPathsOverlap(leftPaths, rightPaths) {
  return leftPaths.some((leftPath) => {
    const normalizedLeft = normalizeManifestPath(leftPath);
    return rightPaths.some((rightPath) => {
      const normalizedRight = normalizeManifestPath(rightPath);
      return (
        normalizedLeft === normalizedRight ||
        normalizedLeft.startsWith(`${normalizedRight}/`) ||
        normalizedRight.startsWith(`${normalizedLeft}/`)
      );
    });
  });
}

function writePathsOverlap(left, right) {
  return manifestPathsOverlap(left.writes, right.writes);
}

function tasksConflict(left, right) {
  const dependencyConflict =
    left.blocked_by.some((value) => findTaskByAnyId([right], value)) ||
    right.blocked_by.some((value) => findTaskByAnyId([left], value));
  const manifestConflict =
    writePathsOverlap(left, right) ||
    manifestPathsOverlap(left.writes, right.reads || []) ||
    manifestPathsOverlap(right.writes, left.reads || []);
  return dependencyConflict || manifestConflict;
}

function selectCompatibleCandidates(ranked, count) {
  const selected = [];
  const rejected = [];
  for (const task of ranked) {
    if (!task.ready) {
      rejected.push({ task, reason: `blocked by ${task.blockers.join(", ")}` });
      continue;
    }
    const conflict = selected.find((candidate) => tasksConflict(task, candidate));
    if (conflict) {
      rejected.push({ task, reason: `conflicts with ${conflict.id}` });
      continue;
    }
    if (selected.length < count) selected.push(task);
  }
  return { selected, rejected };
}

function planTasks(settings, options = {}) {
  const tasks = activeTasks(settings, listTasks(settings));
  const ranked = rankCandidates(tasks, settings);
  const count = Math.max(1, options.count || 1);
  const compatibility = selectCompatibleCandidates(ranked, count);
  const displayLimit = Math.max(1, options.limit || 200);
  const serializeTask = options.verbose ? (task) => task : taskPlanSummary;
  return attachWorkflowState(
    settings,
    {
      action: "planned",
      recommended: compatibility.selected[0] ? serializeTask(compatibility.selected[0]) : null,
      selected: compatibility.selected.map(serializeTask),
      candidates: ranked.slice(0, displayLimit).map(serializeTask),
      rejected: compatibility.rejected.map(({ task, reason }) => ({ task_id: task.id, reason })),
      requested_count: count,
      selected_count: compatibility.selected.length,
      total_candidates: ranked.length,
      count,
      scoring: { direction: "lower_is_better", blocked_penalty: 1000 },
      read_only: true,
    },
    tasks,
  );
}

function taskPlanSummary(task) {
  return {
    id: task.id,
    explicit_id: task.explicit_id,
    identifier: task.identifier,
    task_file: task.task_file,
    task_line: task.task_line,
    title: task.title,
    priority: task.priority,
    value: task.value,
    wave: task.wave,
    state: task.state,
    blocked_by: task.blocked_by,
    requirements: task.requirements,
    reads: task.reads,
    writes: task.writes,
    validation: task.validation,
    validation_evidence: task.validation_evidence,
    ready: task.ready,
    blockers: task.blockers,
    score: task.score,
    score_parts: task.score_parts,
    rank_rationale: task.rank_rationale,
    group: task.group,
    group_info: task.group_info,
    rank: task.rank,
  };
}

function recordDecisionState(settings, selected, ranked, rationale) {
  withFileLock(workflowStatePath(settings), () => {
    const state = loadWorkflowState(settings);
    if (state.error) throw new Error(state.error);
    const now = new Date().toISOString();
    state.recommendation = selected
      ? {
          task_id: selected.id,
          task_identifier: selected.identifier,
          title: selected.title,
          rationale,
          evidence: selected.rank_rationale || [],
          stale_if_changed: [
            selected.task_file,
            path.join(path.dirname(selected.task_file), "requirements.md"),
            path.join(path.dirname(selected.task_file), "design.md"),
          ],
          updated_at: now,
        }
      : null;
    state.ranked_candidates = ranked.slice(0, 15).map((task) => ({
      task_id: task.id,
      task_identifier: task.identifier,
      rank: task.rank,
      score: task.score,
      ready: task.ready,
      blockers: task.blockers,
      rationale: task.rank_rationale,
    }));
    state.repo_revision = currentRevision(settings);
    saveWorkflowState(settings, state);
  });
}

function currentRevision(settings) {
  try {
    return childProcess.execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: repositoryRoot(settings),
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return null;
  }
}

async function pickNextTask(settings, workflow, tasks, attempt, options = {}) {
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
    if (!task.ready) continue;
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
      owner: options.owner,
      summary: options.summary,
      lease_minutes: options.lease_minutes,
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

  if (candidates.length >= 1) {
    const picked = await pickTask(settings, workflow, candidates[0], attempt, {
      existingChecked: true,
      markActive: true,
      owner: options.owner,
      summary: options.summary,
      lease_minutes: options.lease_minutes,
    });
    recordDecisionState(settings, candidates[0], ranked, "Selected the highest-ranked ready unclaimed task.");
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
    const reason = skippedClaimedTasks.length > 0
      ? "All ready local tasks are already claimed."
      : "No ready tasks found. Inspect `workflow plan --json` for blockers.";
    return {
      task: null,
      prompt: null,
      action: "none",
      duplicate_prevented: skippedClaimedTasks.length > 0,
      skipped_claimed_tasks: skippedClaimedTasks,
      reason,
    };
  }

  return {
    task: null,
    prompt: null,
    action: "none",
    duplicate_prevented: skippedClaimedTasks.length > 0,
    candidates: [],
    skipped_claimed_tasks: skippedClaimedTasks,
    reason: "No ready unclaimed tasks found. Inspect `workflow plan --json` for blockers.",
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

async function pickNextTasks(settings, workflow, tasks, attempt, count, options = {}) {
  const ranked = rankCandidates(tasks, settings);
  const pickedTasks = [];
  const skippedClaimedTasks = [];
  const rejectedConflicts = [];

  for (const task of ranked) {
    if (pickedTasks.length >= count) break;
    if (!task.ready) continue;
    const conflict = pickedTasks.find((picked) => tasksConflict(task, picked.task));
    if (conflict) {
      rejectedConflicts.push({ task_id: task.id, reason: `conflicts with ${conflict.task.id}` });
      continue;
    }
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
      owner: options.owner,
      summary: options.summary,
      lease_minutes: options.lease_minutes,
    });
    pickedTasks.push({
      task: picked.task,
      prompt: picked.prompt,
      action: picked.action,
      duplicate_prevented: skippedClaimedTasks.length > 0,
    });
  }

  if (pickedTasks.length > 0) {
    recordDecisionState(
      settings,
      pickedTasks[0].task,
      ranked,
      `Selected ${pickedTasks.length} compatible ready task${pickedTasks.length === 1 ? "" : "s"}.`,
    );
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
    rejected_conflicts: rejectedConflicts,
    reason: pickedTasks.length > 0 ? null : "No ready unclaimed compatible tasks found.",
  };
}

async function pickTask(settings, workflow, task, attempt, options = {}) {
  const gate = checkTaskConsistency(settings, task, "start", options);
  if (!gate.passed) {
    throw new Error(`workflow_start_gate_failed: ${gate.errors.join("; ")}`);
  }
  const linearConflicts = await findActiveLinearConflicts(settings, task);
  if (linearConflicts.length > 0) {
    throw new Error(
      `workflow_start_gate_failed: active Linear-backed conflicts: ${linearConflicts.map((conflict) => conflict.task.id).join(", ")}`,
    );
  }
  const existing =
    options.existingIssue ||
    (settings.resume_existing !== false && !options.existingChecked
      ? await findExistingLinearIssue(settings, task)
      : null);

  if (existing) {
    const claimed = claimedTaskStatus(settings, task, existing);
    if (claimed.is_active && !options.takeover) {
      return {
        task: null,
        prompt: null,
        action: "claimed_active",
        duplicate_prevented: true,
        reason:
          `Task ${task.identifier} is already claimed by ${existing.identifier || "a Linear issue"} ` +
          "and marked active in workflow state. Use takeover with an owner and reason to replace it.",
        skipped_claimed_tasks: [skippedClaimedTask(task, existing, claimed.activity)],
      };
    }

    attachLinearIssue(task, existing);
    let activeRecord = claimed.activity;
    if (options.markActive !== false) {
      const priorState = loadWorkflowState(settings);
      const priorActivity = priorState.error ? null : findTaskActivity(priorState, task, existing);
      const activity = buildTaskActivity(settings, task, existing, "active", {
        owner: options.owner || null,
        summary: options.summary || null,
        lease_minutes: options.lease_minutes || settings.claim_lease_minutes,
        new_lease: Boolean(options.takeover),
      }, priorActivity);
      await updateLinearIssueActivity(settings, existing, activity);
      saveTaskActivity(settings, activity);
      activeRecord = activity;
    }
    task.activity = activeRecord || null;
    return {
      task,
      prompt: renderTemplate(workflow.prompt_template, task, attempt),
      action: claimed.is_inactive ? "resumed_inactive" : "resumed",
      duplicate_prevented: true,
      gate,
    };
  }

  const preliminaryPrompt = renderTemplate(workflow.prompt_template, task, attempt);
  const provisionalActivity = buildTaskActivity(settings, task, null, "active", {
    owner: options.owner || null,
    summary: options.summary || null,
    lease_minutes: options.lease_minutes || settings.claim_lease_minutes,
    new_lease: true,
  });
  const issue = await populateLinear(settings, task, preliminaryPrompt, provisionalActivity);
  attachLinearIssue(task, issue);
  if (options.markActive !== false) {
    const activity = {
      ...provisionalActivity,
      linear_identifier: issue.identifier || null,
      linear_url: issue.url || null,
    };
    saveTaskActivity(settings, activity);
    task.activity = activity;
  }
  return {
    task,
    prompt: renderTemplate(workflow.prompt_template, task, attempt),
    action: "created",
    duplicate_prevented: false,
    gate,
  };
}

async function findActiveLinearConflicts(settings, task) {
  const candidates = activeTasks(settings, listTasks(settings)).filter(
    (candidate) => candidate.id !== task.id && tasksConflict(task, candidate),
  );
  const conflicts = [];
  for (const candidate of candidates) {
    const issue = await findExistingLinearIssue(settings, candidate);
    if (!issue) continue;
    const claimed = claimedTaskStatus(settings, candidate, issue);
    if (claimed.is_active) conflicts.push({ task: candidate, issue, activity: claimed.activity });
  }
  return conflicts;
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
  const terms = [task.id, ...(task.aliases || []), task.identifier];
  const results = await Promise.all(terms.map((term) => linear(settings, query, { term })));
  for (const data of results) {
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
      const repositoryMarker = workflowRepositoryMarker(settings);
      const repositoryMatches = !descriptionHasWorkflowMarkers(description, "workflow.repository:") || descriptionHasMarker(description, repositoryMarker);
      return (
        repositoryMatches &&
        (workflowTaskMarkers(task).some((marker) => descriptionHasMarker(description, marker)) ||
          descriptionHasMarker(description, sourceMarker))
      );
    }) || null
  );
}

function workflowTaskMarker(task) {
  return `workflow.local_task_id:${task.id}`;
}

function workflowTaskMarkers(task) {
  return [
    `workflow.local_task_id:${task.id}`,
    ...(task.aliases || []).flatMap((id) => [
      `workflow.local_task_alias:${id}`,
      `workflow.local_task_id:${id}`,
    ]),
  ];
}

function workflowSourceMarker(task) {
  return `workflow.local_task_source:${task.identifier}`;
}

function workflowRepositoryMarker(settings) {
  const root = repositoryRoot(settings);
  let identity = path.basename(root);
  try {
    identity = childProcess.execFileSync("git", ["config", "--get", "remote.origin.url"], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim() || identity;
  } catch (_error) {
    // A stable repository directory name is safer than an absolute machine-local path.
  }
  const digest = crypto.createHash("sha256").update(identity.toLowerCase()).digest("hex").slice(0, 16);
  return `workflow.repository:${digest}`;
}

function descriptionHasMarker(description, marker) {
  return String(description || "")
    .split(/\r?\n/)
    .some((line) => line.trim() === marker);
}

function descriptionHasWorkflowMarkers(description, prefix) {
  return String(description || "")
    .split(/\r?\n/)
    .some((line) => line.trim().startsWith(prefix));
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
    return candidate.id === key || candidate.identifier === key || candidate.aliases?.includes(key);
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
  const issue = (data.searchIssues?.nodes || []).find((candidate) => {
    return candidate.id === key || candidate.identifier === key || candidate.url === key;
  });
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
    const head = childProcess.execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    const originMain = childProcess.execFileSync("git", ["rev-parse", "--verify", "origin/main"], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    const status = childProcess.execFileSync("git", ["status", "--porcelain"], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    return head === originMain && status.length === 0;
  } catch {
    return null;
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
  let task = tasks.find(
    (candidate) => candidate.id === key || candidate.identifier === key || candidate.aliases?.includes(key),
  );
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
  const state = loadWorkflowState(settings);
  const previousActivity = state.error ? null : findTaskActivity(state, task, issue);
  const activity = buildTaskActivity(settings, task, issue, status, {
    owner: args.owner,
    summary: args.summary,
    lease_minutes: args.lease_minutes || settings.claim_lease_minutes,
    lease_id: args.lease_id,
    new_lease: Boolean(args.takeover),
  }, previousActivity);
  const updatedIssue = await updateLinearIssueActivity(settings, issue, activity);
  saveTaskActivity(settings, activity);
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
    return (
      workflowTaskMarkers(task).some((marker) => descriptionHasMarker(description, marker)) ||
      descriptionHasMarker(description, workflowSourceMarker(task))
    );
  });
}

function finishTask(settings, args) {
  const taskId = args.task_id || args.issue || args.issue_id;
  if (!taskId) {
    throw new Error("Missing task identifier. Pass finish <task-id>.");
  }
  const task = findTask(settings, taskId);
  const gate = checkTaskConsistency(settings, task, "complete", args);
  if (!gate.passed && !args.override_validation) {
    throw new Error(`workflow_completion_gate_failed: ${gate.errors.join("; ")}`);
  }
  if (args.override_validation && !args.override_reason) {
    throw new Error("--override-validation requires --override-reason <reason>.");
  }
  const evidence = updateLocalTaskValidationEvidence(settings, task, args.validation_evidence || args.override_reason);
  const local = updateLocalTaskCheckbox(settings, task, "Done");
  return {
    action: "finished",
    task: findTask(settings, task.id),
    local,
    evidence,
    gate,
    validation_evidence: evidence.evidence,
    override_reason: args.override_reason || null,
    next: "Commit the completed task packet with the implementation, land the PR, sync origin/main, then run workflow close.",
  };
}

async function closeTask(settings, args) {
  const taskId = args.task_id || args.issue || args.issue_id;
  if (!taskId) throw new Error("Missing task identifier. Pass close <task-id>.");
  if (args.override_merge && !args.override_reason) {
    throw new Error("--override-merge requires --override-reason <reason>.");
  }
  const task = findTask(settings, taskId);
  if (task.state !== "Done") {
    throw new Error(`Task ${task.id} is not finished locally. Run finish before committing and landing the PR.`);
  }
  if (task.validation_evidence.length === 0 && !args.override_validation) {
    throw new Error(`Task ${task.id} has no checked-in _validation_evidence. Run finish before merge.`);
  }
  if (args.override_validation && !args.override_reason) {
    throw new Error("--override-validation requires --override-reason <reason>.");
  }
  if (!args.override_merge) {
    const merged = checkMergedToMain(settings);
    if (merged !== true) {
      throw new Error(
        "Task cannot be closed because HEAD is not a clean, exact match for origin/main. " +
          "Fetch and switch to the merged main branch, or use --override-merge with --override-reason after manual verification.",
      );
    }
  }
  const operationId = crypto.randomUUID();
  const existingIssue = await findExistingLinearIssue(settings, task);
  if (!existingIssue) throw new Error(`No non-terminal Linear issue found for ${task.id}.`);
  updateWorkflowOperation(settings, operationId, {
    kind: "close",
    task_id: task.id,
    task_identifier: task.identifier,
    linear_identifier: existingIssue.identifier,
    target_state: args.state_name || args.state || "Done",
    validation_evidence: args.validation_evidence || task.validation_evidence.join("; ") || args.override_reason || null,
    override_reason: args.override_reason || null,
    status: "started",
    steps: [],
  });
  const linear = await moveLinearIssue(settings, {
    issue: existingIssue.identifier,
    state_name: args.state_name || args.state || "Done",
  });
  updateWorkflowOperation(settings, operationId, { status: "in_progress", steps: ["linear_moved"] });
  const priorState = loadWorkflowState(settings);
  const priorActivity = priorState.error ? null : findTaskActivity(priorState, task, linear.issue);
  const activity = buildTaskActivity(settings, task, linear.issue, "inactive", {
    owner: priorActivity?.owner,
    summary: "Closed after merge by workflow command.",
  }, priorActivity);
  await updateLinearIssueActivity(settings, linear.issue, activity);
  saveTaskActivity(settings, activity);
  updateWorkflowOperation(settings, operationId, {
    status: "completed",
    steps: ["linear_moved", "activity_released"],
  });
  return {
    action: "closed",
    task,
    linear,
    activity,
    operation_id: operationId,
  };
}

function completeTask(settings, args) {
  return finishTask(settings, args);
}

async function reconcileWorkflow(settings, args = {}) {
  const journal = loadWorkflowJournal(settings);
  const incomplete = journal.operations.filter((operation) => operation.status !== "completed");
  const results = [];
  for (const operation of incomplete) {
    if (args.task_id && operation.task_id !== args.task_id && operation.task_identifier !== args.task_id) continue;
    const steps = new Set(operation.steps || []);
    const result = { operation_id: operation.id, task_id: operation.task_id, actions: [], repaired: false };
    if (operation.kind !== "close") {
      result.actions.push("Unsupported operation kind; inspect manually.");
      results.push(result);
      continue;
    }

    const task = findTask(settings, operation.task_id || operation.task_identifier);
    if (!steps.has("linear_moved")) {
      result.actions.push("Move the linked Linear issue to Done.");
      if (!args.dry_run) {
        await moveLinearIssue(settings, {
          issue: operation.linear_identifier || task.id,
          state_name: operation.target_state || args.state_name || "Done",
        });
        steps.add("linear_moved");
        result.repaired = true;
      }
    }
    if (steps.has("linear_moved") && !steps.has("activity_released")) {
      result.actions.push("Release the Linear-backed activity lease.");
      if (!args.dry_run) {
        const issue = await resolveLinearIssueForMove(settings, { issue: operation.linear_identifier });
        const activity = buildTaskActivity(settings, task, issue, "inactive", {
          summary: "Recovered lease release after close.",
        });
        await updateLinearIssueActivity(settings, issue, activity);
        saveTaskActivity(settings, activity);
        steps.add("activity_released");
        result.repaired = true;
      }
    }
    if (!args.dry_run && steps.has("linear_moved") && steps.has("activity_released")) {
      updateWorkflowOperation(settings, operation.id, { status: "completed", steps: [...steps] });
    }
    results.push(result);
  }
  return { action: "reconciled", dry_run: Boolean(args.dry_run), incomplete: results };
}

function defaultWorkflowFile() {
  return `---
tracker:
  kind: linear
  api_key: $LINEAR_API_KEY
  team_key: $LINEAR_TEAM_KEY
  active_states: [Todo, In Progress]
  terminal_states: [Done, Closed, Canceled, Cancelled, Duplicate]
  claim_lease_minutes: 120
tasks:
  glob: .agents/specs/**/tasks.md
workflow_state:
  path: .agents/workflow-state.json
workflow_journal:
  path: .agents/workflow-operations.json
---
You are working on a local Sim spec task.

Task: {{ issue.title }}
Task ID: {{ issue.id }}
Source: {{ issue.task_file }}:{{ issue.task_line }}
Priority: {{ issue.priority }}
Blocked by: {{ issue.blocked_by }}
Requirements: {{ issue.requirements }}
Reads: {{ issue.reads }}
Writes: {{ issue.writes }}
Validation: {{ issue.validation }}
Owner: {{ issue.activity.owner }}
Lease expires: {{ issue.activity.expires_at }}

Task body:
{{ issue.task_body }}

Linear: {{ issue.linear.url }}

During work, renew the lease before it expires. If work stops, release it with
a concise summary. Before merge, run workflow finish with validation evidence.
After the merged main branch is synced locally, run workflow close.
`;
}

function initializeWorkflow(settings, args = {}) {
  const filePath = workflowPath(settings, args.workflow_path);
  if (fs.existsSync(filePath) && !args.force) {
    return { action: "unchanged", path: filePath, reason: "WORKFLOW.md already exists." };
  }
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, defaultWorkflowFile(), "utf8");
  const workflow = validateWorkflowContract(parseWorkflow(filePath));
  return { action: "initialized", path: filePath, validation: { valid: true }, workflow };
}

function doctorWorkflow(settings) {
  const checks = [];
  function add(name, passed, detail, severity = "error") {
    checks.push({ name, passed, detail, severity });
  }
  add("node", Number.parseInt(process.versions.node.split(".")[0], 10) >= 18, `Node ${process.versions.node}`);
  try {
    validateWorkflowContract(parseWorkflow(workflowPath(settings)));
    add("workflow", true, workflowPath(settings));
  } catch (error) {
    add("workflow", false, error.message);
  }
  let tasks = [];
  try {
    tasks = listTasks(settings);
    add("tasks", tasks.length > 0, `${tasks.length} task packets found`);
    const explicitCount = tasks.filter((task) => task.explicit_id).length;
    add(
      "durable_ids",
      explicitCount === tasks.length,
      `${explicitCount}/${tasks.length} tasks declare explicit _id metadata`,
      "warning",
    );
  } catch (error) {
    add("tasks", false, error.message);
  }
  try {
    const team = settings.linear_team_id || resolveEnvReference(settings.linear_team_key, "linear_team_key");
    if (!team) throw new Error("Configure tracker.team_id or tracker.team_key");
    add("linear_team", true, settings.linear_team_id ? "Configured by ID" : `Configured as ${team}`);
  } catch (error) {
    add("linear_team", false, error.message);
  }
  try {
    resolveEnvReference(settings.linear_api_key, "linear_api_key");
    add("linear_token", true, "Configured (not displayed)");
  } catch (error) {
    add("linear_token", false, error.message);
  }
  const revision = currentRevision(settings);
  add("git", Boolean(revision), revision || "Git revision unavailable");
  const errors = checks.filter((check) => !check.passed && check.severity === "error");
  const passed = (name) => checks.find((check) => check.name === name)?.passed === true;
  const capabilities = {
    can_plan: passed("node") && passed("workflow") && passed("tasks"),
    can_claim:
      passed("node") && passed("workflow") && passed("tasks") && passed("linear_team") && passed("linear_token"),
    can_complete:
      passed("node") && passed("workflow") && passed("tasks") && passed("linear_team") && passed("linear_token") && passed("git"),
  };
  return { action: "diagnosed", healthy: errors.length === 0, capabilities, checks };
}

async function doctorWorkflowOnline(settings) {
  const result = doctorWorkflow(settings);
  try {
    const teamId = await resolveLinearTeamId(settings);
    result.checks.push({ name: "linear_online", passed: true, detail: `Connected to team ${teamId}`, severity: "error" });
    if (settings.linear_project_id || settings.linear_project_slug) await resolveLinearProjectId(settings);
  } catch (error) {
    result.checks.push({ name: "linear_online", passed: false, detail: error.message, severity: "error" });
    result.healthy = false;
    result.capabilities.can_claim = false;
    result.capabilities.can_complete = false;
  }
  result.online = true;
  return result;
}

function lintTasks(settings, args = {}) {
  const tasks = listTasks(settings);
  const active = activeTasks(settings, tasks);
  const fields = [
    "explicit_id", "priority", "value", "wave", "blocked_by", "reads", "writes", "validation",
    "validation_evidence", "requirements",
  ];
  const coverage = Object.fromEntries(
    fields.map((field) => {
      const present = (task) => Array.isArray(task[field]) ? task[field].length > 0 : task[field] != null && task[field] !== false;
      return [field, {
        total: tasks.filter(present).length,
        active: active.filter(present).length,
        task_count: tasks.length,
        active_task_count: active.length,
      }];
    }),
  );
  const duplicateIds = [...new Set(tasks.map((task) => task.id).filter((id, index, ids) => ids.indexOf(id) !== index))];
  const diagnostics = [];
  for (const task of active) {
    if (!task.explicit_id) diagnostics.push({ severity: args.strict ? "error" : "warning", task: task.identifier, message: "Missing explicit _id metadata." });
    if (task.writes.length === 0) diagnostics.push({ severity: "error", task: task.identifier, message: "Missing _writes metadata." });
    if (task.validation.length === 0) diagnostics.push({ severity: args.strict ? "error" : "warning", task: task.identifier, message: "Missing _validation plan." });
    for (const manifestPath of [...task.reads, ...task.writes]) {
      if (path.isAbsolute(manifestPath) || manifestPath.startsWith("~") || manifestPath.split("/").includes("..")) {
        diagnostics.push({ severity: "error", task: task.identifier, message: `Manifest path must be repository-relative: ${manifestPath}` });
      }
    }
    const requirementsPath = path.resolve(repositoryRoot(settings), path.dirname(task.task_file), "requirements.md");
    if (fs.existsSync(requirementsPath)) {
      const references = parseRequirementReferences(fs.readFileSync(requirementsPath, "utf8"));
      for (const requirement of task.requirements) {
        if (!references.has(requirement)) diagnostics.push({ severity: "error", task: task.identifier, message: `Unknown requirement reference: ${requirement}` });
      }
    }
  }
  for (const id of duplicateIds) diagnostics.push({ severity: "error", task: id, message: "Duplicate task ID." });
  const errors = diagnostics.filter((diagnostic) => diagnostic.severity === "error");
  return {
    action: "linted",
    passed: errors.length === 0,
    strict: Boolean(args.strict),
    task_count: tasks.length,
    active_task_count: active.length,
    coverage,
    diagnostics,
  };
}

function suggestTaskIds(settings) {
  const tasks = listTasks(settings).filter((task) => !task.explicit_id);
  const used = new Set(listTasks(settings).filter((task) => task.explicit_id).map((task) => task.id));
  const suggestions = tasks.map((task) => {
    const spec = path.basename(path.dirname(task.task_file));
    const base = `${spec}-${task.title}`
      .toLowerCase()
      .replace(/[^a-z0-9._-]+/g, "-")
      .replace(/^-+|-+$/g, "");
    let id = base || "task";
    let suffix = 2;
    while (used.has(`task:${id}`)) {
      id = `${base}-${suffix}`;
      suffix += 1;
    }
    used.add(`task:${id}`);
    return {
      task: task.identifier,
      current_id: task.id,
      suggested_id: id,
      metadata: `  - _id: ${id}_`,
    };
  });
  return {
    action: "suggested_ids",
    read_only: true,
    count: suggestions.length,
    warning: "Add IDs before first claim. For claimed tasks, preserve prior markers and coordinate a tracker migration.",
    suggestions,
  };
}

function updateLocalTaskCheckbox(settings, task, stateName) {
  const root = repositoryRoot(settings);
  const filePath = path.resolve(root, task.task_file);
  const currentTasks = parseTaskFile(filePath, root).filter((candidate) => candidate.id === task.id);
  if (currentTasks.length !== 1) {
    throw new Error(`Task ${task.id} could not be uniquely located before updating ${task.task_file}.`);
  }
  const currentTask = currentTasks[0];
  const text = fs.readFileSync(filePath, "utf8");
  const newline = text.includes("\r\n") ? "\r\n" : "\n";
  const lines = text.split(/\r?\n/);
  const lineIndex = currentTask.task_line - 1;
  const line = lines[lineIndex];
  if (line === undefined) {
    throw new Error(`Task source line not found: ${task.identifier}`);
  }

  const marker = localMarkerForState(stateName);
  const currentMarker = line.match(/^- \[([ xX~-])\]/)?.[1];
  if (currentMarker && markerState(currentMarker) === markerState(marker)) {
    return {
      updated: false,
      state: stateName,
      task_file: currentTask.task_file,
      task_line: currentTask.task_line,
      before: line,
      after: line,
      reason: "Task checkbox already has the requested state.",
    };
  }
  const updatedLine = line.replace(/^- \[[ xX~-]\]/, `- [${marker}]`);
  if (updatedLine === line) {
    throw new Error(`Task source line is not a top-level checkbox: ${task.identifier}`);
  }

  lines[lineIndex] = updatedLine;
  const temporaryPath = `${filePath}.${process.pid}.${Date.now()}.tmp`;
  fs.writeFileSync(temporaryPath, lines.join(newline), "utf8");
  fs.renameSync(temporaryPath, filePath);
  return {
    updated: true,
    state: stateName,
    task_file: currentTask.task_file,
    task_line: currentTask.task_line,
    before: line,
    after: updatedLine,
  };
}

function updateLocalTaskValidationEvidence(settings, task, evidence) {
  const summary = String(evidence || "").replace(/\s+/g, " ").trim();
  if (!summary) throw new Error("Validation evidence must not be empty.");
  const root = repositoryRoot(settings);
  const filePath = path.resolve(root, task.task_file);
  const currentTasks = parseTaskFile(filePath, root).filter((candidate) => candidate.id === task.id);
  if (currentTasks.length !== 1) throw new Error(`Task ${task.id} could not be uniquely located before recording evidence.`);
  const currentTask = currentTasks[0];
  const text = fs.readFileSync(filePath, "utf8");
  const newline = text.includes("\r\n") ? "\r\n" : "\n";
  const lines = text.split(/\r?\n/);
  const start = currentTask.task_line;
  let end = lines.length;
  for (let index = start; index < lines.length; index += 1) {
    if (/^- \[[ xX~-]\]\s+/.test(lines[index]) || (lines[index].trim() && !/^\s/.test(lines[index]))) {
      end = index;
      break;
    }
  }
  const existingIndex = lines.findIndex((line, index) => {
    return index >= start && index < end && /^\s*-\s+_validation_evidence:/i.test(line.trim());
  });
  const evidenceLine = `  - _validation_evidence: ${summary}_`;
  if (existingIndex >= 0) lines[existingIndex] = evidenceLine;
  else lines.splice(end, 0, evidenceLine);
  const temporaryPath = `${filePath}.${process.pid}.${Date.now()}.tmp`;
  fs.writeFileSync(temporaryPath, lines.join(newline), "utf8");
  fs.renameSync(temporaryPath, filePath);
  return { updated: true, task_file: currentTask.task_file, evidence: summary };
}

function localMarkerForState(stateName) {
  const normalized = String(stateName || "").toLowerCase();
  if (["done", "closed", "complete", "completed"].includes(normalized)) return "x";
  if (["todo", "backlog", "open"].includes(normalized)) return " ";
  return "~";
}

async function callTool(name, args = {}) {
  const settings = parseSettings();
  if (name === "workflow_init") {
    return initializeWorkflow(settings, args);
  }

  if (name === "workflow_doctor") {
    return args.online ? doctorWorkflowOnline(settings) : doctorWorkflow(settings);
  }

  const validatedWorkflow = validateWorkflowContract(parseWorkflow(workflowPath(settings, args.workflow_path)));

  if (name === "workflow_lint") {
    return lintTasks(settings, args);
  }

  if (name === "workflow_suggest_ids") {
    return suggestTaskIds(settings);
  }

  if (name === "workflow_plan") {
    return planTasks(settings, args);
  }

  if (name === "workflow_check") {
    const task = findTask(settings, args.task_id);
    if (!["start", "complete"].includes(args.phase || "start")) {
      throw new Error("Unsupported check phase. Use --phase start or --phase complete.");
    }
    return checkTaskConsistency(settings, task, args.phase || "start", args);
  }

  if (name === "workflow_reconcile") {
    return reconcileWorkflow(settings, args);
  }

  if (name === "workflow_launch_ui") {
    return launchWorkflowUi(settings, args);
  }

  if (name === "workflow_serve_ui") {
    return serveWorkflowUi(settings, args);
  }

  if (name === "workflow_validate_workflow") {
    const workflow = validatedWorkflow;
    return {
      workflow: publicWorkflow(workflow),
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
      workflow: { path: workflow.path, config: publicWorkflow(workflow).config },
    };
  }

  if (name === "workflow_pick_task") {
    if (args.takeover && (!args.owner || !args.override_reason)) {
      throw new Error("takeover requires --owner <name> and --override-reason <reason>.");
    }
    const task = findTask(settings, args.task_id);
    const workflow = validateWorkflowContract(parseWorkflow(workflowPath(settings)));
    const picked = await pickTask(settings, workflow, task, args.attempt ?? null, {
      takeover: Boolean(args.takeover),
      owner: args.owner || defaultWorkflowOwner(),
      summary: args.summary || null,
      lease_minutes: args.lease_minutes || settings.claim_lease_minutes,
    });
    return {
      task: picked.task,
      prompt: picked.prompt,
      duplicate_prevented: picked.duplicate_prevented,
      action: picked.action,
      reason: picked.reason || null,
      skipped_claimed_tasks: picked.skipped_claimed_tasks || [],
      gate: picked.gate || null,
      workflow: { path: workflow.path, config: publicWorkflow(workflow).config },
    };
  }

  if (name === "workflow_next_task") {
    const workflow = validateWorkflowContract(parseWorkflow(workflowPath(settings)));
    const tasks = activeTasks(settings, listTasks(settings));
    const count = Math.max(1, args.count || 1);
    const claimOptions = {
      owner: args.owner || defaultWorkflowOwner(),
      summary: args.summary || null,
      lease_minutes: args.lease_minutes || settings.claim_lease_minutes,
    };
    const picked =
      count === 1
        ? await pickNextTask(settings, workflow, tasks, args.attempt ?? null, claimOptions)
        : await pickNextTasks(settings, workflow, tasks, args.attempt ?? null, count, claimOptions);
    return {
      ...attachWorkflowState(settings, picked, tasks),
      workflow: { path: workflow.path, config: publicWorkflow(workflow).config },
    };
  }

  if (name === "workflow_move_linear") {
    return moveLinearIssue(settings, args);
  }

  if (name === "workflow_complete_task") {
    return completeTask(settings, args);
  }

  if (name === "workflow_finish_task") {
    return finishTask(settings, args);
  }

  if (name === "workflow_close_task") {
    return closeTask(settings, args);
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
  validateUiHost(host, args.allow_remote);
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
    allow_remote: args.allow_remote,
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
  validateUiHost(host, args.allow_remote);
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

function validateUiHost(host, allowRemote) {
  const loopback = new Set(["127.0.0.1", "localhost", "::1"]);
  if (!loopback.has(host) && !allowRemote) {
    throw new Error("Refusing to expose workflow task data on a non-loopback host without --allow-remote.");
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
    const tasks = activeTasks(settings, listTasks(settings));
    const count = Math.max(1, args.count || 1);
    const claimOptions = {
      owner: args.owner || defaultWorkflowOwner(),
      summary: args.summary || null,
      lease_minutes: args.lease_minutes || settings.claim_lease_minutes,
    };
    const picked =
      count === 1
        ? await pickNextTask(settings, workflow, tasks, args.attempt ?? null, claimOptions)
        : await pickNextTasks(settings, workflow, tasks, args.attempt ?? null, count, claimOptions);
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
    const picked = await pickTask(settings, workflow, task, args.attempt ?? null, {
      owner: args.owner || defaultWorkflowOwner(),
      summary: args.summary || null,
      lease_minutes: args.lease_minutes || settings.claim_lease_minutes,
    });
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
  return attachWorkflowState(settings, {
      tasks: filtered.slice(0, args.limit || 500),
      total: filtered.length,
      ui: { data_source: dataSource, generated_at: new Date().toISOString() },
    }, filtered);
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
  validateCliOptions(command, args);
  let result;

  if (command === "next") {
    result = await callTool("workflow_next_task", args);
  } else if (command === "plan") {
    result = await callTool("workflow_plan", args);
  } else if (command === "init") {
    result = await callTool("workflow_init", args);
  } else if (command === "doctor") {
    result = await callTool("workflow_doctor", args);
  } else if (command === "lint") {
    result = await callTool("workflow_lint", args);
  } else if (command === "migrate-ids") {
    result = await callTool("workflow_suggest_ids", args);
  } else if (command === "check") {
    result = await callTool("workflow_check", {
      ...args,
      task_id: args.task_id || positional[0],
      phase: args.phase || positional[1] || "start",
    });
  } else if (command === "reconcile") {
    result = await callTool("workflow_reconcile", {
      ...args,
      task_id: args.task_id || positional[0],
    });
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
  } else if (command === "finish") {
    result = await callTool("workflow_finish_task", {
      ...args,
      task_id: args.task_id || args.issue || args.issue_id || positional[0],
    });
  } else if (command === "close" || command === "finalize") {
    result = await callTool("workflow_close_task", {
      ...args,
      task_id: args.task_id || args.issue || args.issue_id || positional[0],
    });
  } else if (command === "activity") {
    result = await callTool("workflow_update_activity", {
      ...args,
      task_id: args.task_id || args.issue || args.issue_id || args.task || positional[0],
    });
  } else if (command === "renew" || command === "release") {
    result = await callTool("workflow_update_activity", {
      ...args,
      task_id: args.task_id || positional[0],
      status: command === "renew" ? "active" : "inactive",
    });
  } else if (command === "list") {
    result = await callTool("workflow_list_tasks", { ...args, active_only: args.all ? false : args.active_only });
  } else if (command === "pick") {
    result = await callTool("workflow_pick_task", {
      ...args,
      task_id: args.task_id || positional[0],
    });
  } else if (command === "takeover") {
    result = await callTool("workflow_pick_task", {
      ...args,
      task_id: args.task_id || positional[0],
      takeover: true,
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
  } else if (command === "help") {
    process.stdout.write(`${usage()}\n`);
    return;
  } else {
    throw new Error(`Unknown workflow command: ${command}\n\n${usage()}`);
  }

  if (args.json) {
    process.stdout.write(`${JSON.stringify({ schema_version: 2, ...result }, null, 2)}\n`);
    if (result?.passed === false || result?.healthy === false) process.exitCode = 1;
    return;
  }

  process.stdout.write(formatCliResult(command, result));
  if (result?.passed === false || result?.healthy === false) process.exitCode = 1;
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

  if (command === "complete" || command === "finish") {
    return [
      `Action: ${result.action}`,
      `Task: ${result.task.title}`,
      `Source: ${result.task.identifier}`,
      `Local: ${result.local.updated ? result.local.state : result.local.reason}`,
      result.next ? `Next: ${result.next}` : null,
      "",
    ].filter((line) => line !== null).join("\n");
  }

  if (command === "close" || command === "finalize") {
    return [
      `Action: ${result.action}`,
      `Task: ${result.task.title}`,
      `Linear: ${result.linear.issue.identifier} ${result.linear.issue.state}`,
      `Activity: ${result.activity.status}`,
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
        picked.task.activity?.owner ? `Owner: ${picked.task.activity.owner}` : null,
        picked.task.activity?.expires_at ? `Lease expires: ${picked.task.activity.expires_at}` : null,
        picked.task.activity?.expires_at ? `Renew: .agents/skills/workflow/scripts/workflow renew ${picked.task.id}` : null,
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
      "Pick one with: .agents/skills/workflow/scripts/workflow pick <task-id>",
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
      result.task.activity?.owner ? `Owner: ${result.task.activity.owner}` : null,
      result.task.activity?.expires_at ? `Lease expires: ${result.task.activity.expires_at}` : null,
      result.task.activity?.expires_at ? `Renew: .agents/skills/workflow/scripts/workflow renew ${result.task.id}` : null,
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
    "Usage: .agents/skills/workflow/scripts/workflow [command] [options]",
    "",
    "Commands:",
    "  init                 Create a minimal WORKFLOW.md when missing",
    "  doctor               Check local runtime and workflow readiness",
    "  lint                 Audit task metadata; add --strict for required fields",
    "  migrate-ids          Suggest durable IDs without editing task files",
    "  plan                 Rank ready tasks without tracker writes",
    "  next                 Start the next unclaimed local task",
    "  list                 List active local tasks",
    "  pick <task-id>       Pick or resume a specific task",
    "  render <task-id>     Render a prompt for a specific task",
    "  move <id>            Move a Linear issue or task's issue to another state",
    "  activity <id>        Mark claimed work active or inactive locally",
    "  renew <id>           Renew an active task lease",
    "  release <id>         Release a task lease for handoff",
    "  takeover <id>        Explicitly take over an active task lease",
    "  check <id>           Run a start or completion consistency gate",
    "  finish <task-id>     Validate and mark a task done before merge",
    "  close <task-id>      After merge, close Linear and release the lease",
    "  complete <task-id>   Compatibility alias for finish",
    "  reconcile [task-id]  Inspect or repair interrupted completion operations",
    "  ui                   Open the populated local workflow task board",
    "  begin-ui             Opt-in mode: begin work and open the task board",
    "  validate             Validate WORKFLOW.md",
    "",
    "Options:",
    "  --json               Print machine-readable JSON",
    "  --limit <n>          Limit candidates displayed without changing ranking",
    "  --verbose            Include full task packets in plan output",
    "  --all                Include completed packets when listing tasks",
    "  --online             Verify Linear credentials and routing during doctor",
    "  --count <n>          Pick multiple unclaimed tasks for parallel work",
    "  --attempt <n>        Render with an attempt number",
    "  --status <state>     Activity status for activity command: active|inactive",
    "  --owner <name>       Optional owner for activity records",
    "  --lease-minutes <n>  Claim lease duration (default: 120 minutes)",
    "  --phase <phase>      Consistency gate phase: start|complete",
    "  --validation-evidence <summary>  Validation performed for completion",
    "  --dry-run            Show reconciliation actions without applying them",
    "  --override-validation  Bypass validation evidence only with a reason",
    "  --override-merge     Bypass merge verification only with a reason",
    "  --override-reason <reason>  Required explanation for an override or takeover",
    "  --state-name <name>  Target Linear state for move",
    "  --state-id <id>      Target Linear state ID for move",
    "  --data <source>      UI data source: list, next, pick, or none",
    "  --host <host>        Host for the local UI server",
    "  --allow-remote       Explicitly expose the UI on a non-loopback host",
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
  checkTaskConsistency,
  doctorWorkflow,
  finishTask,
  closeTask,
  lintTasks,
  initializeWorkflow,
  isActivityActive,
  launchWorkflowUi,
  listTasks,
  parseTaskFile,
  planTasks,
  reconcileWorkflow,
  parseWorkflow,
  runCli,
  renderTemplate,
  selectCompatibleCandidates,
  tasksConflict,
  updateWorkflowOperation,
  validateWorkflowContract,
};
