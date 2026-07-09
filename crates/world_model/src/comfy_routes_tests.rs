use crate::{
    ComfyHttpMethod, ComfyRouteCatalog, ComfyRouteHandler, ComfyRouteKind,
    MISSING_ROUTE_ALIAS_CODE, comfy_routes::ComfyRouteDefinition,
};

#[test]
fn route_catalog_resolves_legacy_and_api_aliases_to_same_native_handler() {
    let catalog = ComfyRouteCatalog::default_comfy_routes();

    for (method, legacy_path, api_path, kind) in [
        (
            ComfyHttpMethod::Post,
            "/prompt",
            "/api/prompt",
            ComfyRouteKind::PromptSubmission,
        ),
        (
            ComfyHttpMethod::Get,
            "/queue",
            "/api/queue",
            ComfyRouteKind::Queue,
        ),
        (
            ComfyHttpMethod::Get,
            "/history",
            "/api/history",
            ComfyRouteKind::History,
        ),
        (
            ComfyHttpMethod::Get,
            "/history/{prompt_id}",
            "/api/history/{prompt_id}",
            ComfyRouteKind::HistoryByPromptId,
        ),
        (
            ComfyHttpMethod::Get,
            "/features",
            "/api/features",
            ComfyRouteKind::Features,
        ),
        (
            ComfyHttpMethod::Get,
            "/object_info",
            "/api/object_info",
            ComfyRouteKind::ObjectInfo,
        ),
        (
            ComfyHttpMethod::Get,
            "/object_info/{node_class}",
            "/api/object_info/{node_class}",
            ComfyRouteKind::ObjectInfoByNodeClass,
        ),
        (
            ComfyHttpMethod::Get,
            "/models",
            "/api/models",
            ComfyRouteKind::Models,
        ),
        (
            ComfyHttpMethod::Get,
            "/models/{folder}",
            "/api/models/{folder}",
            ComfyRouteKind::ModelsByFolder,
        ),
        (
            ComfyHttpMethod::Get,
            "/embeddings",
            "/api/embeddings",
            ComfyRouteKind::Embeddings,
        ),
        (
            ComfyHttpMethod::Get,
            "/extensions",
            "/api/extensions",
            ComfyRouteKind::Extensions,
        ),
        (
            ComfyHttpMethod::Get,
            "/jobs",
            "/api/jobs",
            ComfyRouteKind::Jobs,
        ),
        (
            ComfyHttpMethod::Post,
            "/upload/image",
            "/api/upload/image",
            ComfyRouteKind::Upload,
        ),
        (
            ComfyHttpMethod::Get,
            "/view",
            "/api/view",
            ComfyRouteKind::View,
        ),
        (
            ComfyHttpMethod::Get,
            "/settings",
            "/api/settings",
            ComfyRouteKind::AppSettingsRead,
        ),
        (
            ComfyHttpMethod::Post,
            "/settings/{id}",
            "/api/settings/{id}",
            ComfyRouteKind::AppSettingWrite,
        ),
        (
            ComfyHttpMethod::Get,
            "/experiment/models",
            "/api/experiment/models",
            ComfyRouteKind::ExperimentModels,
        ),
        (
            ComfyHttpMethod::Get,
            "/workflow_templates",
            "/api/workflow_templates",
            ComfyRouteKind::WorkflowTemplates,
        ),
        (
            ComfyHttpMethod::Get,
            "/global_subgraphs/{id}",
            "/api/global_subgraphs/{id}",
            ComfyRouteKind::GlobalSubgraphById,
        ),
        (
            ComfyHttpMethod::Delete,
            "/userdata/{file}",
            "/api/userdata/{file}",
            ComfyRouteKind::UserDataDelete,
        ),
        (
            ComfyHttpMethod::Post,
            "/jobs/{job_id}/cancel",
            "/api/jobs/{job_id}/cancel",
            ComfyRouteKind::JobCancelById,
        ),
        (
            ComfyHttpMethod::Post,
            "/upload/mask",
            "/api/upload/mask",
            ComfyRouteKind::UploadMask,
        ),
    ] {
        let legacy = catalog
            .route_for_path(method, legacy_path)
            .unwrap_or_else(|| panic!("missing legacy path {legacy_path}"));
        let api = catalog
            .route_for_path(method, api_path)
            .unwrap_or_else(|| panic!("missing api path {api_path}"));
        assert_eq!(legacy.kind, kind);
        assert_eq!(api.kind, kind);
        assert_eq!(legacy.handler, api.handler);
    }
}

#[test]
fn route_catalog_distinguishes_prompt_get_and_post_handlers() {
    let catalog = ComfyRouteCatalog::default_comfy_routes();

    assert_eq!(
        catalog
            .route_for_path(ComfyHttpMethod::Post, "/prompt")
            .unwrap()
            .kind,
        ComfyRouteKind::PromptSubmission
    );
    assert_eq!(
        catalog
            .route_for_path(ComfyHttpMethod::Get, "/prompt")
            .unwrap()
            .kind,
        ComfyRouteKind::PromptStatus
    );
}

#[test]
fn route_catalog_assigns_routes_to_sim_owned_domains() {
    let catalog = ComfyRouteCatalog::default_comfy_routes();

    assert_eq!(
        catalog
            .route(ComfyRouteKind::PromptSubmission)
            .unwrap()
            .handler,
        ComfyRouteHandler::ControlPlane
    );
    assert_eq!(
        catalog.route(ComfyRouteKind::Queue).unwrap().handler,
        ComfyRouteHandler::JobBridge
    );
    assert_eq!(
        catalog.route(ComfyRouteKind::Models).unwrap().handler,
        ComfyRouteHandler::ModelCatalog
    );
    assert_eq!(
        catalog.route(ComfyRouteKind::ObjectInfo).unwrap().handler,
        ComfyRouteHandler::ObjectInfo
    );
    assert_eq!(
        catalog.route(ComfyRouteKind::Upload).unwrap().handler,
        ComfyRouteHandler::AssetLibrary
    );
    assert_eq!(
        catalog.route(ComfyRouteKind::Extensions).unwrap().handler,
        ComfyRouteHandler::ExtensionRegistry
    );
    assert_eq!(
        catalog
            .route(ComfyRouteKind::WorkflowTemplates)
            .unwrap()
            .handler,
        ComfyRouteHandler::WorkflowRegistry
    );
    assert_eq!(
        catalog.route(ComfyRouteKind::UserDataList).unwrap().handler,
        ComfyRouteHandler::UserDataStore
    );
}

#[test]
fn route_catalog_reports_required_alias_gaps() {
    let catalog = ComfyRouteCatalog::new([ComfyRouteDefinition {
        kind: ComfyRouteKind::Queue,
        method: ComfyHttpMethod::Get,
        legacy_path: "/queue".to_string(),
        api_path: None,
        handler: ComfyRouteHandler::JobBridge,
    }]);

    let diagnostics = catalog
        .validate_required_aliases()
        .expect_err("missing /api alias should be reported");
    assert_eq!(diagnostics[0].code, MISSING_ROUTE_ALIAS_CODE);
    assert_eq!(diagnostics[0].path, "/queue");
}

#[test]
fn default_route_catalog_has_no_alias_gaps() {
    let catalog = ComfyRouteCatalog::default_comfy_routes();

    catalog
        .validate_required_aliases()
        .expect("default catalog should include required aliases");
    assert!(catalog.routes().count() >= 47);
}

#[test]
fn default_route_catalog_covers_runtime_control_plane_backlog_paths() {
    let catalog = ComfyRouteCatalog::default_comfy_routes();

    for (method, path, kind) in [
        (ComfyHttpMethod::Get, "/", ComfyRouteKind::Root),
        (
            ComfyHttpMethod::Get,
            "/system_stats",
            ComfyRouteKind::SystemStats,
        ),
        (ComfyHttpMethod::Get, "/ws", ComfyRouteKind::WebSocket),
        (ComfyHttpMethod::Get, "/i18n", ComfyRouteKind::I18n),
        (
            ComfyHttpMethod::Get,
            "/node_replacements",
            ComfyRouteKind::NodeReplacements,
        ),
        (
            ComfyHttpMethod::Get,
            "/view_metadata/{folder_name}",
            ComfyRouteKind::ViewMetadataByFolder,
        ),
        (
            ComfyHttpMethod::Post,
            "/free",
            ComfyRouteKind::FreeResources,
        ),
        (
            ComfyHttpMethod::Post,
            "/interrupt",
            ComfyRouteKind::Interrupt,
        ),
        (
            ComfyHttpMethod::Get,
            "/v2/userdata",
            ComfyRouteKind::V2UserData,
        ),
        (ComfyHttpMethod::Post, "/users", ComfyRouteKind::UsersWrite),
    ] {
        assert_eq!(
            catalog
                .route_for_path(method, path)
                .unwrap_or_else(|| panic!("missing backlog route {path}"))
                .kind,
            kind
        );
    }
}
