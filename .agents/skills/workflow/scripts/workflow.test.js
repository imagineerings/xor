const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const childProcess = require("node:child_process");
const test = require("node:test");

const {
  callTool,
  checkTaskConsistency,
  finishTask,
  initializeWorkflow,
  isActivityActive,
  listTasks,
  parseTaskFile,
  planTasks,
  renderTemplate,
  tasksConflict,
  updateWorkflowOperation,
} = require("./workflow.js");

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "workflow-test-"));
  const specDirectory = path.join(root, ".agents", "specs", "feature");
  fs.mkdirSync(specDirectory, { recursive: true });
  fs.writeFileSync(
    path.join(specDirectory, "requirements.md"),
    "# Requirements\n\n### Requirement 1: Foundation\n\n#### Acceptance Criteria\n\n1. Foundation works\n2. Foundation remains stable\n\n### Requirement 2: Follow-up\n\n#### Acceptance Criteria\n\n1. Follow-up works\n",
  );
  fs.writeFileSync(path.join(specDirectory, "design.md"), "# Design\n");
  const settings = {
    repository_path: root,
    tasks_glob: ".agents/specs/**/tasks.md",
    workflow_path: "WORKFLOW.md",
    workflow_state_path: ".agents/workflow-state.json",
    workflow_journal_path: ".agents/workflow-operations.json",
    active_states: ["Todo", "In Progress"],
    terminal_states: ["Done"],
    claim_lease_minutes: 120,
  };
  return { root, specDirectory, settings };
}

test("parses durable metadata and preserves IDs when line numbers move", () => {
  const { root, specDirectory } = fixture();
  const taskFile = path.join(specDirectory, "tasks.md");
  const body = `- [ ] 1. Implement foundation
  - _id: feature-foundation_
  - _priority: P1_
  - _value: high_
  - _wave: 2_
  - _blocked_by: another-task, third-task_
  - _reads: crates/a.rs_
  - _writes: crates/b.rs, crates/c.rs_
  - _validation: cargo test -p feature_
  - _Requirements: 1.1, 1.2_
`;
  fs.writeFileSync(taskFile, body);
  const first = parseTaskFile(taskFile, root)[0];
  fs.writeFileSync(taskFile, `# Plan\n\n${body}`);
  const moved = parseTaskFile(taskFile, root)[0];

  assert.equal(first.id, "task:feature-foundation");
  assert.equal(moved.id, first.id);
  assert.deepEqual(first.writes, ["crates/b.rs", "crates/c.rs"]);
  assert.deepEqual(first.blocked_by, ["another-task", "third-task"]);
  assert.deepEqual(first.requirements, ["1.1", "1.2"]);
  assert.equal(first.priority, "P1");
  assert.equal(first.value, "high");
  assert.equal(first.wave, 2);
});

test("parses the coding skill task template as a complete workflow packet", () => {
  const repositoryRoot = path.resolve(__dirname, "../../../..");
  const taskReference = path.join(repositoryRoot, ".agents", "skills", "coding", "references", "tasks.md");
  const [task] = parseTaskFile(taskReference, repositoryRoot);

  assert.equal(task.id, "task:feature-name-descriptive-outcome");
  assert.equal(task.explicit_id, true);
  assert.deepEqual(task.requirements, ["1.1", "1.2"]);
  assert.deepEqual(task.reads, ["path/to/existing.rs"]);
  assert.deepEqual(task.writes, ["path/to/code.rs", "path/to/test.rs"]);
  assert.deepEqual(task.validation, ["cargo test -p relevant_crate test_name"]);
  assert.equal(task.wave, 1);
});

test("plans only ready tasks with compatible writes", () => {
  const { specDirectory, settings } = fixture();
  fs.writeFileSync(
    path.join(specDirectory, "tasks.md"),
    `- [x] 1. Foundation
  - _id: foundation_
  - _writes: crates/base.rs_
  - _Requirements: 1_

- [ ] 2. First candidate
  - _id: first_
  - _priority: P1_
  - _blocked_by: foundation_
  - _writes: crates/shared.rs_
  - _Requirements: 1_

- [ ] 3. Conflicting candidate
  - _id: conflicting_
  - _priority: P2_
  - _writes: crates/shared.rs_
  - _Requirements: 2_

- [ ] 4. Independent candidate
  - _id: independent_
  - _writes: docs/guide.md_
  - _Requirements: 2_

- [ ] 5. Blocked candidate
  - _id: blocked_
  - _blocked_by: missing-task_
  - _writes: crates/blocked.rs_
  - _Requirements: 2_
`,
  );

  const plan = planTasks(settings, { count: 3 });
  assert.deepEqual(
    plan.selected.map((task) => task.id),
    ["task:first", "task:independent"],
  );
  assert(plan.rejected.some((item) => item.task_id === "task:conflicting"));
  assert(plan.rejected.some((item) => item.task_id === "task:blocked"));
  assert.equal(tasksConflict(plan.candidates[0], plan.candidates[1]), true);
});

test("start and completion gates require consistent specs and validation evidence", () => {
  const { specDirectory, settings } = fixture();
  fs.writeFileSync(
    path.join(specDirectory, "tasks.md"),
    `- [ ] 1. Implement feature
  - _id: implement-feature_
  - _writes: crates/feature.rs_
  - _Requirements: 1.1_
`,
  );
  const task = listTasks(settings)[0];
  const start = checkTaskConsistency(settings, task, "start");
  const incomplete = checkTaskConsistency(settings, task, "complete");
  const complete = checkTaskConsistency(settings, task, "complete", { validation_evidence: "cargo test -p feature" });

  assert.equal(start.passed, true);
  assert.equal(incomplete.passed, false);
  assert.equal(complete.passed, true);
});

test("initializes and strictly renders a workflow contract", () => {
  const { settings } = fixture();
  const initialized = initializeWorkflow(settings);
  assert.equal(initialized.validation.valid, true);
  assert.equal(fs.existsSync(path.join(settings.repository_path, "WORKFLOW.md")), true);
  assert.equal(renderTemplate("Task {{ issue.title }}", { title: "Example" }, null), "Task Example");
  assert.throws(() => renderTemplate("{{ issue.missing }}", { title: "Example" }, null), /unknown variable/);
});

test("expires leases and finishes tasks idempotently", () => {
  const { specDirectory, settings } = fixture();
  fs.writeFileSync(
    path.join(specDirectory, "tasks.md"),
    `- [ ] 1. Repair completion
  - _id: repair-completion_
  - _writes: crates/repair.rs_
  - _validation: cargo test -p repair_
  - _Requirements: 1_
`,
  );
  assert.equal(
    isActivityActive({ status: "active", expires_at: new Date(Date.now() + 60_000).toISOString() }),
    true,
  );
  assert.equal(
    isActivityActive({ status: "active", expires_at: new Date(Date.now() - 60_000).toISOString() }),
    false,
  );

  const first = finishTask(settings, {
    task_id: "task:repair-completion",
    validation_evidence: "cargo test -p repair passed",
  });
  assert.equal(first.local.updated, true);
  const second = finishTask(settings, {
    task_id: "task:repair-completion",
    validation_evidence: "cargo test -p repair passed",
  });
  assert.equal(second.local.updated, false);
  const finishedPacket = fs.readFileSync(path.join(specDirectory, "tasks.md"), "utf8");
  assert.match(finishedPacket, /^- \[x\]/);
  assert.match(finishedPacket, /_validation_evidence: cargo test -p repair passed_/);
});

test("matches manifest globs and read-write conflicts", () => {
  const globWriter = { writes: ["icons/*.svg"], reads: [], blocked_by: [], id: "task:glob" };
  const exactWriter = { writes: ["icons/rust.svg"], reads: [], blocked_by: [], id: "task:exact" };
  const reader = { writes: [], reads: ["icons/rust.svg"], blocked_by: [], id: "task:reader" };
  assert.equal(tasksConflict(globWriter, exactWriter), true);
  assert.equal(tasksConflict(globWriter, reader), true);
});

test("rejects nonexistent acceptance criteria", () => {
  const { specDirectory, settings } = fixture();
  fs.writeFileSync(
    path.join(specDirectory, "tasks.md"),
    `- [ ] 1. Invalid reference
  - _id: invalid-reference_
  - _writes: crates/feature.rs_
  - _Requirements: 1.999_
`,
  );
  const result = checkTaskConsistency(settings, listTasks(settings)[0], "start");
  assert.equal(result.passed, false);
  assert.match(result.errors.join("\n"), /1\.999/);
});

test("help flags and bare invocation are read-only", () => {
  const script = path.join(__dirname, "workflow.js");
  for (const args of [[], ["--help"], ["-h"]]) {
    const result = childProcess.spawnSync(process.execPath, [script, ...args], { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /^Usage:/);
  }
  const unknown = childProcess.spawnSync(process.execPath, [script, "plan", "--bogus"], { encoding: "utf8" });
  assert.equal(unknown.status, 1);
  assert.match(unknown.stderr, /Unknown option/);
  const broadForce = childProcess.spawnSync(process.execPath, [script, "pick", "task:example", "--force"], {
    encoding: "utf8",
  });
  assert.equal(broadForce.status, 1);
  assert.match(broadForce.stderr, /narrowly scoped override/);
});

test("semantic gate failures return a nonzero CLI status", () => {
  const { specDirectory, settings } = fixture();
  fs.writeFileSync(
    path.join(specDirectory, "tasks.md"),
    `- [ ] 1. Needs evidence
  - _id: needs-evidence_
  - _writes: crates/feature.rs_
  - _Requirements: 1.1_
`,
  );
  initializeWorkflow(settings);
  const script = path.join(__dirname, "workflow.js");
  const result = childProcess.spawnSync(
    process.execPath,
    [script, "check", "task:needs-evidence", "--phase", "complete", "--json"],
    {
      encoding: "utf8",
      env: { ...process.env, WORKFLOW_SETTINGS: JSON.stringify(settings) },
    },
  );
  assert.equal(result.status, 1, result.stderr);
  assert.equal(JSON.parse(result.stdout).passed, false);
});

test("locks concurrent journal updates", async () => {
  const { settings } = fixture();
  const modulePath = path.join(__dirname, "workflow.js");
  const code = [
    "const workflow = require(process.argv[1]);",
    "const settings = JSON.parse(process.argv[2]);",
    "workflow.updateWorkflowOperation(settings, process.argv[3], { kind: 'close', status: 'started', steps: [] });",
  ].join(" ");
  await Promise.all(
    Array.from({ length: 8 }, (_, index) => new Promise((resolve, reject) => {
      const child = childProcess.spawn(process.execPath, ["-e", code, modulePath, JSON.stringify(settings), `operation-${index}`]);
      child.once("error", reject);
      child.once("exit", (status) => status === 0 ? resolve() : reject(new Error(`child exited ${status}`)));
    })),
  );
  const journal = JSON.parse(fs.readFileSync(path.join(settings.repository_path, settings.workflow_journal_path), "utf8"));
  assert.equal(journal.operations.length, 8);
  updateWorkflowOperation(settings, "operation-0", { status: "completed" });
  const updated = JSON.parse(fs.readFileSync(path.join(settings.repository_path, settings.workflow_journal_path), "utf8"));
  assert.equal(updated.operations.find((operation) => operation.id === "operation-0").status, "completed");
});

test("creates Linear claims with a complete expiring lease in one mutation", async () => {
  const { specDirectory, settings } = fixture();
  fs.writeFileSync(
    path.join(specDirectory, "tasks.md"),
    `- [ ] 1. Claim safely
  - _id: claim-safely_
  - _writes: crates/feature.rs_
  - _validation: cargo test -p feature_
  - _Requirements: 1.1_
`,
  );
  initializeWorkflow(settings);
  const previousSettings = process.env.WORKFLOW_SETTINGS;
  const previousApiKey = process.env.LINEAR_API_KEY;
  const previousTeamKey = process.env.LINEAR_TEAM_KEY;
  const previousFetch = global.fetch;
  const requests = [];
  process.env.WORKFLOW_SETTINGS = JSON.stringify({
    ...settings,
    linear_api_key: "$LINEAR_API_KEY",
    linear_team_key: "$LINEAR_TEAM_KEY",
  });
  process.env.LINEAR_API_KEY = "test-token";
  process.env.LINEAR_TEAM_KEY = "SIM";
  global.fetch = async (_url, options) => {
    const body = JSON.parse(options.body);
    requests.push(body);
    if (body.query.includes("searchIssues")) return mockGraphql({ searchIssues: { nodes: [] } });
    if (body.query.includes("teams(")) return mockGraphql({ teams: { nodes: [{ id: "team-1", key: "SIM", name: "Sim" }] } });
    if (body.query.includes("issueCreate")) {
      return mockGraphql({
        issueCreate: {
          success: true,
          issue: {
            id: "issue-1",
            identifier: "SIM-1",
            title: "Claim safely",
            url: "https://linear.example/SIM-1",
            branchName: "agent/claim-safely",
            state: { name: "Todo" },
          },
        },
      });
    }
    throw new Error(`Unexpected query: ${body.query}`);
  };
  try {
    const result = await callTool("workflow_pick_task", {
      task_id: "task:claim-safely",
      owner: "agent-a",
      lease_minutes: 30,
    });
    assert.equal(result.action, "created");
    const create = requests.find((request) => request.query.includes("issueCreate"));
    assert.match(create.variables.input.description, /^workflow\.activity:active$/m);
    assert.match(create.variables.input.description, /^workflow\.activity_owner:agent-a$/m);
    assert.match(create.variables.input.description, /^workflow\.activity_lease_id:[0-9a-f-]+$/m);
    assert.match(create.variables.input.description, /^workflow\.activity_expires_at:.+$/m);
    assert.equal(requests.some((request) => request.query.includes("issueUpdate")), false);
  } finally {
    if (previousSettings === undefined) delete process.env.WORKFLOW_SETTINGS;
    else process.env.WORKFLOW_SETTINGS = previousSettings;
    if (previousApiKey === undefined) delete process.env.LINEAR_API_KEY;
    else process.env.LINEAR_API_KEY = previousApiKey;
    if (previousTeamKey === undefined) delete process.env.LINEAR_TEAM_KEY;
    else process.env.LINEAR_TEAM_KEY = previousTeamKey;
    global.fetch = previousFetch;
  }
});

test("closes a finished task without editing repository files", async () => {
  const { specDirectory, settings } = fixture();
  const taskPath = path.join(specDirectory, "tasks.md");
  fs.writeFileSync(
    taskPath,
    `- [x] 1. Close safely
  - _id: close-safely_
  - _writes: crates/feature.rs_
  - _validation: cargo test -p feature_
  - _Requirements: 1.1_
`,
  );
  initializeWorkflow(settings);
  const previousSettings = process.env.WORKFLOW_SETTINGS;
  const previousApiKey = process.env.LINEAR_API_KEY;
  const previousTeamKey = process.env.LINEAR_TEAM_KEY;
  const previousFetch = global.fetch;
  process.env.WORKFLOW_SETTINGS = JSON.stringify({
    ...settings,
    linear_api_key: "$LINEAR_API_KEY",
    linear_team_key: "$LINEAR_TEAM_KEY",
  });
  process.env.LINEAR_API_KEY = "test-token";
  process.env.LINEAR_TEAM_KEY = "SIM";
  const issueDescription = [
    "workflow.local_task_id:task:close-safely",
    "workflow.activity:active",
    "workflow.activity_owner:agent-a",
    "workflow.activity_lease_id:lease-1",
    `workflow.activity_expires_at:${new Date(Date.now() + 60_000).toISOString()}`,
  ].join("\n");
  global.fetch = async (_url, options) => {
    const body = JSON.parse(options.body);
    if (body.query.includes("searchIssues")) {
      return mockGraphql({
        searchIssues: {
          nodes: [{
            id: "issue-2",
            identifier: "SIM-2",
            title: "Close safely",
            url: "https://linear.example/SIM-2",
            branchName: "agent/close-safely",
            description: issueDescription,
            state: { name: "In Progress" },
          }],
        },
      });
    }
    if (body.query.includes("teams(")) return mockGraphql({ teams: { nodes: [{ id: "team-1", key: "SIM", name: "Sim" }] } });
    if (body.query.includes("workflowStates")) return mockGraphql({ workflowStates: { nodes: [{ id: "state-done", name: "Done" }] } });
    if (body.query.includes("query($id: String!)")) {
      return mockGraphql({ issue: { id: "issue-2", identifier: "SIM-2", description: issueDescription } });
    }
    if (body.query.includes("issueUpdate")) {
      return mockGraphql({
        issueUpdate: {
          success: true,
          issue: {
            id: "issue-2",
            identifier: "SIM-2",
            title: "Close safely",
            url: "https://linear.example/SIM-2",
            branchName: "agent/close-safely",
            description: body.variables.input.description || issueDescription,
            state: { name: "Done" },
          },
        },
      });
    }
    throw new Error(`Unexpected query: ${body.query}`);
  };
  const before = fs.readFileSync(taskPath, "utf8");
  try {
    const result = await callTool("workflow_close_task", {
      task_id: "task:close-safely",
      override_merge: true,
      override_validation: true,
      override_reason: "Temporary repository fixture has no origin/main.",
    });
    assert.equal(result.action, "closed");
    assert.equal(result.activity.status, "inactive");
    assert.equal(fs.readFileSync(taskPath, "utf8"), before);
    const journal = JSON.parse(fs.readFileSync(path.join(settings.repository_path, settings.workflow_journal_path), "utf8"));
    assert.deepEqual(journal.operations[0].steps, ["linear_moved", "activity_released"]);
    assert.equal(journal.operations[0].status, "completed");
  } finally {
    if (previousSettings === undefined) delete process.env.WORKFLOW_SETTINGS;
    else process.env.WORKFLOW_SETTINGS = previousSettings;
    if (previousApiKey === undefined) delete process.env.LINEAR_API_KEY;
    else process.env.LINEAR_API_KEY = previousApiKey;
    if (previousTeamKey === undefined) delete process.env.LINEAR_TEAM_KEY;
    else process.env.LINEAR_TEAM_KEY = previousTeamKey;
    global.fetch = previousFetch;
  }
});

function mockGraphql(data) {
  return {
    ok: true,
    statusText: "OK",
    json: async () => ({ data }),
  };
}
