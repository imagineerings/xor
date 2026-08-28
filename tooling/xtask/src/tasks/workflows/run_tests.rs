use gh_workflow::{
    Container, Event, Expression, Job, Port, PullRequest, Push, Run, Step, Strategy, Use, Workflow,
    WorkflowDispatch,
};
use indexmap::IndexMap;
use indoc::formatdoc;
use serde_json::json;

use crate::product_manifest::ProductManifest;

use crate::tasks::workflows::{
    steps::{
        CommonJobConditions, CommonPermissionSets, repository_owner_guard_expression, use_clang,
    },
    vars::{self, PathCondition},
};

use super::{
    runners::{self, Platform},
    steps::{self, FluentBuilder, NamedJob, named, release_job},
};

pub(crate) fn run_tests() -> Workflow {
    let shared_clippy = shared_clippy();
    let workspace_tests = workspace_tests();
    let project_benchmarks = project_benchmarks();
    let rust_tools_validation = rust_tools_validation();
    let validation = shared_validation(&[
        &shared_clippy,
        &workspace_tests,
        &project_benchmarks,
        &rust_tools_validation,
    ]);
    let comfy_backend_validation = comfy_backend_validation();
    let product_smoke = product_smoke(&validation);

    named::workflow()
        .with_minimal_permissions()
        .on(Event::default()
            .push(Push::default().add_branch("main"))
            .pull_request(PullRequest::default().add_branch("**"))
            .workflow_dispatch(WorkflowDispatch::default()))
        .concurrency(vars::one_workflow_per_non_main_branch())
        .add_env(("CARGO_TERM_COLOR", "always"))
        .add_env(("RUST_BACKTRACE", 1))
        .add_env(("CARGO_INCREMENTAL", 0))
        .add_job(shared_clippy.name, shared_clippy.job)
        .add_job(workspace_tests.name, workspace_tests.job)
        .add_job(project_benchmarks.name, project_benchmarks.job)
        .add_job(rust_tools_validation.name, rust_tools_validation.job)
        .add_job(validation.name, validation.job)
        .add_job(comfy_backend_validation.name, comfy_backend_validation.job)
        .add_job(product_smoke.name, product_smoke.job)
}

fn shared_clippy() -> NamedJob {
    named::job(
        Job::default()
            .runs_on("ubuntu-22.04")
            .timeout_minutes(60u32)
            .map(use_clang)
            .add_step(steps::checkout_repo())
            .add_step(steps::setup_cargo_config(Platform::Linux))
            .add_step(steps::setup_linux())
            .add_step(steps::cargo_fmt())
            .add_step(named::bash("./script/clippy"))
            .add_step(steps::cleanup_cargo_config(Platform::Linux)),
    )
}

fn workspace_tests() -> NamedJob {
    named::job(
        Job::default()
            .runs_on("ubuntu-22.04")
            .timeout_minutes(75u32)
            .map(use_clang)
            .add_step(steps::checkout_repo())
            .add_step(steps::free_linux_disk_space())
            .add_step(steps::setup_cargo_config(Platform::Linux))
            .map(steps::install_linux_dependencies)
            .add_step(steps::setup_node())
            .add_step(steps::cargo_install_nextest())
            .add_step(named::bash(
                "cargo nextest run --workspace --no-fail-fast --no-tests=warn",
            ))
            .add_step(steps::cleanup_cargo_config(Platform::Linux)),
    )
}

fn project_benchmarks() -> NamedJob {
    named::job(
        Job::default()
            .runs_on("ubuntu-22.04")
            .timeout_minutes(45u32)
            .map(use_clang)
            .add_step(steps::checkout_repo())
            .add_step(steps::setup_cargo_config(Platform::Linux))
            .add_step(steps::setup_linux())
            .add_step(named::bash(
                "cargo bench -p project --features cargo-workspace --bench cargo_workspace -- --noplot",
            ))
            .add_step(named::bash(
                "cargo bench -p project --features structured-execution --bench structured_execution -- --noplot",
            ))
            .add_step(steps::cleanup_cargo_config(Platform::Linux)),
    )
}

fn rust_tools_validation() -> NamedJob {
    named::job(
        Job::default()
            .runs_on("ubuntu-22.04")
            .timeout_minutes(45u32)
            .map(use_clang)
            .add_step(steps::checkout_repo())
            .add_step(steps::setup_cargo_config(Platform::Linux))
            .add_step(steps::setup_linux())
            .add_step(named::bash(
                "./script/test-rust-tools-environments --matrix --offline",
            ))
            .add_step(steps::cleanup_cargo_config(Platform::Linux)),
    )
}

fn shared_validation(workers: &[&NamedJob]) -> NamedJob {
    let worker_names = workers
        .iter()
        .map(|worker| worker.name.clone())
        .collect::<Vec<_>>();
    let check_results = workers.iter().fold(
        named::bash(indoc::indoc! {r#"
            exit_code=0
            for result in "$SHARED_CLIPPY_RESULT" "$WORKSPACE_TESTS_RESULT" "$PROJECT_BENCHMARKS_RESULT" "$RUST_TOOLS_VALIDATION_RESULT"; do
                if [[ "$result" != "success" ]]; then
                    exit_code=1
                fi
            done
            exit "$exit_code"
        "#}),
        |step, worker| {
            step.add_env((
                format!("{}_RESULT", worker.name.to_uppercase()),
                format!("${{{{ needs.{}.result }}}}", worker.name),
            ))
        },
    );

    named::job(
        Job::default()
            .needs(worker_names)
            .cond(Expression::new("${{ always() }}"))
            .runs_on("ubuntu-22.04")
            .timeout_minutes(5u32)
            .add_step(check_results),
    )
}

fn comfy_backend_validation() -> NamedJob {
    let include = vec![
        json!({
            "platform": "linux",
            "runner": "ubuntu-22.04",
        }),
        json!({
            "platform": "macos",
            "runner": "macos-15",
        }),
        json!({
            "platform": "windows",
            "runner": "windows-2022",
        }),
    ];

    named::job(
        Job::default()
            .runs_on("${{ matrix.runner }}")
            .timeout_minutes(60u32)
            .strategy(
                Strategy::default()
                    .fail_fast(false)
                    .matrix(json!({ "include": include })),
            )
            .add_step(steps::checkout_repo())
            .add_step(
                steps::enable_windows_long_paths()
                    .if_condition(Expression::new("matrix.platform == 'windows'")),
            )
            .add_step(
                steps::setup_cargo_config(Platform::Linux)
                    .if_condition(Expression::new("matrix.platform == 'linux'")),
            )
            .add_step(
                steps::setup_cargo_config(Platform::Mac)
                    .if_condition(Expression::new("matrix.platform == 'macos'")),
            )
            .add_step(
                steps::setup_cargo_config(Platform::Windows)
                    .if_condition(Expression::new("matrix.platform == 'windows'")),
            )
            .add_step(
                named::bash(
                    "cargo clippy --release --all-targets --all-features -p comfy_backend_corex -p comfy_backend_cuda -p comfy_backend_directml -p comfy_backend_metal -p comfy_backend_mlu -p comfy_backend_npu -p comfy_backend_rocm -p comfy_backend_xpu -- --deny warnings",
                )
                .if_condition(Expression::new("matrix.platform == 'linux'")),
            )
            .add_step(
                named::bash(
                    "cargo clippy --release --all-targets --all-features -p comfy_backend_metal -- --deny warnings",
                )
                .if_condition(Expression::new("matrix.platform == 'macos'")),
            )
            .add_step(
                named::pwsh(
                    "cargo clippy --release --all-targets --all-features -p comfy_backend_cuda -p comfy_backend_directml -- --deny warnings",
                )
                .if_condition(Expression::new("matrix.platform == 'windows'")),
            )
            .add_step(
                steps::cleanup_cargo_config(Platform::Linux)
                    .if_condition(Expression::new("always() && matrix.platform == 'linux'")),
            )
            .add_step(
                steps::cleanup_cargo_config(Platform::Mac)
                    .if_condition(Expression::new("always() && matrix.platform == 'macos'")),
            )
            .add_step(
                steps::cleanup_cargo_config(Platform::Windows)
                    .if_condition(Expression::new("always() && matrix.platform == 'windows'")),
            ),
    )
}

fn product_smoke(validation: &NamedJob) -> NamedJob {
    let manifest = ProductManifest::load().expect("product catalog must be valid");
    let include = manifest
        .enabled_products()
        .map(|product| {
            json!({
                "product": product.id,
                "application_features": product.cargo_features.join(","),
                "remote_features": product.remote_server_features.join(","),
            })
        })
        .collect::<Vec<_>>();

    named::job(
        Job::default()
            .needs([validation.name.clone()])
            .runs_on("ubuntu-22.04")
            .timeout_minutes(45u32)
            .strategy(Strategy::default().fail_fast(false).matrix(json!({ "include": include })))
            .map(use_clang)
            .add_step(steps::checkout_repo())
            .add_step(steps::setup_cargo_config(Platform::Linux))
            .map(steps::install_linux_dependencies)
            .add_step(named::bash(
                "cargo xtask bundle --product \"$PRODUCT_ID\" --platform linux --target x86_64-unknown-linux-gnu --dry-run",
            ).add_env(("PRODUCT_ID", "${{ matrix.product }}")))
            .add_step(named::bash(
                "cargo check --release --package zed --no-default-features --features \"$APPLICATION_FEATURES\"",
            ).add_env(("ZED_PRODUCT_ID", "${{ matrix.product }}")).add_env(("APPLICATION_FEATURES", "${{ matrix.application_features }}")))
            .add_step(named::bash(
                "cargo check --release --package remote_server --no-default-features --features \"$REMOTE_FEATURES\"",
            ).add_env(("ZED_PRODUCT_ID", "${{ matrix.product }}")).add_env(("REMOTE_FEATURES", "${{ matrix.remote_features }}")))
            .add_step(steps::cleanup_cargo_config(Platform::Linux)),
    )
}

pub fn orchestrate_for_extension(rules: &[&PathCondition]) -> NamedJob {
    orchestrate_impl(rules)
}

fn orchestrate_impl(rules: &[&PathCondition]) -> NamedJob {
    let name = "orchestrate".to_owned();
    let step_name = "filter".to_owned();
    let mut script = String::new();

    script.push_str(indoc::indoc! {r#"
        set -euo pipefail
        if [ -z "$GITHUB_BASE_REF" ]; then
          echo "Not in a PR context (i.e., push to main/stable/preview)"
          COMPARE_REV="$(git rev-parse HEAD~1)"
        else
          echo "In a PR context comparing to pull_request.base.ref"
          git fetch origin "$GITHUB_BASE_REF" --depth=350
          COMPARE_REV="$(git merge-base "origin/${GITHUB_BASE_REF}" HEAD)"
        fi
        CHANGED_FILES="$(git diff --name-only "$COMPARE_REV" "$GITHUB_SHA")"

    "#});

    script.push_str(indoc::indoc! {r#"
        # When running from a subdirectory, git diff returns repo-root-relative paths.
        # Filter to only files within the current working directory and strip the prefix.
        REPO_SUBDIR="$(git rev-parse --show-prefix)"
        REPO_SUBDIR="${REPO_SUBDIR%/}"
        if [ -n "$REPO_SUBDIR" ]; then
            CHANGED_FILES="$(echo "$CHANGED_FILES" | grep "^${REPO_SUBDIR}/" | sed "s|^${REPO_SUBDIR}/||" || true)"
        fi

    "#});

    script.push_str(indoc::indoc! {r#"
        check_pattern() {
          local output_name="$1"
          local pattern="$2"
          local grep_arg="$3"

          echo "$CHANGED_FILES" | grep "$grep_arg" "$pattern" && \
            echo "${output_name}=true" >> "$GITHUB_OUTPUT" || \
            echo "${output_name}=false" >> "$GITHUB_OUTPUT"
        }

    "#});

    let mut outputs = IndexMap::new();

    for rule in rules {
        assert!(
            rule.set_by_step
                .borrow_mut()
                .replace(name.clone())
                .is_none()
        );
        assert!(
            outputs
                .insert(
                    rule.name.to_owned(),
                    format!("${{{{ steps.{}.outputs.{} }}}}", step_name, rule.name)
                )
                .is_none()
        );

        let grep_arg = if rule.invert { "-qvP" } else { "-qP" };
        script.push_str(&format!(
            "check_pattern \"{}\" '{}' {}\n",
            rule.name, rule.pattern, grep_arg
        ));
    }

    let job = Job::default()
        .runs_on(runners::LINUX_SMALL)
        .with_repository_owner_guard()
        .outputs(outputs)
        .add_step(steps::checkout_repo().with_deep_history_on_non_main())
        .add_step(Step::new(step_name.clone()).run(script).id(step_name));

    NamedJob { name, job }
}

pub fn tests_pass(jobs: &[NamedJob], extra_job_names: &[&str]) -> NamedJob {
    let mut script = String::from(indoc::indoc! {r#"
        set +x
        EXIT_CODE=0

        check_result() {
          echo "* $1: $2"
          if [[ "$2" != "skipped" && "$2" != "success" ]]; then EXIT_CODE=1; fi
        }

    "#});

    let all_names: Vec<&str> = jobs
        .iter()
        .map(|job| job.name.as_str())
        .chain(extra_job_names.iter().copied())
        .collect();

    let env_entries: Vec<_> = all_names
        .iter()
        .map(|name| {
            let env_name = format!("RESULT_{}", name.to_uppercase());
            let env_value = format!("${{{{ needs.{}.result }}}}", name);
            (env_name, env_value)
        })
        .collect();

    script.push_str(
        &all_names
            .iter()
            .zip(env_entries.iter())
            .map(|(name, (env_name, _))| format!("check_result \"{}\" \"${}\"", name, env_name))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    script.push_str("\n\nexit $EXIT_CODE\n");

    let job = Job::default()
        .runs_on(runners::LINUX_SMALL)
        .needs(
            all_names
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<String>>(),
        )
        .cond(repository_owner_guard_expression(true))
        .add_step(
            env_entries
                .into_iter()
                .fold(named::bash(&script), |step, env_item| {
                    step.add_env(env_item)
                }),
        );

    named::job(job)
}

/// Bash script snippet that detects changed extension directories from `$CHANGED_FILES`.
/// Assumes `$CHANGED_FILES` is already set. Sets `$EXTENSIONS_JSON` to a JSON array of
/// changed extension paths. Callers are responsible for writing the result to `$GITHUB_OUTPUT`.
pub(crate) const DETECT_CHANGED_EXTENSIONS_SCRIPT: &str = indoc::indoc! {r#"
    # Detect changed extension directories (excluding extensions/workflows)
    CHANGED_EXTENSIONS=$(echo "$CHANGED_FILES" | grep -oP '^extensions/[^/]+(?=/)' | sort -u | grep -v '^extensions/workflows$' || true)
    # Filter out deleted extensions
    EXISTING_EXTENSIONS=""
    for ext in $CHANGED_EXTENSIONS; do
        if [ -f "$ext/extension.toml" ]; then
            EXISTING_EXTENSIONS=$(printf '%s\n%s' "$EXISTING_EXTENSIONS" "$ext")
        fi
    done
    CHANGED_EXTENSIONS=$(echo "$EXISTING_EXTENSIONS" | sed '/^$/d')
    if [ -n "$CHANGED_EXTENSIONS" ]; then
        EXTENSIONS_JSON=$(echo "$CHANGED_EXTENSIONS" | jq -R -s -c 'split("\n") | map(select(length > 0))')
    else
        EXTENSIONS_JSON="[]"
    fi
"#};

const TS_QUERY_LS_FILE: &str = "ts_query_ls-x86_64-unknown-linux-gnu.tar.gz";
const CI_TS_QUERY_RELEASE: &str = "tags/v3.15.1";

pub(crate) fn fetch_ts_query_ls() -> Step<Use> {
    named::uses(
        "dsaltares",
        "fetch-gh-release-asset",
        "aa37ae5c44d3c9820bc12fe675e8670ecd93bd1c",
    ) // v1.1.1
    .add_with(("repo", "ribru17/ts_query_ls"))
    .add_with(("version", CI_TS_QUERY_RELEASE))
    .add_with(("file", TS_QUERY_LS_FILE))
}

pub(crate) enum RunContext {
    Extension,
}

pub(crate) fn run_ts_query_ls(context: RunContext) -> Step<Run> {
    named::bash(formatdoc!(
        r#"tar -xf "$GITHUB_WORKSPACE/{TS_QUERY_LS_FILE}" -C "$GITHUB_WORKSPACE"
        "$GITHUB_WORKSPACE/ts_query_ls" format --check {directory} || {{
            echo "Found unformatted queries, please format them with ts_query_ls."
            echo "For easy use, install the Tree-sitter query extension:"
            echo "zed://extension/tree-sitter-query"
            false
        }}"#,
        directory = match context {
            RunContext::Extension => "languages",
        }
    ))
}

pub(crate) fn run_platform_tests_no_filter(platform: Platform) -> NamedJob {
    run_platform_tests_impl(platform, false, false)
}

fn run_platform_tests_impl(platform: Platform, filter_packages: bool, harden: bool) -> NamedJob {
    let runner = match platform {
        Platform::Windows => runners::WINDOWS_DEFAULT,
        Platform::Linux => runners::LINUX_DEFAULT,
        Platform::Mac => runners::MAC_DEFAULT,
    };
    NamedJob {
        name: format!("run_tests_{platform}"),
        job: release_job(&[])
            .runs_on(runner)
            .when(platform == Platform::Linux, |job| {
                job.add_service(
                    "postgres",
                    Container::new("postgres:15@sha256:1b92e7a80c021647bf70f5d3eb66066a998e4f5cf43c07bb9dc9f729782cf88e")
                        .add_env(("POSTGRES_HOST_AUTH_METHOD", "trust"))
                        .ports(vec![Port::Name("5432:5432".into())])
                        .options(
                            "--health-cmd pg_isready \
                             --health-interval 500ms \
                             --health-timeout 5s \
                             --health-retries 10",
                        ),
                )
            })
            .when(harden && platform == Platform::Linux, |this| {
                this.add_step(steps::harden_runner())
            })
            .add_step(steps::checkout_repo())
            .add_step(steps::setup_cargo_config(platform))
            .when(platform == Platform::Mac, |this| {
                this.add_step(steps::cache_rust_dependencies_namespace())
            })
            .when(platform == Platform::Linux, |this| {
                use_clang(this.add_step(steps::cache_rust_dependencies_namespace()))
            })
            .when(
                platform == Platform::Linux,
                steps::install_linux_dependencies,
            )
            .add_step(steps::setup_node())
            .when(
                platform == Platform::Linux || platform == Platform::Mac,
                |job| job.add_step(steps::cargo_install_nextest()),
            )
            .add_step(steps::clear_target_dir_if_large(platform))
            .add_step(steps::setup_sccache(platform))
            .when(filter_packages, |job| {
                job.add_step(
                    steps::cargo_nextest(platform).with_changed_packages_filter("orchestrate"),
                )
            })
            .when(!filter_packages, |job| {
                job.add_step(steps::cargo_nextest(platform))
            })
            .add_step(steps::show_sccache_stats(platform))
            .add_step(steps::cleanup_cargo_config(platform)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    fn job<'a>(workflow: &'a Value, name: &str) -> &'a Value {
        workflow
            .get("jobs")
            .and_then(|jobs| jobs.get(name))
            .unwrap_or_else(|| panic!("missing {name} job"))
    }

    fn run_commands(job: &Value) -> Vec<&str> {
        job.get("steps")
            .and_then(Value::as_sequence)
            .expect("job steps")
            .iter()
            .filter_map(|step| step.get("run").and_then(Value::as_str))
            .collect()
    }

    fn needs(job: &Value) -> Vec<&str> {
        job.get("needs")
            .and_then(Value::as_sequence)
            .map(|needs| needs.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    #[test]
    fn rust_product_ci_has_shared_validation_and_one_smoke_row() -> anyhow::Result<()> {
        let workflow = run_tests()
            .to_string()
            .map_err(|error| anyhow::anyhow!("failed to serialize CI workflow: {error:?}"))?;
        let parsed: Value = serde_yaml::from_str(&workflow)?;

        let worker_names = [
            "shared_clippy",
            "workspace_tests",
            "project_benchmarks",
            "rust_tools_validation",
        ];
        for worker_name in worker_names {
            assert!(needs(job(&parsed, worker_name)).is_empty());
        }

        let shared_validation = job(&parsed, "shared_validation");
        assert_eq!(needs(shared_validation), worker_names);
        assert_eq!(
            shared_validation.get("if").and_then(Value::as_str),
            Some("${{ always() }}")
        );
        let aggregator_commands = run_commands(shared_validation);
        assert_eq!(aggregator_commands.len(), 1);
        assert!(aggregator_commands[0].contains("[[ \"$result\" != \"success\" ]]"));
        assert!(aggregator_commands[0].contains("exit \"$exit_code\""));
        let aggregator_steps = shared_validation
            .get("steps")
            .and_then(Value::as_sequence)
            .expect("aggregator steps");
        assert_eq!(aggregator_steps.len(), 1);
        let aggregator_env = aggregator_steps[0]
            .get("env")
            .expect("aggregator result environment");
        for worker_name in worker_names {
            let environment_name = format!("{}_RESULT", worker_name.to_uppercase());
            let expected_result = format!("${{{{ needs.{worker_name}.result }}}}");
            assert_eq!(
                aggregator_env
                    .get(&environment_name)
                    .and_then(Value::as_str),
                Some(expected_result.as_str())
            );
        }

        assert_eq!(needs(job(&parsed, "product_smoke")), ["shared_validation"]);

        let shared_clippy_commands = run_commands(job(&parsed, "shared_clippy"));
        assert!(shared_clippy_commands.contains(&"cargo fmt --all -- --check"));
        assert!(shared_clippy_commands.contains(&"./script/clippy"));
        let workspace_test_commands = run_commands(job(&parsed, "workspace_tests"));
        assert!(workspace_test_commands.iter().any(|command| {
            *command == "cargo nextest run --workspace --no-fail-fast --no-tests=warn"
        }));
        let benchmark_commands = run_commands(job(&parsed, "project_benchmarks"));
        assert!(
            benchmark_commands
                .iter()
                .any(|command| command.contains("--bench cargo_workspace -- --noplot"))
        );
        assert!(
            benchmark_commands
                .iter()
                .any(|command| command.contains("--bench structured_execution -- --noplot"))
        );
        let rust_tools_commands = run_commands(job(&parsed, "rust_tools_validation"));
        assert!(rust_tools_commands.iter().any(|command| {
            *command == "./script/test-rust-tools-environments --matrix --offline"
        }));

        assert_eq!(workflow.matches("runs-on:").count(), 7);
        assert!(workflow.contains("runs-on: ubuntu-22.04"));
        assert!(workflow.contains("runner: macos-15"));
        assert!(workflow.contains("runner: windows-2022"));
        assert!(workflow.contains("workflow_dispatch: {}"));
        assert!(workflow.contains("cargo fmt --all -- --check"));
        assert!(workflow.contains("./script/clippy"));
        assert!(workflow.contains("cargo nextest run --workspace"));
        assert!(workflow.contains("--bench cargo_workspace -- --noplot"));
        assert!(workflow.contains("--bench structured_execution -- --noplot"));
        assert!(workflow.contains("test-rust-tools-environments --matrix --offline"));
        assert!(workflow.contains("application_features: agentic-tools,rust-tools"));
        assert!(workflow.contains("remote_features: rust-tools"));
        assert!(workflow.contains("cargo xtask bundle --product"));
        assert_eq!(workflow.matches("cargo fmt --all -- --check").count(), 1);
        assert_eq!(workflow.matches("run: ./script/clippy").count(), 1);
        assert_eq!(workflow.matches("cargo nextest run --workspace").count(), 1);
        assert_eq!(
            workflow
                .matches("--bench cargo_workspace -- --noplot")
                .count(),
            1
        );
        assert_eq!(
            workflow
                .matches("--bench structured_execution -- --noplot")
                .count(),
            1
        );
        assert_eq!(
            workflow
                .matches("test-rust-tools-environments --matrix --offline")
                .count(),
            1
        );
        assert!(workflow.contains("-p comfy_backend_corex"));
        assert!(workflow.contains("-p comfy_backend_cuda"));
        assert!(workflow.contains("-p comfy_backend_directml"));
        assert!(workflow.contains("-p comfy_backend_metal"));
        assert!(workflow.contains("-p comfy_backend_mlu"));
        assert!(workflow.contains("-p comfy_backend_npu"));
        assert!(workflow.contains("-p comfy_backend_rocm"));
        assert!(workflow.contains("-p comfy_backend_xpu"));
        assert_eq!(workflow.matches("--all-targets --all-features").count(), 3);
        assert_eq!(workflow.matches("-- --deny warnings").count(), 3);
        assert_eq!(
            workflow
                .matches("git config --global core.longpaths true")
                .count(),
            1
        );
        let windows_long_paths = workflow
            .find("git config --global core.longpaths true")
            .expect("Windows long-path setup should be generated");
        let windows_backend_clippy = workflow
            .find("-p comfy_backend_cuda -p comfy_backend_directml")
            .expect("Windows backend Clippy command should be generated");
        assert!(windows_long_paths < windows_backend_clippy);
        assert!(workflow.contains("cancel-in-progress: true"));
        assert!(!workflow.contains("cargo clean"));

        for forbidden in [
            "repository_owner",
            "namespace-profile",
            "self-32vcpu",
            "R2_ACCOUNT_ID",
            "SENTRY_AUTH_TOKEN",
            "SLACK_WEBHOOK",
            "postgres:",
            "Rustlings",
            "product: jvm",
            "product: game",
        ] {
            assert!(!workflow.contains(forbidden), "found {forbidden} in CI");
        }

        Ok(())
    }
}
