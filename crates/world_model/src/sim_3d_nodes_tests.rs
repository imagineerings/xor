use crate::{
    MeshArtifactMetadata, MeshBackend, MeshFormat, SIM_THREE_D_DEPENDENCY_REVIEW_REQUIRED_CODE,
    SIM_THREE_D_INVALID_GEOMETRY_CODE, SIM_THREE_D_UNSUPPORTED_FORMAT_CODE, SimThreeDArtifactKind,
    SimThreeDBackendStatus, SimThreeDMetadata, SimThreeDNodeAdapter, SimThreeDOperation,
};

#[test]
fn three_d_adapter_loads_previews_and_saves_mesh_metadata() {
    let adapter = SimThreeDNodeAdapter::new();
    let mesh_metadata = MeshArtifactMetadata::new("outputs/chair.glb", MeshFormat::Glb)
        .with_preview("previews/chair.png")
        .with_provenance("prov-chair")
        .with_source_asset("inputs/chair.png")
        .with_vertex_count(1_000)
        .with_triangle_count(2_000);

    let loaded = adapter.load_mesh(mesh_metadata);
    assert_eq!(loaded.reference, "outputs/chair.glb");
    assert_eq!(loaded.kind, SimThreeDArtifactKind::Mesh);
    assert_eq!(
        loaded.metadata.preview_reference.as_deref(),
        Some("previews/chair.png")
    );
    assert_eq!(loaded.metadata.provenance_id.as_deref(), Some("prov-chair"));
    assert_eq!(loaded.metadata.vertex_count, Some(1_000));
    assert_eq!(loaded.metadata.triangle_count, Some(2_000));

    let previewed = adapter.preview(&loaded, "previews/chair-turntable.png");
    assert_eq!(
        previewed.metadata.preview_reference.as_deref(),
        Some("previews/chair-turntable.png")
    );

    let saved = adapter
        .save_mesh(&loaded, "exports/chair.obj", MeshFormat::Obj)
        .expect("save mesh");
    assert_eq!(saved.reference, "exports/chair.obj");
    assert_eq!(saved.metadata.format, "obj");
}

#[test]
fn three_d_adapter_registers_geometry_point_clouds_and_gaussian_splats() {
    let adapter = SimThreeDNodeAdapter::new();

    let depth = adapter
        .register_geometry(
            "geometry://depth",
            SimThreeDArtifactKind::DepthMap,
            SimThreeDMetadata::new("depth")
                .with_preview_reference("previews/depth.png")
                .with_provenance("prov-depth"),
        )
        .expect("depth registration");
    assert_eq!(depth.kind, SimThreeDArtifactKind::DepthMap);
    assert_eq!(
        depth.metadata.preview_reference.as_deref(),
        Some("previews/depth.png")
    );

    let point_cloud = adapter
        .register_geometry(
            "geometry://cloud.ply",
            SimThreeDArtifactKind::PointCloud,
            SimThreeDMetadata::new("ply").with_point_count(42_000),
        )
        .expect("point cloud");
    assert_eq!(point_cloud.metadata.point_count, Some(42_000));

    let splat = adapter
        .register_geometry(
            "geometry://splat.splat",
            SimThreeDArtifactKind::GaussianSplat,
            SimThreeDMetadata::new("splat").with_point_count(9_000),
        )
        .expect("splat");
    let preview = adapter
        .gaussian_splat_preview(&splat, "previews/splat.png")
        .expect("splat preview");
    assert_eq!(
        preview.metadata.preview_reference.as_deref(),
        Some("previews/splat.png")
    );
}

#[test]
fn three_d_adapter_rejects_invalid_geometry() {
    let adapter = SimThreeDNodeAdapter::new();

    let mesh_diagnostic = adapter
        .register_geometry(
            "geometry://mesh.obj",
            SimThreeDArtifactKind::Mesh,
            SimThreeDMetadata::new("obj").with_vertex_count(3),
        )
        .expect_err("missing triangle count");
    assert_eq!(mesh_diagnostic.code, SIM_THREE_D_INVALID_GEOMETRY_CODE);

    let cloud_diagnostic = adapter
        .register_geometry(
            "geometry://cloud.ply",
            SimThreeDArtifactKind::PointCloud,
            SimThreeDMetadata::new("ply"),
        )
        .expect_err("missing point count");
    assert_eq!(cloud_diagnostic.code, SIM_THREE_D_INVALID_GEOMETRY_CODE);

    let mesh = adapter.load_mesh(MeshArtifactMetadata::new(
        "outputs/chair.obj",
        MeshFormat::Obj,
    ));
    let splat_diagnostic = adapter
        .gaussian_splat_preview(&mesh, "previews/not-splat.png")
        .expect_err("not a splat");
    assert_eq!(splat_diagnostic.code, SIM_THREE_D_INVALID_GEOMETRY_CODE);
}

#[test]
fn three_d_adapter_delegates_textured_mesh_lifecycle() {
    let adapter = SimThreeDNodeAdapter::new();
    let mesh_metadata = MeshArtifactMetadata::new("outputs/statue.glb", MeshFormat::Glb)
        .with_preview("previews/statue.png")
        .with_textures(2048)
        .with_source_asset("inputs/statue.png");

    let delegation = adapter
        .delegate_textured_mesh_export(mesh_metadata)
        .expect("delegation");
    assert_eq!(delegation.artifact.kind, SimThreeDArtifactKind::Mesh);
    assert_eq!(delegation.mesh_metadata.texture_resolution, Some(2048));
    assert_eq!(
        delegation
            .artifact
            .metadata
            .fields
            .get("sim.has_textures")
            .map(String::as_str),
        Some("true")
    );

    let untextured = MeshArtifactMetadata::new("outputs/plain.obj", MeshFormat::Obj);
    let diagnostic = adapter
        .delegate_textured_mesh_export(untextured)
        .expect_err("untextured mesh");
    assert_eq!(diagnostic.code, SIM_THREE_D_INVALID_GEOMETRY_CODE);
}

#[test]
fn three_d_adapter_reports_backend_and_format_diagnostics() {
    let adapter = SimThreeDNodeAdapter::new();

    let format_diagnostic = adapter
        .format_diagnostic(SimThreeDOperation::Save, MeshFormat::Fbx)
        .expect("format diagnostic");
    assert_eq!(
        format_diagnostic.code,
        SIM_THREE_D_DEPENDENCY_REVIEW_REQUIRED_CODE
    );

    let backend_diagnostic = adapter
        .backend_dependency_diagnostic(SimThreeDOperation::TexturedMeshExport, MeshBackend::Native)
        .expect("backend diagnostic");
    assert_eq!(
        backend_diagnostic.code,
        SIM_THREE_D_DEPENDENCY_REVIEW_REQUIRED_CODE
    );

    let unsupported = adapter
        .backend_diagnostic(
            SimThreeDOperation::Convert,
            SimThreeDBackendStatus::Unsupported,
            "custom voxel format",
        )
        .expect("unsupported diagnostic");
    assert_eq!(unsupported.code, SIM_THREE_D_UNSUPPORTED_FORMAT_CODE);

    assert!(
        adapter
            .backend_diagnostic(
                SimThreeDOperation::Render,
                SimThreeDBackendStatus::Native,
                "turntable render"
            )
            .is_none()
    );
}
