use gh_workflow::{
    Event, Expression, Job, Level, Permissions, Push, Run, Step, Strategy, Use, Workflow,
    WorkflowDispatch,
};
use indoc::formatdoc;
use serde_json::json;

use crate::product_manifest::ProductManifest;

use crate::tasks::workflows::{
    run_bundling::upload_artifact,
    runners,
    steps::{
        self, CommonPermissionSets, DownloadArtifactStep, IfNoFilesFound, NamedJob, dependant_job,
        named,
    },
    vars::{self, StepOutput, WorkflowInput, assets},
};

const CURRENT_ACTION_RUN_URL: &str =
    "${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}";

pub(crate) fn release() -> Workflow {
    let tag = WorkflowInput::string("tag", None)
        .description("Existing rust-v* tag to publish when the workflow is run manually");
    let build = product_builds();
    let publish = publish_product(&build);

    named::workflow()
        .on(Event::default()
            .push(Push::default().tags(vec!["rust-v*".to_string()]))
            .workflow_dispatch(WorkflowDispatch::default().add_input(tag.name, tag.input())))
        .concurrency(vars::one_workflow_per_non_main_branch())
        .with_minimal_permissions()
        .add_env(("CARGO_TERM_COLOR", "always"))
        .add_env(("RUST_BACKTRACE", "1"))
        .add_job(build.name, build.job)
        .add_job(publish.name, publish.job)
}

const PRODUCT_RELEASE_REF: &str =
    "${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref }}";

fn product_builds() -> NamedJob {
    let manifest = ProductManifest::load().expect("product catalog must be valid");
    let mut include = Vec::new();
    for product in manifest.enabled_products() {
        for target in &product.targets {
            let row = match target.as_str() {
                "linux-x86_64" => json!({
                    "product": product.id,
                    "platform": "linux",
                    "runner": "ubuntu-22.04",
                    "target": "x86_64-unknown-linux-gnu",
                    "artifact": format!("{}-linux-x86_64", product.id),
                    "artifact_path": format!("target/products/{}/release/{}-*-linux-x86_64.tar.gz", product.id, product.id),
                }),
                "macos-aarch64" => json!({
                    "product": product.id,
                    "platform": "macos",
                    "runner": "macos-15",
                    "target": "aarch64-apple-darwin",
                    "artifact": format!("{}-macos-aarch64", product.id),
                    "artifact_path": format!("target/products/{}/release/{}-*-macos-aarch64.dmg", product.id, product.id),
                }),
                "windows-x86_64" => json!({
                    "product": product.id,
                    "platform": "windows",
                    "runner": "windows-2022",
                    "target": "x86_64-pc-windows-msvc",
                    "artifact": format!("{}-windows-x86_64", product.id),
                    "artifact_path": format!("target/products/{}/release/{}-*-windows-x86_64.exe", product.id, product.id),
                }),
                unsupported => {
                    panic!("validated catalog contains unsupported target {unsupported}")
                }
            };
            include.push(row);
        }
    }

    let resolve_version = named::bash(indoc::indoc! {r#"
        if [[ "$TAG" != rust-v* ]]; then
            echo "Release tag must start with rust-v: $TAG" >&2
            exit 1
        fi
        echo "RELEASE_VERSION=${TAG#rust-v}" >> "$GITHUB_ENV"
    "#})
    .add_env((
        "TAG",
        "${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref_name }}",
    ));

    named::job(
        Job::default()
            .runs_on("${{ matrix.runner }}")
            .timeout_minutes(120u32)
            .strategy(Strategy::default().fail_fast(false).matrix(json!({ "include": include })))
            .add_step(steps::checkout_repo().with_ref(PRODUCT_RELEASE_REF))
            .add_step(
                steps::enable_windows_long_paths()
                    .if_condition(Expression::new("matrix.platform == 'windows'")),
            )
            .add_step(resolve_version)
            .add_step(
                named::bash("./script/linux && ./script/download-wasi-sdk")
                    .if_condition(Expression::new("matrix.platform == 'linux'")),
            )
            .add_step(
                named::bash("cargo xtask bundle --product \"$PRODUCT_ID\" --platform \"$PLATFORM\" --target \"$TARGET\"")
                    .if_condition(Expression::new("matrix.platform == 'linux'"))
                    .add_env(("PRODUCT_ID", "${{ matrix.product }}"))
                    .add_env(("PLATFORM", "${{ matrix.platform }}"))
                    .add_env(("TARGET", "${{ matrix.target }}"))
                    .add_env(("CC", "clang"))
                    .add_env(("CXX", "clang++")),
            )
            .add_step(
                named::bash("cargo xtask bundle --product \"$PRODUCT_ID\" --platform macos --target \"$TARGET\"")
                    .if_condition(Expression::new("matrix.platform == 'macos'"))
                    .add_env(("PRODUCT_ID", "${{ matrix.product }}"))
                    .add_env(("TARGET", "${{ matrix.target }}"))
                    .add_env(("MACOS_SIGNING_IDENTITY", "${{ secrets.MACOS_SIGNING_IDENTITY }}"))
                    .add_env(("MACOS_CERTIFICATE", "${{ secrets.MACOS_CERTIFICATE }}"))
                    .add_env(("MACOS_CERTIFICATE_PASSWORD", "${{ secrets.MACOS_CERTIFICATE_PASSWORD }}"))
                    .add_env(("APPLE_NOTARIZATION_KEY", "${{ secrets.APPLE_NOTARIZATION_KEY }}"))
                    .add_env(("APPLE_NOTARIZATION_KEY_ID", "${{ secrets.APPLE_NOTARIZATION_KEY_ID }}"))
                    .add_env(("APPLE_NOTARIZATION_ISSUER_ID", "${{ secrets.APPLE_NOTARIZATION_ISSUER_ID }}")),
            )
            .add_step(
                named::pwsh("cargo xtask bundle --product $env:PRODUCT_ID --platform windows --target $env:TARGET")
                    .if_condition(Expression::new("matrix.platform == 'windows'"))
                    .add_env(("PRODUCT_ID", "${{ matrix.product }}"))
                    .add_env(("TARGET", "${{ matrix.target }}"))
                    .add_env(("AZURE_TENANT_ID", "${{ secrets.AZURE_TENANT_ID }}"))
                    .add_env(("AZURE_CLIENT_ID", "${{ secrets.AZURE_CLIENT_ID }}"))
                    .add_env(("AZURE_CLIENT_SECRET", "${{ secrets.AZURE_CLIENT_SECRET }}"))
                    .add_env(("ACCOUNT_NAME", "${{ secrets.ACCOUNT_NAME }}"))
                    .add_env(("CERT_PROFILE_NAME", "${{ secrets.CERT_PROFILE_NAME }}"))
                    .add_env(("ENDPOINT", "${{ secrets.ENDPOINT }}"))
                    .add_env(("FILE_DIGEST", "${{ secrets.FILE_DIGEST }}"))
                    .add_env(("TIMESTAMP_DIGEST", "${{ secrets.TIMESTAMP_DIGEST }}"))
                    .add_env(("TIMESTAMP_SERVER", "${{ secrets.TIMESTAMP_SERVER }}")),
            )
            .add_step(
                steps::upload_artifact("${{ matrix.artifact }}", "${{ matrix.artifact_path }}")
                    .if_no_files_found(IfNoFilesFound::Error),
            ),
    )
}

fn publish_product(build: &NamedJob) -> NamedJob {
    let manifest = ProductManifest::load().expect("product catalog must be valid");
    let product = manifest.product("rust").expect("Rust product must exist");
    let publish = named::bash(
        indoc::indoc! {r#"
        if [[ "$TAG" != rust-v* ]]; then
            echo "Release tag must start with rust-v: $TAG" >&2
            exit 1
        fi

        mapfile -t FILES < <(find release-artifacts -type f)
        if [[ ${#FILES[@]} -ne 3 ]]; then
            echo "Expected exactly three Rust product artifacts, found ${#FILES[@]}" >&2
            exit 1
        fi
        PATTERNS=(
            'rust-*-linux-x86_64.tar.gz'
            'rust-*-macos-aarch64.dmg'
            'rust-*-windows-x86_64.exe'
        )
        for PATTERN in "${PATTERNS[@]}"; do
            mapfile -t MATCHES < <(find release-artifacts -type f -name "$PATTERN")
            if [[ ${#MATCHES[@]} -ne 1 ]]; then
                echo "Expected exactly one artifact matching $PATTERN, found ${#MATCHES[@]}" >&2
                exit 1
            fi
        done
        TITLE="PRODUCT_DISPLAY_NAME ${TAG#rust-}"
        if gh release view "$TAG" >/dev/null 2>&1; then
            gh release edit "$TAG" --title "$TITLE"
            gh release upload "$TAG" "${FILES[@]}" --clobber
        else
            gh release create "$TAG" "${FILES[@]}" --title "$TITLE" --generate-notes --verify-tag
        fi
    "#}
        .replace("PRODUCT_DISPLAY_NAME", &product.display_name),
    )
    .add_env((
        "TAG",
        "${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref_name }}",
    ))
    .add_env(("GH_TOKEN", vars::GITHUB_TOKEN));

    named::job(
        Job::default()
            .needs([build.name.clone()])
            .runs_on("ubuntu-22.04")
            .timeout_minutes(15u32)
            .permissions(Permissions::default().contents(Level::Write))
            .add_step(steps::download_artifact().path("release-artifacts"))
            .add_step(publish),
    )
}

pub(crate) struct ReleaseBundleJobs {
    pub linux_aarch64: NamedJob,
    pub linux_x86_64: NamedJob,
    pub bwrap_linux_aarch64: NamedJob,
    pub bwrap_linux_x86_64: NamedJob,
    pub mac_aarch64: NamedJob,
    pub mac_x86_64: NamedJob,
    pub windows_aarch64: NamedJob,
    pub windows_x86_64: NamedJob,
}

impl ReleaseBundleJobs {
    pub fn jobs(&self) -> Vec<&NamedJob> {
        vec![
            &self.linux_aarch64,
            &self.linux_x86_64,
            &self.bwrap_linux_aarch64,
            &self.bwrap_linux_x86_64,
            &self.mac_aarch64,
            &self.mac_x86_64,
            &self.windows_aarch64,
            &self.windows_x86_64,
        ]
    }

    pub fn into_jobs(self) -> Vec<NamedJob> {
        vec![
            self.linux_aarch64,
            self.linux_x86_64,
            self.bwrap_linux_aarch64,
            self.bwrap_linux_x86_64,
            self.mac_aarch64,
            self.mac_x86_64,
            self.windows_aarch64,
            self.windows_x86_64,
        ]
    }
}

pub(crate) fn create_sentry_release() -> Step<Use> {
    named::uses(
        "getsentry",
        "action-release",
        "526942b68292201ac6bbb99b9a0747d4abee354c", // v3
    )
    .add_env(("SENTRY_ORG", "zed-dev"))
    .add_env(("SENTRY_PROJECT", "zed"))
    .add_env(("SENTRY_AUTH_TOKEN", vars::SENTRY_AUTH_TOKEN))
    .add_with(("environment", "production"))
}

pub(crate) const COMPLIANCE_REPORT_PATH: &str = "compliance-report-${GITHUB_REF_NAME}.md";
pub(crate) const COMPLIANCE_REPORT_ARTIFACT_PATH: &str =
    "compliance-report-${{ github.ref_name }}.md";
pub(crate) const COMPLIANCE_STEP_ID: &str = "run-compliance-check";
const NEEDS_REVIEW_PULLS_URL: &str = "https://github.com/simtropolis/zed/pulls?q=is%3Apr+is%3Aclosed+label%3A%22PR+state%3Aneeds+review%22";

pub(crate) enum ComplianceContext {
    Scheduled { tag_source: StepOutput },
}

pub(crate) fn add_compliance_steps(
    job: gh_workflow::Job,
    context: ComplianceContext,
) -> (gh_workflow::Job, StepOutput) {
    let ComplianceContext::Scheduled { tag_source } = context;
    let tag_source = tag_source.to_string();
    let compliance_step = named::bash(formatdoc! {r#"
        cargo xtask compliance version "$LATEST_TAG" --branch main --report-path "{COMPLIANCE_REPORT_PATH}"
    "#})
    .id(COMPLIANCE_STEP_ID)
    .add_env(("GITHUB_APP_ID", vars::ZED_ZIPPY_APP_ID))
    .add_env(("GITHUB_APP_KEY", vars::ZED_ZIPPY_APP_PRIVATE_KEY))
    .add_env(("LATEST_TAG", tag_source.clone()))
    .continue_on_error(true);
    let check_result = StepOutput::new_unchecked(&compliance_step, "outcome");

    let notification_script = formatdoc! {r#"
        if [ "$COMPLIANCE_OUTCOME" == "success" ]; then
            STATUS="✅ Scheduled compliance check passed for $COMPLIANCE_TAG"
            MESSAGE=$(printf "%s\n\nReport: %s" "$STATUS" "$ARTIFACT_URL")
        else
            STATUS="⚠️ Scheduled compliance check failed for $COMPLIANCE_TAG"
            MESSAGE=$(printf "%s\n\nReport: %s\nPRs needing review: %s" "$STATUS" "$ARTIFACT_URL" "{NEEDS_REVIEW_PULLS_URL}")
        fi

        curl -X POST -H 'Content-type: application/json' \
            --data "$(jq -n --arg text "$MESSAGE" '{{"text": $text}}')" \
            "$SLACK_WEBHOOK"
    "#};

    let notification_step = Step::new("send_compliance_slack_notification")
        .run(notification_script)
        .if_condition(Expression::new("${{ always() }}"))
        .add_env(("SLACK_WEBHOOK", vars::SLACK_WEBHOOK_WORKFLOW_FAILURES))
        .add_env((
            "COMPLIANCE_OUTCOME",
            format!("${{{{ steps.{COMPLIANCE_STEP_ID}.outcome }}}}"),
        ))
        .add_env(("COMPLIANCE_TAG", tag_source))
        .add_env((
            "ARTIFACT_URL",
            format!("{CURRENT_ACTION_RUN_URL}#artifacts"),
        ));

    (
        job.add_step(compliance_step)
            .add_step(
                upload_artifact(COMPLIANCE_REPORT_ARTIFACT_PATH)
                    .if_condition(Expression::new("always()")),
            )
            .add_step(notification_step),
        check_result,
    )
}

pub(crate) fn download_workflow_artifacts() -> DownloadArtifactStep {
    steps::download_artifact().path("./artifacts/")
}

pub(crate) fn prep_release_artifacts() -> Step<Run> {
    let mut script_lines = vec!["mkdir -p release-artifacts/\n".to_string()];
    for asset in assets::all() {
        let mv_command = format!("mv ./artifacts/{asset}/{asset} release-artifacts/{asset}");
        script_lines.push(mv_command)
    }

    named::bash(&script_lines.join("\n"))
}

pub(crate) fn notify_on_failure(deps: &[&NamedJob]) -> NamedJob {
    let failure_message = format!("❌ ${{{{ github.workflow }}}} failed: {CURRENT_ACTION_RUN_URL}");
    let notification = named::bash(
        r#"curl -X POST -H 'Content-type: application/json' --data "$(jq -n --arg text "$SLACK_MESSAGE" '{"text": $text}')" "$SLACK_WEBHOOK""#,
    )
    .add_env(("SLACK_WEBHOOK", vars::SLACK_WEBHOOK_WORKFLOW_FAILURES))
    .add_env(("SLACK_MESSAGE", failure_message));

    named::job(
        dependant_job(deps)
            .runs_on(runners::LINUX_SMALL)
            .cond(Expression::new("failure()"))
            .add_step(notification),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_product_release_builds_then_publishes() -> anyhow::Result<()> {
        let workflow = release()
            .to_string()
            .map_err(|error| anyhow::anyhow!("failed to serialize release workflow: {error:?}"))?;

        assert_eq!(workflow.matches("runs-on:").count(), 2);
        assert!(workflow.contains("runner: ubuntu-22.04"));
        assert!(workflow.contains("runner: macos-15"));
        assert!(workflow.contains("runner: windows-2022"));
        assert!(workflow.contains("cargo xtask bundle --product"));
        assert!(workflow.contains("product: rust"));
        assert!(workflow.contains("target: x86_64-unknown-linux-gnu"));
        assert!(workflow.contains("target: aarch64-apple-darwin"));
        assert!(workflow.contains("target: x86_64-pc-windows-msvc"));
        assert!(workflow.contains("git config --global core.longpaths true"));
        assert!(workflow.contains("CC: clang"));
        assert!(workflow.contains("CXX: clang++"));
        assert!(workflow.contains("needs:\n    - product_builds"));
        assert_eq!(workflow.matches("contents: write").count(), 1);
        let rust = ProductManifest::load()?
            .product("rust")?
            .display_name
            .clone();
        assert!(workflow.contains(&format!("TITLE=\"{rust} ${{TAG#rust-}}\"")));

        for artifact in [
            "rust-*-linux-x86_64.tar.gz",
            "rust-*-macos-aarch64.dmg",
            "rust-*-windows-x86_64.exe",
        ] {
            assert!(workflow.contains(artifact), "missing {artifact}");
        }

        for forbidden in [
            "simtropolis/zed",
            "repository_owner",
            "namespace-profile",
            "self-32vcpu",
            "R2_ACCOUNT_ID",
            "SENTRY_AUTH_TOKEN",
            "SLACK_WEBHOOK",
            "compliance",
            "Rustlings",
        ] {
            assert!(
                !workflow.contains(forbidden),
                "found {forbidden} in release"
            );
        }

        Ok(())
    }
}
