use std::path::Path;

use crate::{FixtureAttribution, FixtureLicense, FixtureManifest, FixtureSource};

// ---------------------------------------------------------------------------
// FixtureSource
// ---------------------------------------------------------------------------

#[test]
fn godot_project_source_description() {
    let source = FixtureSource::GodotProject {
        root: Path::new("/projects/my-game").to_path_buf(),
        relative_path: Path::new("assets/icon.png").to_path_buf(),
    };
    let desc = source.description();
    assert!(desc.contains("Godot project"));
    assert!(desc.contains("assets/icon.png"));
}

#[test]
fn url_source_description() {
    let source = FixtureSource::Url {
        url: "https://example.com/texture.png".into(),
    };
    assert_eq!(source.description(), "URL: https://example.com/texture.png");
}

#[test]
fn original_source_description() {
    let source = FixtureSource::Original;
    assert!(source.description().contains("Original"));
}

#[test]
fn comfy_export_with_node() {
    let source = FixtureSource::ComfyExport {
        workflow_name: "upscale".into(),
        node_id: Some("KSampler_12".into()),
    };
    let desc = source.description();
    assert!(desc.contains("upscale"));
    assert!(desc.contains("KSampler_12"));
}

#[test]
fn comfy_export_without_node() {
    let source = FixtureSource::ComfyExport {
        workflow_name: "texture-gen".into(),
        node_id: None,
    };
    let desc = source.description();
    assert!(desc.contains("texture-gen"));
    assert!(!desc.contains("node"));
}

// ---------------------------------------------------------------------------
// FixtureLicense
// ---------------------------------------------------------------------------

#[test]
fn spdx_license_label() {
    let license = FixtureLicense::Spdx("CC0-1.0".into());
    assert_eq!(license.label(), "CC0-1.0");
}

#[test]
fn custom_license_label() {
    let license = FixtureLicense::Custom("Used with permission".into());
    assert_eq!(license.label(), "Custom: Used with permission");
}

#[test]
fn unlicensed_with_author() {
    let license = FixtureLicense::Unlicensed {
        author: Some("John Doe".into()),
    };
    assert_eq!(license.label(), "Unlicensed © John Doe");
}

#[test]
fn unlicensed_without_author() {
    let license = FixtureLicense::Unlicensed { author: None };
    assert_eq!(license.label(), "Unlicensed");
}

// ---------------------------------------------------------------------------
// FixtureAttribution
// ---------------------------------------------------------------------------

#[test]
fn attribution_creates_with_required_fields() {
    let attr = FixtureAttribution::new(
        "fixtures/icon.png",
        FixtureSource::Original,
        FixtureLicense::Spdx("MIT".into()),
    );
    assert_eq!(attr.fixture_path, Path::new("fixtures/icon.png"));
    assert_eq!(attr.source, FixtureSource::Original);
    assert_eq!(attr.license, FixtureLicense::Spdx("MIT".into()));
    assert!(attr.author.is_none());
    assert!(attr.notes.is_none());
}

#[test]
fn attribution_with_author_and_notes() {
    let attr = FixtureAttribution::new(
        "textures/floor.png",
        FixtureSource::Url {
            url: "https://example.com/floor.png".into(),
        },
        FixtureLicense::Custom("CC BY 4.0".into()),
    )
    .with_author("TextureAuthor")
    .with_notes("Resized to 512x512");
    assert_eq!(attr.author.as_deref(), Some("TextureAuthor"));
    assert_eq!(attr.notes.as_deref(), Some("Resized to 512x512"));
}

// ---------------------------------------------------------------------------
// FixtureManifest
// ---------------------------------------------------------------------------

#[test]
fn empty_manifest_is_empty() {
    let manifest = FixtureManifest::new();
    assert!(manifest.is_empty());
    assert_eq!(manifest.len(), 0);
}

#[test]
fn manifest_with_one_fixture() {
    let mut manifest = FixtureManifest::new();
    manifest.push(FixtureAttribution::new(
        "icon.png",
        FixtureSource::Original,
        FixtureLicense::Spdx("MIT".into()),
    ));
    assert!(!manifest.is_empty());
    assert_eq!(manifest.len(), 1);
}

#[test]
fn manifest_extend_merges() {
    let mut a = FixtureManifest::new();
    a.push(FixtureAttribution::new(
        "a.png",
        FixtureSource::Original,
        FixtureLicense::Spdx("MIT".into()),
    ));
    let mut b = FixtureManifest::new();
    b.push(FixtureAttribution::new(
        "b.png",
        FixtureSource::Original,
        FixtureLicense::Spdx("CC0".into()),
    ));
    a.extend(b);
    assert_eq!(a.len(), 2);
}

#[test]
fn manifest_find_by_path_matches() {
    let mut manifest = FixtureManifest::new();
    manifest.push(FixtureAttribution::new(
        "icons/play.png",
        FixtureSource::Original,
        FixtureLicense::Spdx("MIT".into()),
    ));
    manifest.push(FixtureAttribution::new(
        "icons/stop.png",
        FixtureSource::Original,
        FixtureLicense::Spdx("MIT".into()),
    ));
    let found = manifest.find_by_path(Path::new("icons/play.png"));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fixture_path, Path::new("icons/play.png"));
}

#[test]
fn manifest_find_by_path_no_match() {
    let manifest = FixtureManifest::new();
    let found = manifest.find_by_path(Path::new("missing.png"));
    assert!(found.is_empty());
}

#[test]
fn manifest_validate_accepts_good_fixtures() {
    let mut manifest = FixtureManifest::new();
    manifest.push(FixtureAttribution::new(
        "good.png",
        FixtureSource::Original,
        FixtureLicense::Spdx("MIT".into()),
    ));
    let errors = manifest.validate();
    assert!(errors.is_empty());
}

#[test]
fn manifest_from_iterator() {
    let attrs = vec![
        FixtureAttribution::new(
            "a.png",
            FixtureSource::Original,
            FixtureLicense::Spdx("MIT".into()),
        ),
        FixtureAttribution::new(
            "b.png",
            FixtureSource::Original,
            FixtureLicense::Spdx("CC0".into()),
        ),
    ];
    let manifest: FixtureManifest = attrs.into_iter().collect();
    assert_eq!(manifest.len(), 2);
}

#[test]
fn manifest_into_iterator() {
    let mut manifest = FixtureManifest::new();
    manifest.push(FixtureAttribution::new(
        "x.png",
        FixtureSource::Original,
        FixtureLicense::Spdx("MIT".into()),
    ));
    let count = manifest.into_iter().count();
    assert_eq!(count, 1);
}
