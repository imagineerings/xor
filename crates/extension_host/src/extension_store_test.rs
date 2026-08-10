use crate::{
    COMFY_COMPONENT_BINARY_FILE, COMFY_COMPONENT_MANIFEST_FILE, ComponentLifecycleAdapter, Event,
    ExtensionIndex, ExtensionIndexEntry, ExtensionIndexLanguageEntry, ExtensionIndexThemeEntry,
    ExtensionManifest, ExtensionStore, GrammarManifestEntry, InstalledComponent,
    RELOAD_DEBOUNCE_DURATION, RegisteredComponentAdapters, SchemaVersion,
    register_component_lifecycle_adapter, validate_component_file_metadata,
};
use async_compression::futures::bufread::GzipEncoder;
use collections::{BTreeMap, HashSet};
use extension::ExtensionHostProxy;
use fs::{FakeFs, Fs, RealFs};
use futures::{AsyncReadExt, FutureExt, StreamExt, future::BoxFuture, io::BufReader};
use gpui::{AppContext as _, BackgroundExecutor, TaskExt, TestAppContext};
use http_client::{FakeHttpClient, Response};
use language::{BinaryStatus, LanguageMatcher, LanguageName, LanguageRegistry};
use language_extension::LspAccess;
use lsp::LanguageServerName;
use node_runtime::NodeRuntime;
use parking_lot::Mutex;
use project::{DEFAULT_COMPLETION_CONTEXT, Project};
use release_channel::AppVersion;
use reqwest_client::ReqwestClient;
use serde_json::json;
use settings::SettingsStore;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};
use theme::ThemeRegistry;
use util::{rel_path::rel_path_buf, test::TempTree};

#[cfg(test)]
#[ctor::ctor(unsafe)]
fn init_logger() {
    zlog::init_test();
}

fn remote_sync_entry(id: &str, manifest_body: &str) -> ExtensionIndexEntry {
    remote_sync_entry_with_version(id, "1.0.0", manifest_body)
}

fn remote_sync_entry_with_version(
    id: &str,
    version: &str,
    manifest_body: &str,
) -> ExtensionIndexEntry {
    let id = toml::Value::String(id.to_owned()).to_string();
    let version = toml::Value::String(version.to_owned()).to_string();
    let manifest = format!(
        r#"
        id = {id}
        name = {id}
        version = {version}
        schema_version = 0

        {manifest_body}
        "#
    );

    ExtensionIndexEntry {
        manifest: Arc::new(toml::from_str(&manifest).unwrap()),
        dev: false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecordedComponent {
    extension_id: String,
    extension_version: String,
    manifest_bytes: Vec<u8>,
    component_bytes: Vec<u8>,
}

#[derive(Clone)]
struct RecordingComponentAdapter {
    snapshots: Arc<Mutex<Vec<Vec<RecordedComponent>>>>,
    rejected_manifest: Option<Vec<u8>>,
}

impl RecordingComponentAdapter {
    fn new() -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(Vec::new())),
            rejected_manifest: None,
        }
    }

    fn rejecting(rejected_manifest: Vec<u8>) -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(Vec::new())),
            rejected_manifest: Some(rejected_manifest),
        }
    }

    fn snapshots(&self) -> Vec<Vec<RecordedComponent>> {
        self.snapshots.lock().clone()
    }
}

impl ComponentLifecycleAdapter for RecordingComponentAdapter {
    fn adapter_id(&self) -> &'static str {
        "test.component-lifecycle"
    }

    fn synchronize(
        &self,
        components: Vec<InstalledComponent>,
    ) -> BoxFuture<'static, Result<(), String>> {
        let snapshots = self.snapshots.clone();
        let rejected_manifest = self.rejected_manifest.clone();
        Box::pin(async move {
            if let Some(rejected_manifest) = rejected_manifest
                && components
                    .iter()
                    .any(|component| component.manifest_bytes() == rejected_manifest)
            {
                return Err("component verification failed".to_owned());
            }
            snapshots.lock().push(
                components
                    .into_iter()
                    .map(|component| RecordedComponent {
                        extension_id: component.extension_id().to_owned(),
                        extension_version: component.extension_version().to_owned(),
                        manifest_bytes: component.manifest_bytes().to_vec(),
                        component_bytes: component.component_bytes().to_vec(),
                    })
                    .collect(),
            );
            Ok(())
        })
    }
}

fn remote_sync_language_entry(extension: &str, path: &str) -> ExtensionIndexLanguageEntry {
    ExtensionIndexLanguageEntry {
        extension: extension.into(),
        path: path.into(),
        matcher: LanguageMatcher::default(),
        hidden: false,
        grammar: None,
    }
}

fn remote_sync_extension_ids(index: &ExtensionIndex) -> Vec<String> {
    let mut extensions = index
        .extensions_to_sync_to_remote()
        .into_entries()
        .map(|(id, _)| id.to_string())
        .collect::<Vec<_>>();

    extensions.sort();

    extensions
}

#[gpui::test]
async fn component_inventory_is_fixed_bounded_and_symlink_free(cx: &mut TestAppContext) {
    assert_eq!(COMFY_COMPONENT_MANIFEST_FILE, "comfy-plugin.json");
    assert_eq!(COMFY_COMPONENT_BINARY_FILE, "comfy-plugin.wasm");
    let fs = FakeFs::new(cx.executor());
    fs.insert_tree(
        "/component-extensions/installed/alpha",
        json!({
            "comfy-plugin.json": r#"{"identifier":"alpha"}"#,
            "comfy-plugin.wasm": "component-bytes",
        }),
    )
    .await;
    let entries = [remote_sync_entry("alpha", "")];
    let components = ExtensionStore::load_installed_components(
        fs.clone(),
        Path::new("/component-extensions/installed"),
        &entries,
    )
    .await
    .expect("load fixed component pair");
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].extension_id(), "alpha");
    assert_eq!(components[0].extension_version(), "1.0.0");
    assert_eq!(components[0].manifest_bytes(), br#"{"identifier":"alpha"}"#);
    assert_eq!(components[0].component_bytes(), b"component-bytes");

    fs.insert_tree(
        "/component-extensions/installed/missing-pair",
        json!({
            "comfy-plugin.json": r#"{"identifier":"missing-pair"}"#,
        }),
    )
    .await;
    let missing_pair = [remote_sync_entry("missing-pair", "")];
    let error = ExtensionStore::load_installed_components(
        fs.clone(),
        Path::new("/component-extensions/installed"),
        &missing_pair,
    )
    .await
    .err()
    .expect("component inventory must reject an incomplete fixed pair");
    assert!(error.to_string().contains("must provide both"));

    fs.insert_tree(
        "/component-extensions/installed/symlinked",
        json!({
            "comfy-plugin.json": r#"{"identifier":"symlinked"}"#,
        }),
    )
    .await;
    fs.insert_tree(
        "/component-extensions/source",
        json!({"component.wasm": "outside-component"}),
    )
    .await;
    fs.insert_symlink(
        "/component-extensions/installed/symlinked/comfy-plugin.wasm",
        PathBuf::from("/component-extensions/source/component.wasm"),
    )
    .await;
    let symlinked = [remote_sync_entry("symlinked", "")];
    let error = ExtensionStore::load_installed_components(
        fs.clone(),
        Path::new("/component-extensions/installed"),
        &symlinked,
    )
    .await
    .err()
    .expect("component inventory must reject symlinked component bytes");
    assert!(error.to_string().contains("is invalid"));

    fs.insert_tree(
        "/component-extensions/outside/symlink-parent",
        json!({
            "comfy-plugin.json": r#"{"identifier":"symlink-parent"}"#,
            "comfy-plugin.wasm": "outside-component",
        }),
    )
    .await;
    fs.insert_symlink(
        "/component-extensions/installed/symlink-parent",
        PathBuf::from("/component-extensions/outside/symlink-parent"),
    )
    .await;
    let symlink_parent = [remote_sync_entry("symlink-parent", "")];
    let error = ExtensionStore::load_installed_components(
        fs.clone(),
        Path::new("/component-extensions/installed"),
        &symlink_parent,
    )
    .await
    .err()
    .expect("component inventory must reject a symlinked extension directory");
    assert!(error.to_string().contains("not a direct real directory"));

    for extension_id in ["../escape", "nested/escape", r"nested\escape"] {
        let traversal = [remote_sync_entry(extension_id, "")];
        let error = ExtensionStore::load_installed_components(
            fs.clone(),
            Path::new("/component-extensions/installed"),
            &traversal,
        )
        .await
        .err()
        .expect("component inventory must reject a non-component extension identifier");
        assert!(
            error.to_string().contains("one normal path component"),
            "unexpected traversal error for {extension_id}: {error}"
        );
    }

    let oversized_manifest_dir = Path::new("/component-extensions/installed/oversized-manifest");
    fs.create_dir(oversized_manifest_dir)
        .await
        .expect("create oversized manifest extension directory");
    fs.insert_file(
        oversized_manifest_dir.join("comfy-plugin.json"),
        vec![
            b'x';
            usize::try_from(crate::MAXIMUM_COMFY_COMPONENT_MANIFEST_BYTES + 1)
                .expect("manifest limit fits usize")
        ],
    )
    .await;
    fs.insert_file(
        oversized_manifest_dir.join("comfy-plugin.wasm"),
        b"component".to_vec(),
    )
    .await;
    let oversized_manifest = [remote_sync_entry("oversized-manifest", "")];
    let error = ExtensionStore::load_installed_components(
        fs.clone(),
        Path::new("/component-extensions/installed"),
        &oversized_manifest,
    )
    .await
    .err()
    .expect("component inventory must reject an oversized manifest");
    assert!(error.to_string().contains("is oversized"));

    let metadata = fs
        .metadata(Path::new(
            "/component-extensions/installed/alpha/comfy-plugin.wasm",
        ))
        .await
        .expect("read component metadata")
        .expect("component binary exists");
    let mut changed_metadata = metadata;
    changed_metadata.len += 1;
    let error = validate_component_file_metadata(
        &metadata,
        &changed_metadata,
        changed_metadata.len,
        crate::MAXIMUM_COMFY_COMPONENT_BINARY_BYTES,
        "component binary",
        "alpha",
    )
    .expect_err("component inventory must reject post-read identity changes");
    assert!(
        error
            .to_string()
            .contains("changed while it was being loaded")
    );

    let error = validate_component_file_metadata(
        &metadata,
        &metadata,
        crate::MAXIMUM_COMFY_COMPONENT_BINARY_BYTES + 1,
        crate::MAXIMUM_COMFY_COMPONENT_BINARY_BYTES,
        "component binary",
        "alpha",
    )
    .expect_err("component inventory must enforce bounds after reading bytes");
    assert!(error.to_string().contains("is oversized"));
}

#[test]
fn installed_component_constructor_owns_identity_and_payload_bounds() {
    let empty_bytes: Arc<[u8]> = Vec::new().into();
    let error = InstalledComponent::checked(
        "../escape".into(),
        "1.0.0".into(),
        empty_bytes.clone(),
        empty_bytes.clone(),
    )
    .err()
    .expect("installed component must reject a non-component identifier");
    assert!(error.to_string().contains("one normal path component"));

    for invalid_identifier in ["", ".", "..", "a/b", "a\\b", "a:b", "NUL", "trailing."] {
        let error = InstalledComponent::checked(
            invalid_identifier.into(),
            "1.0.0".into(),
            empty_bytes.clone(),
            empty_bytes.clone(),
        )
        .err()
        .expect("non-portable component identities must be rejected");
        assert!(error.to_string().contains("one normal path component"));
    }

    InstalledComponent::checked(
        "插件".into(),
        "1.0.0".into(),
        empty_bytes.clone(),
        empty_bytes,
    )
    .expect("one normal non-ASCII path component is a valid canonical identity");

    crate::validate_component_payload_length(
        crate::MAXIMUM_COMFY_COMPONENT_MANIFEST_BYTES,
        crate::MAXIMUM_COMFY_COMPONENT_MANIFEST_BYTES,
        "component manifest",
        "alpha",
    )
    .expect("manifest payload at the canonical bound is valid");
    let error = crate::validate_component_payload_length(
        crate::MAXIMUM_COMFY_COMPONENT_MANIFEST_BYTES + 1,
        crate::MAXIMUM_COMFY_COMPONENT_MANIFEST_BYTES,
        "component manifest",
        "alpha",
    )
    .expect_err("manifest payload over the canonical bound must fail");
    assert!(error.to_string().contains("is oversized"));

    let error = crate::validate_component_payload_length(
        crate::MAXIMUM_COMFY_COMPONENT_BINARY_BYTES + 1,
        crate::MAXIMUM_COMFY_COMPONENT_BINARY_BYTES,
        "component binary",
        "alpha",
    )
    .expect_err("component payload over the canonical bound must fail");
    assert!(error.to_string().contains("is oversized"));
}

#[test]
fn extension_lifecycle_mutations_share_one_checked_destination_mapper() {
    let installed = Path::new("/extensions/installed");
    assert_eq!(
        crate::checked_extension_dir(installed, "theme-pack")
            .expect("normal extension identifier must map below the installed root"),
        installed.join("theme-pack")
    );
    for extension_id in [
        "",
        ".",
        "..",
        "../escape",
        "nested/escape",
        r"nested\escape",
        "CON",
        "aux.txt",
        "trailing.",
    ] {
        let error = crate::checked_extension_dir(installed, extension_id)
            .expect_err("unsafe extension identifier must fail before a lifecycle mutation");
        assert!(
            error.to_string().contains("one normal path component"),
            "unexpected destination error for {extension_id:?}: {error}"
        );
    }
}

#[gpui::test]
async fn registered_component_adapter_tracks_lifecycle_and_restart(cx: &mut TestAppContext) {
    let fs = FakeFs::new(cx.executor());
    let installed_dir = Path::new("/component-lifecycle/installed");
    fs.insert_tree(
        installed_dir.join("alpha"),
        json!({
            "comfy-plugin.json": "manifest-v1",
            "comfy-plugin.wasm": "component-v1",
        }),
    )
    .await;

    let adapter = Arc::new(RecordingComponentAdapter::new());
    cx.update(|cx| {
        register_component_lifecycle_adapter(adapter.clone(), cx)
            .expect("register component lifecycle adapter")
    });
    let adapters = cx.update(|cx| cx.global::<RegisteredComponentAdapters>().0.clone());

    let mut entries = vec![remote_sync_entry_with_version("alpha", "1.0.0", "")];
    let errors = ExtensionStore::synchronize_component_adapters(
        fs.clone(),
        installed_dir,
        &entries,
        &adapters,
    )
    .await;
    assert!(errors.is_empty());
    assert_eq!(
        adapter.snapshots(),
        vec![vec![RecordedComponent {
            extension_id: "alpha".to_owned(),
            extension_version: "1.0.0".to_owned(),
            manifest_bytes: b"manifest-v1".to_vec(),
            component_bytes: b"component-v1".to_vec(),
        }]]
    );

    fs.insert_file(
        installed_dir.join("alpha/comfy-plugin.json"),
        b"manifest-v2".to_vec(),
    )
    .await;
    fs.insert_file(
        installed_dir.join("alpha/comfy-plugin.wasm"),
        b"component-v2".to_vec(),
    )
    .await;
    entries = vec![remote_sync_entry_with_version("alpha", "2.0.0", "")];
    let errors = ExtensionStore::synchronize_component_adapters(
        fs.clone(),
        installed_dir,
        &entries,
        &adapters,
    )
    .await;
    assert!(errors.is_empty());
    assert_eq!(
        adapter.snapshots().last(),
        Some(&vec![RecordedComponent {
            extension_id: "alpha".to_owned(),
            extension_version: "2.0.0".to_owned(),
            manifest_bytes: b"manifest-v2".to_vec(),
            component_bytes: b"component-v2".to_vec(),
        }])
    );

    let restarted_adapter = Arc::new(RecordingComponentAdapter::new());
    let restarted_adapters: Vec<Arc<dyn ComponentLifecycleAdapter>> =
        vec![restarted_adapter.clone()];
    let errors = ExtensionStore::synchronize_component_adapters(
        fs.clone(),
        installed_dir,
        &entries,
        &restarted_adapters,
    )
    .await;
    assert!(errors.is_empty());
    assert_eq!(
        restarted_adapter.snapshots(),
        vec![vec![RecordedComponent {
            extension_id: "alpha".to_owned(),
            extension_version: "2.0.0".to_owned(),
            manifest_bytes: b"manifest-v2".to_vec(),
            component_bytes: b"component-v2".to_vec(),
        }]]
    );

    let errors =
        ExtensionStore::synchronize_component_adapters(fs, installed_dir, &[], &adapters).await;
    assert!(errors.is_empty());
    assert_eq!(adapter.snapshots().last(), Some(&Vec::new()));
}

#[gpui::test]
async fn component_adapter_failures_converge_without_partial_verification(cx: &mut TestAppContext) {
    let fs = FakeFs::new(cx.executor());
    let installed_dir = Path::new("/component-failures/installed");
    fs.insert_tree(
        installed_dir.join("alpha"),
        json!({
            "comfy-plugin.json": "accepted-manifest",
            "comfy-plugin.wasm": "component",
        }),
    )
    .await;
    let entries = vec![remote_sync_entry("alpha", "")];
    let adapter = Arc::new(RecordingComponentAdapter::new());
    let adapters: Vec<Arc<dyn ComponentLifecycleAdapter>> = vec![adapter.clone()];

    let errors = ExtensionStore::synchronize_component_adapters(
        fs.clone(),
        installed_dir,
        &entries,
        &adapters,
    )
    .await;
    assert!(errors.is_empty());

    let missing_pair = vec![remote_sync_entry("missing-pair", "")];
    fs.insert_tree(
        installed_dir.join("missing-pair"),
        json!({"comfy-plugin.json": "manifest"}),
    )
    .await;
    let errors = ExtensionStore::synchronize_component_adapters(
        fs.clone(),
        installed_dir,
        &missing_pair,
        &adapters,
    )
    .await;
    assert!(
        errors
            .get(adapter.adapter_id())
            .is_some_and(|error| error.contains("must provide both"))
    );
    assert_eq!(adapter.snapshots().last(), Some(&Vec::new()));

    let errors = ExtensionStore::synchronize_component_adapters(
        fs.clone(),
        installed_dir,
        &entries,
        &adapters,
    )
    .await;
    assert!(errors.is_empty());
    assert_eq!(adapter.snapshots().last().map(Vec::len), Some(1));

    let rejecting_adapter = Arc::new(RecordingComponentAdapter::rejecting(
        b"rejected-manifest".to_vec(),
    ));
    let rejecting_adapters: Vec<Arc<dyn ComponentLifecycleAdapter>> =
        vec![rejecting_adapter.clone()];
    let errors = ExtensionStore::synchronize_component_adapters(
        fs.clone(),
        installed_dir,
        &entries,
        &rejecting_adapters,
    )
    .await;
    assert!(errors.is_empty());
    let accepted_snapshots = rejecting_adapter.snapshots();

    fs.insert_file(
        installed_dir.join("alpha/comfy-plugin.json"),
        b"rejected-manifest".to_vec(),
    )
    .await;
    let errors = ExtensionStore::synchronize_component_adapters(
        fs.clone(),
        installed_dir,
        &entries,
        &rejecting_adapters,
    )
    .await;
    assert_eq!(
        errors.get(rejecting_adapter.adapter_id()),
        Some(&"component verification failed".to_owned())
    );
    assert_eq!(
        rejecting_adapter.snapshots(),
        accepted_snapshots,
        "verification failures must preserve the adapter's prior atomic state"
    );

    let errors =
        ExtensionStore::synchronize_component_adapters(fs, installed_dir, &[], &rejecting_adapters)
            .await;
    assert!(errors.is_empty());
    assert_eq!(rejecting_adapter.snapshots().last(), Some(&Vec::new()));
}

#[gpui::test]
async fn component_adapter_failure_is_returned_by_reload(cx: &mut TestAppContext) {
    init_test(cx);
    let fs = FakeFs::new(cx.executor());
    let extensions_dir = PathBuf::from("/component-reload-result");
    fs.insert_tree(
        extensions_dir.join("installed/alpha"),
        json!({
            "extension.json": r#"{
                "id": "alpha",
                "name": "Alpha",
                "version": "1.0.0"
            }"#,
            "comfy-plugin.json": "rejected-manifest",
            "comfy-plugin.wasm": "component",
        }),
    )
    .await;

    let proxy = Arc::new(ExtensionHostProxy::new());
    let theme_registry = Arc::new(ThemeRegistry::new(Box::new(())));
    theme_extension::init(proxy.clone(), theme_registry, cx.executor());
    let language_registry = Arc::new(LanguageRegistry::test(cx.executor()));
    language_extension::init(LspAccess::Noop, proxy.clone(), language_registry);
    let http_client = FakeHttpClient::with_200_response();
    let adapter = Arc::new(RecordingComponentAdapter::rejecting(
        b"rejected-manifest".to_vec(),
    ));
    let store = cx.new(|cx| {
        let mut store = ExtensionStore::new(
            extensions_dir,
            None,
            proxy,
            fs,
            http_client.clone(),
            http_client,
            None,
            NodeRuntime::unavailable(),
            cx,
        );
        store.component_lifecycle_adapters = vec![adapter];
        store
    });

    cx.executor().advance_clock(RELOAD_DEBOUNCE_DURATION);
    cx.executor().run_until_parked();
    let reload = store.update(cx, |store, cx| store.reload(None, cx));
    cx.executor().advance_clock(RELOAD_DEBOUNCE_DURATION);
    let error = reload
        .await
        .expect_err("adapter verification failure must fail extension reload");
    assert!(error.to_string().contains("component verification failed"));
    store.read_with(cx, |store, _| {
        assert_eq!(
            store
                .component_adapter_errors()
                .get("test.component-lifecycle"),
            Some(&"component verification failed".to_owned())
        );
    });
}

#[test]
fn remote_sync_includes_language_dependencies() {
    let index = ExtensionIndex {
        extensions: [
            (
                "bar-language".into(),
                remote_sync_entry("bar-language", r#"languages = ["languages/bar"]"#),
            ),
            (
                "foo-lsp".into(),
                remote_sync_entry(
                    "foo-lsp",
                    r#"
                    [language_servers.foo]
                    language = "Foo"
                    "#,
                ),
            ),
            (
                "foo-language".into(),
                remote_sync_entry("foo-language", r#"languages = ["languages/foo"]"#),
            ),
        ]
        .into_iter()
        .collect(),
        languages: [
            (
                "Bar".into(),
                remote_sync_language_entry("bar-language", "languages/bar"),
            ),
            (
                "Foo".into(),
                remote_sync_language_entry("foo-language", "languages/foo"),
            ),
        ]
        .into_iter()
        .collect(),
        themes: BTreeMap::default(),
        icon_themes: BTreeMap::default(),
    };

    assert_eq!(
        remote_sync_extension_ids(&index),
        ["foo-language", "foo-lsp"]
    );
}

#[test]
fn remote_sync_keeps_shared_language_dependency_once() {
    let index = ExtensionIndex {
        extensions: [
            (
                "aaa-lsp".into(),
                remote_sync_entry(
                    "aaa-lsp",
                    r#"
                    [language_servers.aaa]
                    language = "Foo"
                    "#,
                ),
            ),
            (
                "bbb-lsp".into(),
                remote_sync_entry(
                    "bbb-lsp",
                    r#"
                    [language_servers.bbb]
                    language = "Foo"
                    "#,
                ),
            ),
            (
                "zzz-language".into(),
                remote_sync_entry("zzz-language", r#"languages = ["languages/foo"]"#),
            ),
        ]
        .into_iter()
        .collect(),
        languages: [(
            "Foo".into(),
            remote_sync_language_entry("zzz-language", "languages/foo"),
        )]
        .into_iter()
        .collect(),
        themes: BTreeMap::default(),
        icon_themes: BTreeMap::default(),
    };

    assert_eq!(
        remote_sync_extension_ids(&index),
        ["aaa-lsp", "bbb-lsp", "zzz-language"]
    );
}

#[test]
fn remote_sync_keeps_remote_loadable_extensions_without_language_dependency() {
    let index = ExtensionIndex {
        extensions: [(
            "foo".into(),
            remote_sync_entry(
                "foo",
                r#"
                [language_servers.foo]
                language = "Foo"
                "#,
            ),
        )]
        .into_iter()
        .collect(),
        languages: BTreeMap::default(),
        themes: BTreeMap::default(),
        icon_themes: BTreeMap::default(),
    };

    assert_eq!(remote_sync_extension_ids(&index), ["foo"]);
}

#[test]
fn remote_sync_keeps_debug_adapters() {
    let index = ExtensionIndex {
        extensions: [(
            "foo".into(),
            remote_sync_entry(
                "foo",
                r#"
                [debug_adapters.foo]
                "#,
            ),
        )]
        .into_iter()
        .collect(),
        languages: BTreeMap::default(),
        themes: BTreeMap::default(),
        icon_themes: BTreeMap::default(),
    };

    assert_eq!(remote_sync_extension_ids(&index), ["foo"]);
}

#[gpui::test]
async fn test_extension_store(cx: &mut TestAppContext) {
    init_test(cx);

    let fs = FakeFs::new(cx.executor());
    let http_client = FakeHttpClient::with_200_response();

    fs.insert_tree(
        "/the-extension-dir",
        json!({
            "installed": {
                "sim-monokai": {
                    "extension.json": r#"{
                        "id": "sim-monokai",
                        "name": "Sim Monokai",
                        "version": "2.0.0",
                        "themes": {
                            "Monokai Dark": "themes/monokai.json",
                            "Monokai Light": "themes/monokai.json",
                            "Monokai Pro Dark": "themes/monokai-pro.json",
                            "Monokai Pro Light": "themes/monokai-pro.json"
                        }
                    }"#,
                    "themes": {
                        "monokai.json": r#"{
                            "name": "Monokai",
                            "author": "Someone",
                            "themes": [
                                {
                                    "name": "Monokai Dark",
                                    "appearance": "dark",
                                    "style": {}
                                },
                                {
                                    "name": "Monokai Light",
                                    "appearance": "light",
                                    "style": {}
                                }
                            ]
                        }"#,
                        "monokai-pro.json": r#"{
                            "name": "Monokai Pro",
                            "author": "Someone",
                            "themes": [
                                {
                                    "name": "Monokai Pro Dark",
                                    "appearance": "dark",
                                    "style": {}
                                },
                                {
                                    "name": "Monokai Pro Light",
                                    "appearance": "light",
                                    "style": {}
                                }
                            ]
                        }"#,
                    }
                },
                "sim-ruby": {
                    "extension.json": r#"{
                        "id": "sim-ruby",
                        "name": "Sim Ruby",
                        "version": "1.0.0",
                        "grammars": {
                            "ruby": "grammars/ruby.wasm",
                            "embedded_template": "grammars/embedded_template.wasm"
                        },
                        "languages": {
                            "ruby": "languages/ruby",
                            "erb": "languages/erb"
                        }
                    }"#,
                    "grammars": {
                        "ruby.wasm": "",
                        "embedded_template.wasm": "",
                    },
                    "languages": {
                        "ruby": {
                            "config.toml": r#"
                                name = "Ruby"
                                grammar = "ruby"
                                path_suffixes = ["rb"]
                            "#,
                            "highlights.scm": "",
                        },
                        "erb": {
                            "config.toml": r#"
                                name = "ERB"
                                grammar = "embedded_template"
                                path_suffixes = ["erb"]
                            "#,
                            "highlights.scm": "",
                        }
                    },
                }
            }
        }),
    )
    .await;

    let mut expected_index = ExtensionIndex {
        extensions: [
            (
                "sim-ruby".into(),
                ExtensionIndexEntry {
                    manifest: Arc::new(ExtensionManifest {
                        id: "sim-ruby".into(),
                        name: "Sim Ruby".into(),
                        version: "1.0.0".into(),
                        schema_version: SchemaVersion::ZERO,
                        description: None,
                        authors: Vec::new(),
                        repository: None,
                        themes: Default::default(),
                        icon_themes: Vec::new(),
                        lib: Default::default(),
                        languages: vec![
                            rel_path_buf("languages/erb"),
                            rel_path_buf("languages/ruby"),
                        ],
                        grammars: [
                            ("embedded_template".into(), GrammarManifestEntry::default()),
                            ("ruby".into(), GrammarManifestEntry::default()),
                        ]
                        .into_iter()
                        .collect(),
                        language_servers: BTreeMap::default(),
                        context_servers: BTreeMap::default(),
                        slash_commands: BTreeMap::default(),
                        snippets: None,
                        capabilities: Vec::new(),
                        debug_adapters: Default::default(),
                        debug_locators: Default::default(),
                        language_model_providers: BTreeMap::default(),
                    }),
                    dev: false,
                },
            ),
            (
                "sim-monokai".into(),
                ExtensionIndexEntry {
                    manifest: Arc::new(ExtensionManifest {
                        id: "sim-monokai".into(),
                        name: "Sim Monokai".into(),
                        version: "2.0.0".into(),
                        schema_version: SchemaVersion::ZERO,
                        description: None,
                        authors: vec![],
                        repository: None,
                        themes: vec![
                            rel_path_buf("themes/monokai-pro.json"),
                            rel_path_buf("themes/monokai.json"),
                        ],
                        icon_themes: Vec::new(),
                        lib: Default::default(),
                        languages: Default::default(),
                        grammars: BTreeMap::default(),
                        language_servers: BTreeMap::default(),
                        context_servers: BTreeMap::default(),
                        slash_commands: BTreeMap::default(),
                        snippets: None,
                        capabilities: Vec::new(),
                        debug_adapters: Default::default(),
                        debug_locators: Default::default(),
                        language_model_providers: BTreeMap::default(),
                    }),
                    dev: false,
                },
            ),
        ]
        .into_iter()
        .collect(),
        languages: [
            (
                "ERB".into(),
                ExtensionIndexLanguageEntry {
                    extension: "sim-ruby".into(),
                    path: "languages/erb".into(),
                    grammar: Some("embedded_template".into()),
                    hidden: false,
                    matcher: LanguageMatcher {
                        path_suffixes: vec!["erb".into()],
                        first_line_pattern: None,
                        ..LanguageMatcher::default()
                    },
                },
            ),
            (
                "Ruby".into(),
                ExtensionIndexLanguageEntry {
                    extension: "sim-ruby".into(),
                    path: "languages/ruby".into(),
                    grammar: Some("ruby".into()),
                    hidden: false,
                    matcher: LanguageMatcher {
                        path_suffixes: vec!["rb".into()],
                        first_line_pattern: None,
                        ..LanguageMatcher::default()
                    },
                },
            ),
        ]
        .into_iter()
        .collect(),
        themes: [
            (
                "Monokai Dark".into(),
                ExtensionIndexThemeEntry {
                    extension: "sim-monokai".into(),
                    path: "themes/monokai.json".into(),
                },
            ),
            (
                "Monokai Light".into(),
                ExtensionIndexThemeEntry {
                    extension: "sim-monokai".into(),
                    path: "themes/monokai.json".into(),
                },
            ),
            (
                "Monokai Pro Dark".into(),
                ExtensionIndexThemeEntry {
                    extension: "sim-monokai".into(),
                    path: "themes/monokai-pro.json".into(),
                },
            ),
            (
                "Monokai Pro Light".into(),
                ExtensionIndexThemeEntry {
                    extension: "sim-monokai".into(),
                    path: "themes/monokai-pro.json".into(),
                },
            ),
        ]
        .into_iter()
        .collect(),
        icon_themes: BTreeMap::default(),
    };

    let proxy = Arc::new(ExtensionHostProxy::new());
    let theme_registry = Arc::new(ThemeRegistry::new(Box::new(())));
    theme_extension::init(proxy.clone(), theme_registry.clone(), cx.executor());
    let language_registry = Arc::new(LanguageRegistry::test(cx.executor()));
    language_extension::init(LspAccess::Noop, proxy.clone(), language_registry.clone());
    let node_runtime = NodeRuntime::unavailable();

    let store = cx.new(|cx| {
        ExtensionStore::new(
            PathBuf::from("/the-extension-dir"),
            None,
            proxy.clone(),
            fs.clone(),
            http_client.clone(),
            http_client.clone(),
            None,
            node_runtime.clone(),
            cx,
        )
    });

    cx.executor().advance_clock(RELOAD_DEBOUNCE_DURATION);
    store.read_with(cx, |store, _| {
        let index = &store.extension_index;
        assert_eq!(index.extensions, expected_index.extensions);

        for ((actual_key, actual_language), (expected_key, expected_language)) in
            index.languages.iter().zip(expected_index.languages.iter())
        {
            assert_eq!(actual_key, expected_key);
            assert_eq!(actual_language.grammar, expected_language.grammar);
            assert_eq!(actual_language.matcher, expected_language.matcher);
            assert_eq!(actual_language.hidden, expected_language.hidden);
        }
        assert_eq!(index.themes, expected_index.themes);

        assert_eq!(
            language_registry.language_names(),
            [
                LanguageName::new_static("ERB"),
                LanguageName::new_static("Plain Text"),
                LanguageName::new_static("Ruby"),
            ]
        );
        assert_eq!(
            theme_registry.list_names(),
            [
                "Monokai Dark",
                "Monokai Light",
                "Monokai Pro Dark",
                "Monokai Pro Light",
                "One Dark",
            ]
        );
    });

    fs.insert_tree(
        "/the-extension-dir/installed/sim-gruvbox",
        json!({
            "extension.json": r#"{
                "id": "sim-gruvbox",
                "name": "Sim Gruvbox",
                "version": "1.0.0",
                "themes": {
                    "Gruvbox": "themes/gruvbox.json"
                }
            }"#,
            "themes": {
                "gruvbox.json": r#"{
                    "name": "Gruvbox",
                    "author": "Someone Else",
                    "themes": [
                        {
                            "name": "Gruvbox",
                            "appearance": "dark",
                            "style": {}
                        }
                    ]
                }"#,
            }
        }),
    )
    .await;

    expected_index.extensions.insert(
        "sim-gruvbox".into(),
        ExtensionIndexEntry {
            manifest: Arc::new(ExtensionManifest {
                id: "sim-gruvbox".into(),
                name: "Sim Gruvbox".into(),
                version: "1.0.0".into(),
                schema_version: SchemaVersion::ZERO,
                description: None,
                authors: vec![],
                repository: None,
                themes: vec![rel_path_buf("themes/gruvbox.json")],
                icon_themes: Vec::new(),
                lib: Default::default(),
                languages: Default::default(),
                grammars: BTreeMap::default(),
                language_servers: BTreeMap::default(),
                context_servers: BTreeMap::default(),
                slash_commands: BTreeMap::default(),
                snippets: None,
                capabilities: Vec::new(),
                debug_adapters: Default::default(),
                debug_locators: Default::default(),
                language_model_providers: BTreeMap::default(),
            }),
            dev: false,
        },
    );
    expected_index.themes.insert(
        "Gruvbox".into(),
        ExtensionIndexThemeEntry {
            extension: "sim-gruvbox".into(),
            path: "themes/gruvbox.json".into(),
        },
    );

    let reload = store.update(cx, |store, cx| store.reload(None, cx));

    cx.executor().advance_clock(RELOAD_DEBOUNCE_DURATION);
    reload.await.expect("reload extension inventory");
    store.read_with(cx, |store, _| {
        let index = &store.extension_index;

        for ((actual_key, actual_language), (expected_key, expected_language)) in
            index.languages.iter().zip(expected_index.languages.iter())
        {
            assert_eq!(actual_key, expected_key);
            assert_eq!(actual_language.grammar, expected_language.grammar);
            assert_eq!(actual_language.matcher, expected_language.matcher);
            assert_eq!(actual_language.hidden, expected_language.hidden);
        }

        assert_eq!(index.extensions, expected_index.extensions);
        assert_eq!(index.themes, expected_index.themes);

        assert_eq!(
            theme_registry.list_names(),
            [
                "Gruvbox",
                "Monokai Dark",
                "Monokai Light",
                "Monokai Pro Dark",
                "Monokai Pro Light",
                "One Dark",
            ]
        );
    });

    let prev_fs_metadata_call_count = fs.metadata_call_count();
    let prev_fs_read_dir_call_count = fs.read_dir_call_count();

    // Create new extension store, as if Sim were restarting.
    drop(store);
    let store = cx.new(|cx| {
        ExtensionStore::new(
            PathBuf::from("/the-extension-dir"),
            None,
            proxy,
            fs.clone(),
            http_client.clone(),
            http_client.clone(),
            None,
            node_runtime.clone(),
            cx,
        )
    });

    cx.executor().run_until_parked();
    store.read_with(cx, |store, _| {
        assert_eq!(store.extension_index.extensions, expected_index.extensions);
        assert_eq!(store.extension_index.themes, expected_index.themes);
        assert_eq!(
            store.extension_index.icon_themes,
            expected_index.icon_themes
        );

        for ((actual_key, actual_language), (expected_key, expected_language)) in store
            .extension_index
            .languages
            .iter()
            .zip(expected_index.languages.iter())
        {
            assert_eq!(actual_key, expected_key);
            assert_eq!(actual_language.grammar, expected_language.grammar);
            assert_eq!(actual_language.matcher, expected_language.matcher);
            assert_eq!(actual_language.hidden, expected_language.hidden);
        }

        assert_eq!(
            language_registry.language_names(),
            [
                LanguageName::new_static("ERB"),
                LanguageName::new_static("Plain Text"),
                LanguageName::new_static("Ruby"),
            ]
        );
        assert_eq!(
            language_registry.grammar_names(),
            ["embedded_template".into(), "ruby".into()]
        );
        assert_eq!(
            theme_registry.list_names(),
            [
                "Gruvbox",
                "Monokai Dark",
                "Monokai Light",
                "Monokai Pro Dark",
                "Monokai Pro Light",
                "One Dark",
            ]
        );

        // The on-disk manifest limits the number of FS calls that need to be made
        // on startup.
        assert_eq!(fs.read_dir_call_count(), prev_fs_read_dir_call_count);
        assert_eq!(fs.metadata_call_count(), prev_fs_metadata_call_count + 2);
    });

    store.update(cx, |store, cx| {
        store
            .uninstall_extension("sim-ruby".into(), cx)
            .detach_and_log_err(cx);
    });

    cx.executor().advance_clock(RELOAD_DEBOUNCE_DURATION);
    expected_index.extensions.remove("sim-ruby");
    expected_index.languages.remove("Ruby");
    expected_index.languages.remove("ERB");

    store.read_with(cx, |store, _| {
        assert_eq!(store.extension_index.extensions, expected_index.extensions);
        assert_eq!(store.extension_index.themes, expected_index.themes);
        assert_eq!(
            store.extension_index.icon_themes,
            expected_index.icon_themes
        );

        for ((actual_key, actual_language), (expected_key, expected_language)) in store
            .extension_index
            .languages
            .iter()
            .zip(expected_index.languages.iter())
        {
            assert_eq!(actual_key, expected_key);
            assert_eq!(actual_language.grammar, expected_language.grammar);
            assert_eq!(actual_language.matcher, expected_language.matcher);
            assert_eq!(actual_language.hidden, expected_language.hidden);
        }

        assert_eq!(
            language_registry.language_names(),
            [LanguageName::new_static("Plain Text")]
        );
        assert_eq!(language_registry.grammar_names(), []);
    });
}

#[gpui::test]
async fn test_extension_store_with_test_extension(cx: &mut TestAppContext) {
    init_test(cx);
    cx.executor().allow_parking();

    let executor = cx.executor();
    async fn await_or_timeout<T>(
        executor: &BackgroundExecutor,
        what: &'static str,
        seconds: u64,
        future: impl std::future::Future<Output = T>,
    ) -> T {
        let timeout = executor.timer(std::time::Duration::from_secs(seconds));

        futures::select! {
            output = future.fuse() => output,
            _ = futures::FutureExt::fuse(timeout) => panic!(
            "[test_extension_store_with_test_extension] timed out after {seconds}s while {what}"
        )
        }
    }

    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let cache_dir = root_dir.join("target");
    let test_extension_id = "test-extension";
    let test_extension_dir = root_dir.join("extensions").join(test_extension_id);

    let fs = Arc::new(RealFs::new(None, cx.executor()));
    let extensions_tree = TempTree::new(json!({
        "installed": {},
        "work": {}
    }));
    let project_dir = TempTree::new(json!({
        "test.gleam": ""
    }));

    let extensions_dir = extensions_tree.path().canonicalize().unwrap();
    let project_dir = project_dir.path().canonicalize().unwrap();

    let project = await_or_timeout(
        &executor,
        "awaiting Project::test",
        5,
        Project::test(fs.clone(), [project_dir.as_path()], cx),
    )
    .await;

    let proxy = Arc::new(ExtensionHostProxy::new());
    let theme_registry = Arc::new(ThemeRegistry::new(Box::new(())));
    theme_extension::init(proxy.clone(), theme_registry.clone(), cx.executor());
    let language_registry = project.read_with(cx, |project, _cx| project.languages().clone());
    language_extension::init(
        LspAccess::ViaLspStore(
            project
                .update(cx, |project, _| project.lsp_store())
                .downgrade(),
        ),
        proxy.clone(),
        language_registry.clone(),
    );
    let node_runtime = NodeRuntime::unavailable();

    let mut status_updates = language_registry.language_server_binary_statuses();

    struct FakeLanguageServerVersion {
        version: String,
        binary_contents: String,
        http_request_count: usize,
    }

    let language_server_version = Arc::new(Mutex::new(FakeLanguageServerVersion {
        version: "v1.2.3".into(),
        binary_contents: "the-binary-contents".into(),
        http_request_count: 0,
    }));

    let extension_client = FakeHttpClient::create({
        let language_server_version = language_server_version.clone();
        move |request| {
            let language_server_version = language_server_version.clone();
            async move {
                let version = language_server_version.lock().version.clone();
                let binary_contents = language_server_version.lock().binary_contents.clone();

                let github_releases_uri = "https://api.github.com/repos/gleam-lang/gleam/releases";
                let asset_download_uri =
                    format!("https://fake-download.example.com/gleam-{version}");

                let uri = request.uri().to_string();
                if uri == github_releases_uri {
                    language_server_version.lock().http_request_count += 1;
                    Ok(Response::new(
                        json!([
                            {
                                "tag_name": version,
                                "prerelease": false,
                                "tarball_url": "",
                                "zipball_url": "",
                                "assets": [
                                    {
                                        "name": format!("gleam-{version}-aarch64-apple-darwin.tar.gz"),
                                        "browser_download_url": asset_download_uri
                                    },
                                    {
                                        "name": format!("gleam-{version}-x86_64-unknown-linux-musl.tar.gz"),
                                        "browser_download_url": asset_download_uri
                                    },
                                    {
                                        "name": format!("gleam-{version}-aarch64-unknown-linux-musl.tar.gz"),
                                        "browser_download_url": asset_download_uri
                                    },
                                    {
                                        "name": format!("gleam-{version}-x86_64-pc-windows-msvc.tar.gz"),
                                        "browser_download_url": asset_download_uri
                                    }
                                ]
                            }
                        ])
                        .to_string()
                        .into(),
                    ))
                } else if uri == asset_download_uri {
                    language_server_version.lock().http_request_count += 1;
                    let mut bytes = Vec::<u8>::new();
                    let mut archive = async_tar::Builder::new(&mut bytes);
                    let mut header = async_tar::Header::new_gnu();
                    header.set_size(binary_contents.len() as u64);
                    archive
                        .append_data(&mut header, "gleam", binary_contents.as_bytes())
                        .await
                        .unwrap();
                    archive.into_inner().await.unwrap();
                    let mut gzipped_bytes = Vec::new();
                    let mut encoder = GzipEncoder::new(BufReader::new(bytes.as_slice()));
                    encoder.read_to_end(&mut gzipped_bytes).await.unwrap();
                    Ok(Response::new(gzipped_bytes.into()))
                } else {
                    Ok(Response::builder().status(404).body("not found".into())?)
                }
            }
        }
    });
    let user_agent = cx.update(|cx| {
        format!(
            "Sim/{} ({}; {})",
            AppVersion::global(cx),
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    });
    let builder_client =
        Arc::new(ReqwestClient::user_agent(&user_agent).expect("Could not create HTTP client"));

    let extension_store = cx.new(|cx| {
        ExtensionStore::new(
            extensions_dir.clone(),
            Some(cache_dir),
            proxy,
            fs.clone(),
            extension_client.clone(),
            builder_client,
            None,
            node_runtime,
            cx,
        )
    });

    // Ensure that debounces fire.
    let mut events = cx.events(&extension_store);
    let executor = cx.executor();
    let _task = cx.executor().spawn(async move {
        while let Some(event) = events.next().await {
            if let Event::StartedReloading = event {
                executor.advance_clock(RELOAD_DEBOUNCE_DURATION);
            }
        }
    });

    extension_store.update(cx, |_, cx| {
        cx.subscribe(&extension_store, |_, _, event, _| {
            if matches!(event, Event::ExtensionFailedToLoad(_)) {
                panic!("extension failed to load");
            }
        })
        .detach();
    });

    let mut extension_events = cx.events(&cx.update(|cx| {
        extension::ExtensionEvents::try_global(cx)
            .expect("ExtensionEvents should be initialized in tests")
    }));

    let executor = cx.executor();
    await_or_timeout(
        &executor,
        "awaiting install_dev_extension",
        60,
        extension_store.update(cx, |store, cx| {
            store.install_dev_extension(test_extension_dir.clone(), cx)
        }),
    )
    .await
    .unwrap();

    await_or_timeout(
        &executor,
        "awaiting ExtensionsInstalledChanged",
        10,
        async {
            while let Some(event) = extension_events.next().await {
                if matches!(event, extension::Event::ExtensionsInstalledChanged) {
                    return;
                }
            }

            panic!(
                "[test_extension_store_with_test_extension] extension event stream ended before ExtensionsInstalledChanged"
            );
        },
    )
    .await;

    let mut fake_servers = language_registry.register_fake_lsp_server(
        LanguageServerName("gleam".into()),
        lsp::ServerCapabilities {
            completion_provider: Some(Default::default()),
            ..Default::default()
        },
        None,
    );
    cx.executor().run_until_parked();

    let mut project_events = cx.events(&project);
    let buffer_path = project_dir.join("test.gleam");
    let (buffer, _handle) = await_or_timeout(
        &executor,
        "awaiting open_local_buffer_with_lsp",
        5,
        project.update(cx, |project, cx| {
            project.open_local_buffer_with_lsp(buffer_path.clone(), cx)
        }),
    )
    .await
    .unwrap();
    cx.executor().run_until_parked();

    let buffer_remote_id = buffer.read_with(cx, |buffer, _cx| buffer.remote_id());

    let fake_server = await_or_timeout(
        &executor,
        "awaiting first fake server spawn",
        10,
        fake_servers.next(),
    )
    .await
    .unwrap();

    let work_dir = extensions_dir.join(format!("work/{test_extension_id}"));
    let expected_server_path = work_dir.join("gleam-v1.2.3/gleam");
    let expected_binary_contents = language_server_version.lock().binary_contents.clone();

    // check that IO operations in extension work correctly
    assert!(work_dir.join("dir-created-with-rel-path").exists());
    assert!(work_dir.join("dir-created-with-abs-path").exists());
    assert!(work_dir.join("file-created-with-abs-path").exists());
    assert!(work_dir.join("file-created-with-rel-path").exists());

    assert_eq!(fake_server.binary.path, expected_server_path);
    assert_eq!(fake_server.binary.arguments, [OsString::from("lsp")]);
    assert_eq!(
        await_or_timeout(
            &executor,
            "awaiting fs.load(expected_server_path)",
            5,
            fs.load(&expected_server_path)
        )
        .await
        .unwrap(),
        expected_binary_contents
    );
    assert_eq!(language_server_version.lock().http_request_count, 2);
    assert_eq!(
        [
            await_or_timeout(
                &executor,
                "awaiting status_updates #1",
                5,
                status_updates.next()
            )
            .await
            .unwrap(),
            await_or_timeout(
                &executor,
                "awaiting status_updates #2",
                5,
                status_updates.next()
            )
            .await
            .unwrap(),
            await_or_timeout(
                &executor,
                "awaiting status_updates #3",
                5,
                status_updates.next()
            )
            .await
            .unwrap(),
            await_or_timeout(
                &executor,
                "awaiting status_updates #4",
                5,
                status_updates.next()
            )
            .await
            .unwrap(),
        ],
        [
            (
                LanguageServerName::new_static("gleam"),
                BinaryStatus::Starting
            ),
            (
                LanguageServerName::new_static("gleam"),
                BinaryStatus::CheckingForUpdate
            ),
            (
                LanguageServerName::new_static("gleam"),
                BinaryStatus::Downloading
            ),
            (LanguageServerName::new_static("gleam"), BinaryStatus::None)
        ]
    );

    // The extension creates custom labels for completion items.
    fake_server.set_request_handler::<lsp::request::Completion, _, _>(|_, _| async move {
        Ok(Some(lsp::CompletionResponse::Array(vec![
            lsp::CompletionItem {
                label: "foo".into(),
                kind: Some(lsp::CompletionItemKind::FUNCTION),
                detail: Some("fn() -> Result(Nil, Error)".into()),
                ..Default::default()
            },
            lsp::CompletionItem {
                label: "bar.baz".into(),
                kind: Some(lsp::CompletionItemKind::FUNCTION),
                detail: Some("fn(List(a)) -> a".into()),
                ..Default::default()
            },
            lsp::CompletionItem {
                label: "Quux".into(),
                kind: Some(lsp::CompletionItemKind::CONSTRUCTOR),
                detail: Some("fn(String) -> T".into()),
                ..Default::default()
            },
            lsp::CompletionItem {
                label: "my_string".into(),
                kind: Some(lsp::CompletionItemKind::CONSTANT),
                detail: Some("String".into()),
                ..Default::default()
            },
        ])))
    });

    // `register_fake_lsp_server` can yield a server instance before the client has fully registered
    // the buffer with the project LSP plumbing. Wait for the project to observe that registration
    // before issuing requests like completion.
    await_or_timeout(
        &executor,
        "awaiting LanguageServerBufferRegistered",
        5,
        async {
            while let Some(event) = project_events.next().await {
                if let project::Event::LanguageServerBufferRegistered { buffer_id, .. } = event {
                    if buffer_id == buffer_remote_id {
                        return;
                    }
                }
            }

            panic!(
                "[test_extension_store_with_test_extension] project event stream ended before buffer registration for {}",
                buffer_path.display()
            );
        },
    )
    .await;

    let completion_labels = await_or_timeout(
        &executor,
        "awaiting completions",
        5,
        project.update(cx, |project, cx| {
            project.completions(&buffer, 0, DEFAULT_COMPLETION_CONTEXT, cx)
        }),
    )
    .await
    .unwrap()
    .into_iter()
    .flat_map(|response| response.completions)
    .map(|c| c.label.text)
    .collect::<Vec<_>>();
    assert_eq!(
        completion_labels,
        [
            "foo: fn() -> Result(Nil, Error)".to_string(),
            "bar.baz: fn(List(a)) -> a".to_string(),
            "Quux: fn(String) -> T".to_string(),
            "my_string: String".to_string(),
        ]
    );

    // Simulate a new version of the language server being released
    language_server_version.lock().version = "v2.0.0".into();
    language_server_version.lock().binary_contents = "the-new-binary-contents".into();
    language_server_version.lock().http_request_count = 0;

    // Start a new instance of the language server.
    project.update(cx, |project, cx| {
        project.restart_language_servers_for_buffers(
            vec![buffer.clone()],
            HashSet::default(),
            true,
            cx,
        )
    });
    cx.executor().run_until_parked();

    // The extension has cached the binary path, and does not attempt
    // to reinstall it.
    let fake_server = await_or_timeout(
        &executor,
        "awaiting second fake server spawn",
        5,
        fake_servers.next(),
    )
    .await
    .unwrap();
    assert_eq!(fake_server.binary.path, expected_server_path);
    assert_eq!(
        await_or_timeout(
            &executor,
            "awaiting fs.load(expected_server_path) after restart",
            5,
            fs.load(&expected_server_path)
        )
        .await
        .unwrap(),
        expected_binary_contents
    );
    assert_eq!(language_server_version.lock().http_request_count, 0);

    // Reload the extension, clearing its cache.
    // Start a new instance of the language server.
    await_or_timeout(
        &executor,
        "awaiting extension_store.reload(test-extension)",
        5,
        extension_store.update(cx, |store, cx| {
            store.reload(Some("test-extension".into()), cx)
        }),
    )
    .await
    .expect("reload test extension");
    cx.executor().run_until_parked();
    project.update(cx, |project, cx| {
        project.restart_language_servers_for_buffers(
            vec![buffer.clone()],
            HashSet::default(),
            true,
            cx,
        )
    });

    // The extension re-fetches the latest version of the language server.
    let fake_server = await_or_timeout(
        &executor,
        "awaiting third fake server spawn",
        5,
        fake_servers.next(),
    )
    .await
    .unwrap();
    let new_expected_server_path =
        extensions_dir.join(format!("work/{test_extension_id}/gleam-v2.0.0/gleam"));
    let expected_binary_contents = language_server_version.lock().binary_contents.clone();
    assert_eq!(fake_server.binary.path, new_expected_server_path);
    assert_eq!(fake_server.binary.arguments, [OsString::from("lsp")]);
    assert_eq!(
        await_or_timeout(
            &executor,
            "awaiting fs.load(new_expected_server_path)",
            5,
            fs.load(&new_expected_server_path)
        )
        .await
        .unwrap(),
        expected_binary_contents
    );

    // The old language server directory has been cleaned up.
    assert!(
        await_or_timeout(
            &executor,
            "awaiting fs.metadata(expected_server_path)",
            5,
            fs.metadata(&expected_server_path)
        )
        .await
        .unwrap()
        .is_none()
    );
}

fn init_test(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let store = SettingsStore::test(cx);
        cx.set_global(store);
        release_channel::init(semver::Version::new(0, 0, 0), cx);
        extension::init(cx);
        theme_settings::init(theme::LoadThemes::JustBase, cx);
        gpui_tokio::init(cx);
    });
}
