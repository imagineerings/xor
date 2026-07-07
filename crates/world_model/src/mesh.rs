use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Mesh backend
// ---------------------------------------------------------------------------

/// The mesh generation backend to use.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MeshBackend {
    /// External Python/PyTorch-based mesh generation (e.g., TripoSR, CRM).
    Python,
    /// Remote API-based mesh generation (e.g., Meshy, Rodin).
    RemoteApi,
    /// Native Rust mesh generation (requires dependency review per Req 7.3).
    Native,
    /// Backend auto-selected by the system.
    #[default]
    Automatic,
}

impl MeshBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::RemoteApi => "remote_api",
            Self::Native => "native",
            Self::Automatic => "automatic",
        }
    }

    /// Whether this backend is likely to require a native or heavy dependency
    /// review before implementation (Req 7.3).
    pub fn requires_dependency_review(self) -> bool {
        matches!(self, Self::Native)
    }
}

// ---------------------------------------------------------------------------
// Mesh format
// ---------------------------------------------------------------------------

/// Target export format for the generated mesh.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MeshFormat {
    /// Wavefront OBJ.
    #[default]
    Obj,
    /// glTF 2.0 Binary.
    Glb,
    /// glTF 2.0 (separate files).
    Gltf,
    /// FBX (requires dependency review; non-preferred).
    Fbx,
    /// Stanford PLY.
    Ply,
    /// Universal Scene Description.
    Usd,
    /// Alembic.
    Abc,
    /// STereoLithography (binary).
    Stl,
}

impl MeshFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Obj => "obj",
            Self::Glb => "glb",
            Self::Gltf => "gltf",
            Self::Fbx => "fbx",
            Self::Ply => "ply",
            Self::Usd => "usd",
            Self::Abc => "abc",
            Self::Stl => "stl",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Obj => "Wavefront OBJ",
            Self::Glb => "glTF Binary",
            Self::Gltf => "glTF",
            Self::Fbx => "FBX",
            Self::Ply => "Stanford PLY",
            Self::Usd => "USD",
            Self::Abc => "Alembic",
            Self::Stl => "STL",
        }
    }

    /// Whether this format typically requires a native or heavy dependency
    /// to produce (Req 7.3).
    pub fn requires_dependency_review(self) -> bool {
        matches!(self, Self::Fbx | Self::Usd | Self::Abc)
    }
}

// ---------------------------------------------------------------------------
// Texture options
// ---------------------------------------------------------------------------

/// Texture generation options for the mesh.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TextureOptions {
    /// Whether to generate textures.
    pub enabled: bool,
    /// Target texture resolution (e.g., 1024, 2048).
    pub resolution: Option<u32>,
    /// Whether to bake ambient occlusion.
    pub bake_ao: bool,
    /// Whether to generate a normal map.
    pub normal_map: bool,
    /// Whether to generate a roughness/metallic map.
    pub pbr_maps: bool,
    /// Additional texture generation parameters.
    pub extra: HashMap<String, String>,
}

/// Backend-specific model or configuration selector.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BackendOptions {
    /// Backend-specific model name or preset.
    pub model: Option<String>,
    /// Backend-specific configuration parameters.
    pub params: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Mesh generation request
// ---------------------------------------------------------------------------

/// A request to generate a textured 3D mesh (Req 7.1).
///
/// Captures the prompt, optional reference image, target format, texture
/// options, backend selection, and backend-specific options.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MeshGenerationRequest {
    /// Text description of the desired mesh.
    pub prompt: String,
    /// Optional reference image path for image-conditioned generation.
    pub reference_image: Option<String>,
    /// Target export format.
    pub target_format: MeshFormat,
    /// Texture generation options.
    pub textures: TextureOptions,
    /// Mesh generation backend.
    pub backend: MeshBackend,
    /// Backend-specific options (model selection, parameters).
    pub backend_options: BackendOptions,
    /// Arbitrary metadata for extensibility.
    pub metadata: HashMap<String, String>,
}

impl MeshGenerationRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    pub fn with_reference_image(mut self, path: impl Into<String>) -> Self {
        self.reference_image = Some(path.into());
        self
    }

    pub fn with_target_format(mut self, format: MeshFormat) -> Self {
        self.target_format = format;
        self
    }

    pub fn with_textures(mut self, textures: TextureOptions) -> Self {
        self.textures = textures;
        self
    }

    pub fn with_backend(mut self, backend: MeshBackend) -> Self {
        self.backend = backend;
        self
    }

    pub fn with_backend_options(mut self, opts: BackendOptions) -> Self {
        self.backend_options = opts;
        self
    }
}

// ---------------------------------------------------------------------------
// Mesh artifact metadata
// ---------------------------------------------------------------------------

/// A generated mesh artifact with preview, export, provenance, and source
/// asset metadata (Req 7.2).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MeshArtifactMetadata {
    /// Path to the generated mesh file.
    pub mesh_path: String,
    /// Target format of the mesh file.
    pub format: MeshFormat,
    /// Preview image path (e.g., rendered thumbnail).
    pub preview_path: Option<String>,
    /// Exported path if a different format was chosen at export time.
    pub export_path: Option<String>,
    /// Exported format (None if not re-exported).
    pub export_format: Option<MeshFormat>,
    /// Reference to generation provenance (usually the GenerationProvenance
    /// from the provenance module).
    pub provenance_id: Option<String>,
    /// Original source asset paths used in generation.
    pub source_assets: Vec<String>,
    /// Triangle count if available.
    pub triangle_count: Option<u64>,
    /// Vertex count if available.
    pub vertex_count: Option<u64>,
    /// Whether textures were generated and applied.
    pub has_textures: bool,
    /// Texture resolution (if textures were generated).
    pub texture_resolution: Option<u32>,
}

impl MeshArtifactMetadata {
    pub fn new(mesh_path: impl Into<String>, format: MeshFormat) -> Self {
        Self {
            mesh_path: mesh_path.into(),
            format,
            ..Default::default()
        }
    }

    pub fn with_preview(mut self, path: impl Into<String>) -> Self {
        self.preview_path = Some(path.into());
        self
    }

    pub fn with_export(mut self, path: impl Into<String>, format: MeshFormat) -> Self {
        self.export_path = Some(path.into());
        self.export_format = Some(format);
        self
    }

    pub fn with_provenance(mut self, id: impl Into<String>) -> Self {
        self.provenance_id = Some(id.into());
        self
    }

    pub fn with_source_asset(mut self, asset: impl Into<String>) -> Self {
        self.source_assets.push(asset.into());
        self
    }

    pub fn with_triangle_count(mut self, count: u64) -> Self {
        self.triangle_count = Some(count);
        self
    }

    pub fn with_vertex_count(mut self, count: u64) -> Self {
        self.vertex_count = Some(count);
        self
    }

    pub fn with_textures(mut self, resolution: u32) -> Self {
        self.has_textures = true;
        self.texture_resolution = Some(resolution);
        self
    }
}
