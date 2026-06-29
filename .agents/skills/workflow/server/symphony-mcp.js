#!/usr/bin/env node

const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const readline = require("node:readline");

const MCP_VERSION = "2025-06-18";
const GITHUB_GRAPHQL_URL = "https://api.github.com/graphql";

const DEFAULT_SETTINGS = {
  github_token: "$GITHUB_TOKEN",
  owner: "",
  project_number: 1,
  repository_path: "",
  workflow_path: "WORKFLOW.md",
  status_field: "Status",
  active_states: ["Todo", "In Progress"],
  terminal_states: ["Done", "Closed", "Cancelled", "Canceled", "Duplicate"],
};

const TOOLS = [
  {
    name: "symphony_validate_workflow",
    description:
      "Load and validate WORKFLOW.md using Symphony's workflow-file rules.",
    inputSchema: {
      type: "object",
      properties: {
        workflow_path: { type: "string" },
      },
    },
  },
  {
    name: "symphony_list_items",
    description:
      "List GitHub Projects items, optionally filtered to active Symphony states.",
    inputSchema: {
      type: "object",
      properties: {
        active_only: { type: "boolean", default: true },
        limit: { type: "integer", minimum: 1, maximum: 100, default: 20 },
      },
    },
  },
  {
    name: "symphony_render_prompt",
    description:
      "Render the Symphony prompt template for a specific GitHub Project item.",
    inputSchema: {
      type: "object",
      required: ["item_id"],
      properties: {
        item_id: { type: "string" },
        attempt: { type: ["integer", "null"] },
      },
    },
  },
  {
    name: "symphony_next_work_item",
    description:
      "Return the next active Project item with a rendered prompt for the current Zed agent.",
    inputSchema: {
      type: "object",
      properties: {
        claim_status: {
          type: "string",
          description:
            "Optional status to set before returning the work packet, for example In Progress.",
        },
      },
    },
  },
  {
    name: "symphony_update_item_status",
    description: "Move a GitHub Projects item to another single-select status.",
    inputSchema: {
      type: "object",
      required: ["item_id", "status"],
      properties: {
        item_id: { type: "string" },
        status: { type: "string" },
      },
    },
  },
];

function mergedSettings() {
  const raw = process.env.SYMPHONY_SETTINGS || "{}";
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    throw new Error(`SYMPHONY_SETTINGS is not valid JSON: ${error.message}`);
  }
  const base = { ...DEFAULT_SETTINGS, ...parsed };
  return { ...DEFAULT_SETTINGS, ...workflowSettings(base), ...parsed };
}

function workflowSettings(settings) {
  try {
    const workflow = parseWorkflow(workflowPath(settings));
    const tracker = workflow.config.tracker || {};
    const mapped = {};
    if (tracker.github_token || tracker.api_key) mapped.github_token = tracker.github_token || tracker.api_key;
    if (tracker.owner) mapped.owner = tracker.owner;
    if (tracker.project_number) mapped.project_number = tracker.project_number;
    if (tracker.status_field) mapped.status_field = tracker.status_field;
    if (Array.isArray(tracker.active_states)) mapped.active_states = tracker.active_states;
    if (Array.isArray(tracker.terminal_states)) mapped.terminal_states = tracker.terminal_states;
    return mapped;
  } catch (_error) {
    return {};
  }
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

function workflowPath(settings, override) {
  const configured = override || settings.workflow_path || "WORKFLOW.md";
  if (path.isAbsolute(configured)) return configured;
  const root = settings.repository_path || process.cwd();
  if (root.startsWith("~")) {
    return path.resolve(os.homedir(), root.slice(1), configured);
  }
  return path.resolve(root, configured);
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
    const yaml = text.slice(3 + newline.length, end);
    config = parseSimpleYamlMap(yaml);
    body = text.slice(end + marker.length);
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

function renderTemplate(template, issue, attempt) {
  const source = template || "You are working on an issue from GitHub Projects.";
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

function validateWorkflowContract(workflow) {
  const template = workflow.prompt_template || "";
  if (/{%[\s\S]*?%}/.test(template)) {
    throw new Error("workflow_validation_error: Liquid tag blocks are not supported; use {{ issue.field }} interpolation only");
  }
  const allowedIssueFields = new Set([
    "id",
    "type",
    "title",
    "body",
    "url",
    "number",
    "state",
    "repository",
    "status",
    "labels",
    "assignees",
    "fields",
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

async function github(settings, query, variables = {}) {
  const token = resolveEnvReference(settings.github_token, "github_token");
  const response = await fetch(GITHUB_GRAPHQL_URL, {
    method: "POST",
    headers: {
      authorization: `Bearer ${token}`,
      "content-type": "application/json",
      "user-agent": "symphony-zed-extension",
    },
    body: JSON.stringify({ query, variables }),
  });
  const payload = await response.json();
  if (!response.ok || payload.errors) {
    const details = payload.errors ? JSON.stringify(payload.errors) : response.statusText;
    throw new Error(`GitHub GraphQL request failed: ${details}`);
  }
  return payload.data;
}

async function project(settings) {
  const query = `
    query($owner: String!, $number: Int!) {
      organization(login: $owner) { projectV2(number: $number) { ...ProjectFields } }
      user(login: $owner) { projectV2(number: $number) { ...ProjectFields } }
    }
    fragment ProjectFields on ProjectV2 {
      id
      title
      fields(first: 50) {
        nodes {
          ... on ProjectV2SingleSelectField {
            id
            name
            options { id name }
          }
          ... on ProjectV2Field {
            id
            name
          }
        }
      }
    }`;
  const data = await github(settings, query, {
    owner: settings.owner,
    number: settings.project_number,
  });
  const project = data.organization?.projectV2 || data.user?.projectV2;
  if (!project) {
    throw new Error(`GitHub Project not found: ${settings.owner}/${settings.project_number}`);
  }
  return project;
}

async function listItems(settings, limit = 20) {
  const query = `
    query($owner: String!, $number: Int!, $limit: Int!) {
      organization(login: $owner) { projectV2(number: $number) { ...ProjectItems } }
      user(login: $owner) { projectV2(number: $number) { ...ProjectItems } }
    }
    fragment ProjectItems on ProjectV2 {
      id
      title
      items(first: $limit) {
        nodes {
          id
          type
          fieldValues(first: 50) {
            nodes {
              ... on ProjectV2ItemFieldSingleSelectValue {
                name
                field { ... on ProjectV2FieldCommon { id name } }
              }
              ... on ProjectV2ItemFieldTextValue {
                text
                field { ... on ProjectV2FieldCommon { id name } }
              }
              ... on ProjectV2ItemFieldDateValue {
                date
                field { ... on ProjectV2FieldCommon { id name } }
              }
            }
          }
          content {
            ... on DraftIssue { title body }
            ... on Issue {
              title body url number state
              repository { nameWithOwner }
              labels(first: 20) { nodes { name } }
              assignees(first: 10) { nodes { login } }
            }
            ... on PullRequest {
              title body url number state
              repository { nameWithOwner }
              labels(first: 20) { nodes { name } }
              assignees(first: 10) { nodes { login } }
            }
          }
        }
      }
    }`;
  const data = await github(settings, query, {
    owner: settings.owner,
    number: settings.project_number,
    limit,
  });
  const projectData = data.organization?.projectV2 || data.user?.projectV2;
  return (projectData?.items?.nodes || []).map((item) => normalizeItem(item, settings));
}

function normalizeItem(item, settings) {
  const fields = {};
  for (const field of item.fieldValues?.nodes || []) {
    if (!field?.field?.name) continue;
    fields[field.field.name] = field.name || field.text || field.date || null;
  }
  const content = item.content || {};
  return {
    id: item.id,
    type: item.type,
    title: content.title || "(untitled)",
    body: content.body || "",
    url: content.url || null,
    number: content.number || null,
    state: content.state || null,
    repository: content.repository?.nameWithOwner || null,
    status: fields[settings.status_field] || null,
    labels: (content.labels?.nodes || []).map((label) => label.name),
    assignees: (content.assignees?.nodes || []).map((assignee) => assignee.login),
    fields,
  };
}

async function updateStatus(settings, itemId, statusName) {
  const projectData = await project(settings);
  const statusField = projectData.fields.nodes.find(
    (field) => field?.name === settings.status_field && Array.isArray(field.options),
  );
  if (!statusField) {
    throw new Error(`Status field not found or not single-select: ${settings.status_field}`);
  }
  const option = statusField.options.find((candidate) => candidate.name === statusName);
  if (!option) {
    throw new Error(`Status option not found: ${statusName}`);
  }
  const mutation = `
    mutation($projectId: ID!, $itemId: ID!, $fieldId: ID!, $optionId: String!) {
      updateProjectV2ItemFieldValue(input: {
        projectId: $projectId
        itemId: $itemId
        fieldId: $fieldId
        value: { singleSelectOptionId: $optionId }
      }) {
        projectV2Item { id }
      }
    }`;
  await github(settings, mutation, {
    projectId: projectData.id,
    itemId,
    fieldId: statusField.id,
    optionId: option.id,
  });
  return { item_id: itemId, status: statusName };
}

async function callTool(name, args) {
  const settings = mergedSettings();
  if (name === "symphony_validate_workflow") {
    return {
      ...validateWorkflowContract(parseWorkflow(workflowPath(settings, args.workflow_path))),
      validation: { valid: true },
    };
  }
  validateSettings(settings);
  if (name === "symphony_list_items") {
    const items = await listItems(settings, args.limit || 20);
    return {
      items: args.active_only === false ? items : activeItems(settings, items),
    };
  }
  if (name === "symphony_render_prompt") {
    const items = await listItems(settings, 100);
    const item = items.find((candidate) => candidate.id === args.item_id);
    if (!item) throw new Error(`Project item not found: ${args.item_id}`);
    const workflow = parseWorkflow(workflowPath(settings));
    validateWorkflowContract(workflow);
    return {
      item,
      prompt: renderTemplate(workflow.prompt_template, item, args.attempt ?? null),
      workflow,
    };
  }
  if (name === "symphony_next_work_item") {
    const item = activeItems(settings, await listItems(settings, 100))[0];
    if (!item) return { item: null, prompt: null };
    if (args.claim_status) {
      await updateStatus(settings, item.id, args.claim_status);
      item.status = args.claim_status;
    }
    const workflow = parseWorkflow(workflowPath(settings));
    validateWorkflowContract(workflow);
    return {
      item,
      prompt: renderTemplate(workflow.prompt_template, item, null),
      workflow: { path: workflow.path, config: workflow.config },
    };
  }
  if (name === "symphony_update_item_status") {
    return updateStatus(settings, args.item_id, args.status);
  }
  throw new Error(`Unknown tool: ${name}`);
}

function activeItems(settings, items) {
  const active = new Set((settings.active_states || []).map((state) => state.toLowerCase()));
  const terminal = new Set((settings.terminal_states || []).map((state) => state.toLowerCase()));
  return items.filter((item) => {
    const status = (item.status || "").toLowerCase();
    if (active.size > 0) return active.has(status);
    return !terminal.has(status);
  });
}

function validateSettings(settings) {
  if (!settings.owner) throw new Error("Missing required setting: owner");
  if (!settings.project_number) throw new Error("Missing required setting: project_number");
  if (!settings.github_token) throw new Error("Missing required setting: github_token");
}

function result(id, value) {
  return {
    jsonrpc: "2.0",
    id,
    result: value,
  };
}

function error(id, err) {
  return {
    jsonrpc: "2.0",
    id,
    error: {
      code: -32000,
      message: err.message || String(err),
    },
  };
}

function toolContent(value) {
  return {
    content: [
      {
        type: "text",
        text: JSON.stringify(value, null, 2),
      },
    ],
  };
}

async function handle(message) {
  if (!message || !message.method) return null;
  if (message.method === "initialize") {
    return result(message.id, {
      protocolVersion: MCP_VERSION,
      serverInfo: { name: "symphony", version: "0.1.0" },
      capabilities: { tools: {} },
    });
  }
  if (message.method === "tools/list") {
    return result(message.id, { tools: TOOLS });
  }
  if (message.method === "tools/call") {
    const { name, arguments: args = {} } = message.params || {};
    return result(message.id, toolContent(await callTool(name, args)));
  }
  if (message.method === "notifications/initialized") return null;
  return error(message.id, new Error(`Unsupported method: ${message.method}`));
}

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

rl.on("line", async (line) => {
  if (!line.trim()) return;
  let message;
  try {
    message = JSON.parse(line);
  } catch (err) {
    process.stdout.write(`${JSON.stringify(error(null, err))}\n`);
    return;
  }
  try {
    const response = await handle(message);
    if (response) process.stdout.write(`${JSON.stringify(response)}\n`);
  } catch (err) {
    process.stdout.write(`${JSON.stringify(error(message.id, err))}\n`);
  }
});
