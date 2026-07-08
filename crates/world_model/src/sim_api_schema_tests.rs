use crate::{
    API_SCHEMA_MISSING_REASON_CODE, API_SCHEMA_MISSING_SCHEMA_CODE, ComfyHttpMethod,
    ComfyRouteCatalog, SimApiRouteSupport, SimApiSchemaCatalog, SimApiSchemaRoute,
};

#[test]
fn api_schema_catalog_covers_default_native_routes() {
    let route_catalog = ComfyRouteCatalog::default_comfy_routes();
    let schema_catalog = SimApiSchemaCatalog::from_comfy_route_catalog(&route_catalog);

    schema_catalog
        .validate()
        .expect("default implemented routes should have schema refs");
    assert_eq!(schema_catalog.routes.len(), route_catalog.routes().count());
    let prompt = schema_catalog
        .route(ComfyHttpMethod::Post, "/api/prompt")
        .expect("prompt route should be covered");
    assert!(prompt.support.is_implemented());
    assert_eq!(prompt.native_handler.as_deref(), Some("control_plane"));
    assert_eq!(prompt.schema_ref.as_deref(), Some("#/paths/prompt"));
}

#[test]
fn api_schema_catalog_classifies_documented_non_local_routes() {
    let catalog = SimApiSchemaCatalog::default()
        .with_route(SimApiSchemaRoute::documented(
            ComfyHttpMethod::Post,
            "/api/provider/run",
            SimApiRouteSupport::CloudOnly {
                reason: "provider execution is delegated to approved remote providers".to_string(),
            },
        ))
        .with_route(SimApiSchemaRoute::documented(
            ComfyHttpMethod::Post,
            "/api/manager/install",
            SimApiRouteSupport::Planned {
                reason: "manager compatibility requires the extension policy gate".to_string(),
            },
        ))
        .with_route(SimApiSchemaRoute::documented(
            ComfyHttpMethod::Get,
            "/api/external/news",
            SimApiRouteSupport::External {
                reason: "upstream marketplace data remains external".to_string(),
            },
        ))
        .with_route(SimApiSchemaRoute::documented(
            ComfyHttpMethod::Post,
            "/api/raw-python",
            SimApiRouteSupport::Unsupported {
                reason: "arbitrary Python execution is outside the Sim boundary".to_string(),
            },
        ));

    catalog
        .validate()
        .expect("classified routes should be valid");
    assert!(matches!(
        catalog
            .route(ComfyHttpMethod::Post, "/api/provider/run")
            .unwrap()
            .support,
        SimApiRouteSupport::CloudOnly { .. }
    ));
    assert!(matches!(
        catalog
            .route(ComfyHttpMethod::Post, "/api/manager/install")
            .unwrap()
            .support,
        SimApiRouteSupport::Planned { .. }
    ));
    assert!(matches!(
        catalog
            .route(ComfyHttpMethod::Get, "/api/external/news")
            .unwrap()
            .support,
        SimApiRouteSupport::External { .. }
    ));
    assert!(matches!(
        catalog
            .route(ComfyHttpMethod::Post, "/api/raw-python")
            .unwrap()
            .support,
        SimApiRouteSupport::Unsupported { .. }
    ));
}

#[test]
fn api_schema_catalog_rejects_implemented_routes_without_schema_refs() {
    let catalog = SimApiSchemaCatalog::new([SimApiSchemaRoute {
        method: ComfyHttpMethod::Get,
        path: "/api/missing-schema".to_string(),
        support: SimApiRouteSupport::Implemented,
        native_handler: Some("test".to_string()),
        schema_ref: None,
        notes: None,
    }]);

    let diagnostics = catalog.validate().unwrap_err();
    assert_eq!(diagnostics[0].code, API_SCHEMA_MISSING_SCHEMA_CODE);
    assert_eq!(diagnostics[0].path, "/api/missing-schema");
}

#[test]
fn api_schema_catalog_rejects_non_local_routes_without_reasons() {
    let catalog = SimApiSchemaCatalog::new([SimApiSchemaRoute::documented(
        ComfyHttpMethod::Get,
        "/api/planned",
        SimApiRouteSupport::Planned {
            reason: " ".to_string(),
        },
    )]);

    let diagnostics = catalog.validate().unwrap_err();
    assert_eq!(diagnostics[0].code, API_SCHEMA_MISSING_REASON_CODE);
    assert_eq!(diagnostics[0].path, "/api/planned");
}

#[test]
fn api_routes_fixture_matches_default_catalog() {
    let fixture = include_str!("../fixtures/comfy/api_routes.json");
    let fixture_catalog: SimApiSchemaCatalog =
        serde_json::from_str(fixture).expect("fixture should parse");
    let default_catalog =
        SimApiSchemaCatalog::from_comfy_route_catalog(&ComfyRouteCatalog::default_comfy_routes());

    assert_eq!(fixture_catalog, default_catalog);
}
