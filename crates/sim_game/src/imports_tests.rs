use crate::SimGameImportMetadataLinker;

#[test]
fn import_linker_links_source_and_generated_files() {
    let source = r#"
[remap]
importer="texture"
type="CompressedTexture2D"
uid="uid://texture"
path="res://.godot/imported/hero.png.ctex"

[deps]
source_file="res://textures/hero.png"
dest_files=["res://.godot/imported/hero.png.ctex","res://.godot/imported/hero.png.s3tc.ctex"]
"#;

    let link = SimGameImportMetadataLinker::new().link("textures/hero.png.import", source);

    assert_eq!(link.source_file.as_deref(), Some("res://textures/hero.png"));
    assert_eq!(link.generated_files.len(), 2);
    assert!(link.diagnostics.is_empty());
}

#[test]
fn import_linker_reports_missing_import_metadata() {
    let link = SimGameImportMetadataLinker::new().link("textures/hero.png.import", "[deps]");

    assert_eq!(link.diagnostics.len(), 2);
}
