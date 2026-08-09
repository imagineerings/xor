use comfy_tensor::{DType, StorageId, Tensor, TensorError};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, mem};
use thiserror::Error;

const MAX_FRAMES: usize = 65_536;
const MAX_ITEMS: usize = 65_536;
const MAX_TEXT_BYTES: usize = 4_096;
const FACE_LANDMARK_COUNT: usize = 478;
const FACE_BLENDSHAPE_COUNT: usize = 52;

pub const MEDIAPIPE_FACE_BLENDSHAPE_NAMES: [&str; FACE_BLENDSHAPE_COUNT] = [
    "_neutral",
    "browDownLeft",
    "browDownRight",
    "browInnerUp",
    "browOuterUpLeft",
    "browOuterUpRight",
    "cheekPuff",
    "cheekSquintLeft",
    "cheekSquintRight",
    "eyeBlinkLeft",
    "eyeBlinkRight",
    "eyeLookDownLeft",
    "eyeLookDownRight",
    "eyeLookInLeft",
    "eyeLookInRight",
    "eyeLookOutLeft",
    "eyeLookOutRight",
    "eyeLookUpLeft",
    "eyeLookUpRight",
    "eyeSquintLeft",
    "eyeSquintRight",
    "eyeWideLeft",
    "eyeWideRight",
    "jawForward",
    "jawLeft",
    "jawOpen",
    "jawRight",
    "mouthClose",
    "mouthDimpleLeft",
    "mouthDimpleRight",
    "mouthFrownLeft",
    "mouthFrownRight",
    "mouthFunnel",
    "mouthLeft",
    "mouthLowerDownLeft",
    "mouthLowerDownRight",
    "mouthPressLeft",
    "mouthPressRight",
    "mouthPucker",
    "mouthRight",
    "mouthRollLower",
    "mouthRollUpper",
    "mouthShrugLower",
    "mouthShrugUpper",
    "mouthSmileLeft",
    "mouthSmileRight",
    "mouthStretchLeft",
    "mouthStretchRight",
    "mouthUpperUpLeft",
    "mouthUpperUpRight",
    "noseSneerLeft",
    "noseSneerRight",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NativePoint2 {
    x: f64,
    y: f64,
}

impl NativePoint2 {
    pub fn checked(x: f64, y: f64) -> Result<Self, NativeMediaPayloadError> {
        require_finite("point x", x)?;
        require_finite("point y", y)?;
        Ok(Self { x, y })
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NativePoint3 {
    x: f64,
    y: f64,
    z: f64,
}

impl NativePoint3 {
    pub fn checked(x: f64, y: f64, z: f64) -> Result<Self, NativeMediaPayloadError> {
        require_finite("point x", x)?;
        require_finite("point y", y)?;
        require_finite("point z", z)?;
        Ok(Self { x, y, z })
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }

    pub const fn z(self) -> f64 {
        self.z
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeBoundingBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    label: Option<Box<str>>,
    score: Option<f64>,
}

impl NativeBoundingBox {
    pub fn checked(
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        label: Option<String>,
        score: Option<f64>,
    ) -> Result<Self, NativeMediaPayloadError> {
        require_finite("bounding box x", x)?;
        require_finite("bounding box y", y)?;
        require_non_negative("bounding box width", width)?;
        require_non_negative("bounding box height", height)?;
        let label = label
            .map(|label| checked_text("bounding box label", label))
            .transpose()?
            .map(String::into_boxed_str);
        if let Some(score) = score {
            require_probability("bounding box score", score)?;
        }
        Ok(Self {
            x,
            y,
            width,
            height,
            label,
            score,
        })
    }

    pub const fn x(&self) -> f64 {
        self.x
    }

    pub const fn y(&self) -> f64 {
        self.y
    }

    pub const fn width(&self) -> f64 {
        self.width
    }

    pub const fn height(&self) -> f64 {
        self.height
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub const fn score(&self) -> Option<f64> {
        self.score
    }

    fn validate(&self) -> Result<(), NativeMediaPayloadError> {
        Self::checked(
            self.x,
            self.y,
            self.width,
            self.height,
            self.label.as_deref().map(str::to_owned),
            self.score,
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct NativeBoundingBoxPayload {
    frames: Box<[Box<[NativeBoundingBox]>]>,
    semantic_digest_sha256: [u8; 32],
    resident_bytes: u64,
}

impl NativeBoundingBoxPayload {
    pub const SOURCE_TYPE_ID: &'static str = "BOUNDING_BOX";

    pub fn checked(frames: Vec<Vec<NativeBoundingBox>>) -> Result<Self, NativeMediaPayloadError> {
        check_count("bounding box frames", frames.len(), MAX_FRAMES)?;
        let frames = frames
            .into_iter()
            .map(|frame| {
                check_count("bounding boxes per frame", frame.len(), MAX_ITEMS)?;
                for bounding_box in &frame {
                    bounding_box.validate()?;
                }
                Ok(frame.into_boxed_slice())
            })
            .collect::<Result<Vec<_>, NativeMediaPayloadError>>()?
            .into_boxed_slice();
        let (semantic_digest_sha256, resident_bytes) = project_bounding_boxes(&frames)?;
        Ok(Self {
            frames,
            semantic_digest_sha256,
            resident_bytes,
        })
    }

    pub fn frames(&self) -> &[Box<[NativeBoundingBox]>] {
        &self.frames
    }

    pub const fn semantic_digest_sha256(&self) -> &[u8; 32] {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn validate(&self) -> Result<(), NativeMediaPayloadError> {
        let (digest, resident_bytes) = project_bounding_boxes(&self.frames)?;
        require_projection(
            self.semantic_digest_sha256,
            digest,
            self.resident_bytes,
            resident_bytes,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeFaceBlendshape {
    name: Box<str>,
    score: f64,
}

impl NativeFaceBlendshape {
    pub fn checked(name: String, score: f64) -> Result<Self, NativeMediaPayloadError> {
        let name = checked_text("face blendshape name", name)?.into_boxed_str();
        require_probability("face blendshape score", score)?;
        Ok(Self { name, score })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn score(&self) -> f64 {
        self.score
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeFaceLandmark {
    bbox_xyxy: [f64; 4],
    blendshapes: Box<[NativeFaceBlendshape]>,
    landmarks_xy: Box<[NativePoint2]>,
    landmarks_3d: Box<[NativePoint3]>,
    presence: f64,
    score: f64,
    transformation_matrix: [f64; 16],
}

impl NativeFaceLandmark {
    #[allow(clippy::too_many_arguments)]
    pub fn checked(
        bbox_xyxy: [f64; 4],
        blendshapes: Vec<NativeFaceBlendshape>,
        landmarks_xy: Vec<NativePoint2>,
        landmarks_3d: Vec<NativePoint3>,
        presence: f64,
        score: f64,
        transformation_matrix: [f64; 16],
    ) -> Result<Self, NativeMediaPayloadError> {
        for value in bbox_xyxy {
            require_finite("face bounding box", value)?;
        }
        if bbox_xyxy[2] < bbox_xyxy[0] || bbox_xyxy[3] < bbox_xyxy[1] {
            return Err(NativeMediaPayloadError::InvalidExtent("face bounding box"));
        }
        check_exact_count("face blendshapes", blendshapes.len(), FACE_BLENDSHAPE_COUNT)?;
        for (index, blendshape) in blendshapes.iter().enumerate() {
            let expected = MEDIAPIPE_FACE_BLENDSHAPE_NAMES[index];
            if blendshape.name() != expected {
                return Err(NativeMediaPayloadError::UnexpectedBlendshape {
                    index,
                    expected,
                    actual: blendshape.name().to_owned(),
                });
            }
            require_probability("face blendshape score", blendshape.score())?;
        }
        check_exact_count("2D face landmarks", landmarks_xy.len(), FACE_LANDMARK_COUNT)?;
        check_exact_count("3D face landmarks", landmarks_3d.len(), FACE_LANDMARK_COUNT)?;
        require_finite("face presence", presence)?;
        require_probability("face score", score)?;
        for value in transformation_matrix {
            require_finite("face transformation matrix", value)?;
        }
        Ok(Self {
            bbox_xyxy,
            blendshapes: blendshapes.into_boxed_slice(),
            landmarks_xy: landmarks_xy.into_boxed_slice(),
            landmarks_3d: landmarks_3d.into_boxed_slice(),
            presence,
            score,
            transformation_matrix,
        })
    }

    pub const fn bbox_xyxy(&self) -> &[f64; 4] {
        &self.bbox_xyxy
    }

    pub fn blendshapes(&self) -> &[NativeFaceBlendshape] {
        &self.blendshapes
    }

    pub fn landmarks_xy(&self) -> &[NativePoint2] {
        &self.landmarks_xy
    }

    pub fn landmarks_3d(&self) -> &[NativePoint3] {
        &self.landmarks_3d
    }

    pub const fn presence(&self) -> f64 {
        self.presence
    }

    pub const fn score(&self) -> f64 {
        self.score
    }

    pub const fn transformation_matrix(&self) -> &[f64; 16] {
        &self.transformation_matrix
    }

    fn validate(&self) -> Result<(), NativeMediaPayloadError> {
        Self::checked(
            self.bbox_xyxy,
            self.blendshapes.to_vec(),
            self.landmarks_xy.to_vec(),
            self.landmarks_3d.to_vec(),
            self.presence,
            self.score,
            self.transformation_matrix,
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeFaceConnectionSet {
    name: Box<str>,
    edges: Box<[[u16; 2]]>,
}

impl NativeFaceConnectionSet {
    pub fn checked(
        name: String,
        mut edges: Vec<[u16; 2]>,
    ) -> Result<Self, NativeMediaPayloadError> {
        let name = checked_text("face connection set name", name)?.into_boxed_str();
        check_count("face connection edges", edges.len(), MAX_ITEMS)?;
        for edge in &edges {
            if usize::from(edge[0]) >= FACE_LANDMARK_COUNT
                || usize::from(edge[1]) >= FACE_LANDMARK_COUNT
                || edge[0] == edge[1]
            {
                return Err(NativeMediaPayloadError::InvalidFaceConnection(*edge));
            }
        }
        edges.sort_unstable();
        if edges.windows(2).any(|window| window[0] == window[1]) {
            return Err(NativeMediaPayloadError::DuplicateFaceConnection);
        }
        Ok(Self {
            name,
            edges: edges.into_boxed_slice(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn edges(&self) -> &[[u16; 2]] {
        &self.edges
    }
}

#[derive(Clone, Debug)]
pub struct NativeFaceLandmarksPayload {
    image_height: u32,
    image_width: u32,
    frames: Box<[Box<[NativeFaceLandmark]>]>,
    connection_sets: Box<[NativeFaceConnectionSet]>,
    semantic_digest_sha256: [u8; 32],
    resident_bytes: u64,
}

impl NativeFaceLandmarksPayload {
    pub const SOURCE_TYPE_ID: &'static str = "FACE_LANDMARKS";

    pub fn checked(
        image_height: u32,
        image_width: u32,
        frames: Vec<Vec<NativeFaceLandmark>>,
        mut connection_sets: Vec<NativeFaceConnectionSet>,
    ) -> Result<Self, NativeMediaPayloadError> {
        require_image_size(image_height, image_width)?;
        check_count("face landmark frames", frames.len(), MAX_FRAMES)?;
        let frames = frames
            .into_iter()
            .map(|frame| {
                check_count("faces per frame", frame.len(), MAX_ITEMS)?;
                for face in &frame {
                    face.validate()?;
                }
                Ok(frame.into_boxed_slice())
            })
            .collect::<Result<Vec<_>, NativeMediaPayloadError>>()?
            .into_boxed_slice();
        check_count("face connection sets", connection_sets.len(), MAX_ITEMS)?;
        connection_sets.sort_by(|left, right| left.name.cmp(&right.name));
        if connection_sets
            .windows(2)
            .any(|window| window[0].name == window[1].name)
        {
            return Err(NativeMediaPayloadError::DuplicateConnectionSet);
        }
        let connection_sets = connection_sets.into_boxed_slice();
        let (semantic_digest_sha256, resident_bytes) =
            project_face_landmarks(image_height, image_width, &frames, &connection_sets)?;
        Ok(Self {
            image_height,
            image_width,
            frames,
            connection_sets,
            semantic_digest_sha256,
            resident_bytes,
        })
    }

    pub const fn image_height(&self) -> u32 {
        self.image_height
    }

    pub const fn image_width(&self) -> u32 {
        self.image_width
    }

    pub fn frames(&self) -> &[Box<[NativeFaceLandmark]>] {
        &self.frames
    }

    pub fn connection_sets(&self) -> &[NativeFaceConnectionSet] {
        &self.connection_sets
    }

    pub const fn semantic_digest_sha256(&self) -> &[u8; 32] {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn validate(&self) -> Result<(), NativeMediaPayloadError> {
        require_image_size(self.image_height, self.image_width)?;
        let (digest, resident_bytes) = project_face_landmarks(
            self.image_height,
            self.image_width,
            &self.frames,
            &self.connection_sets,
        )?;
        require_projection(
            self.semantic_digest_sha256,
            digest,
            self.resident_bytes,
            resident_bytes,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NativePoseKeypoint {
    x: f64,
    y: f64,
    score: f64,
}

impl NativePoseKeypoint {
    pub fn checked(x: f64, y: f64, score: f64) -> Result<Self, NativeMediaPayloadError> {
        require_finite("pose keypoint x", x)?;
        require_finite("pose keypoint y", y)?;
        require_probability("pose keypoint score", score)?;
        Ok(Self { x, y, score })
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }

    pub const fn score(self) -> f64 {
        self.score
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativePosePerson {
    pose: Box<[NativePoseKeypoint]>,
    foot: Box<[NativePoseKeypoint]>,
    face: Box<[NativePoseKeypoint]>,
    hand_right: Box<[NativePoseKeypoint]>,
    hand_left: Box<[NativePoseKeypoint]>,
}

impl NativePosePerson {
    pub fn checked(
        pose: Vec<NativePoseKeypoint>,
        foot: Vec<NativePoseKeypoint>,
        face: Vec<NativePoseKeypoint>,
        hand_right: Vec<NativePoseKeypoint>,
        hand_left: Vec<NativePoseKeypoint>,
    ) -> Result<Self, NativeMediaPayloadError> {
        check_exact_count("body pose keypoints", pose.len(), 18)?;
        check_exact_count("foot keypoints", foot.len(), 6)?;
        check_exact_count("face pose keypoints", face.len(), 70)?;
        check_exact_count("right hand keypoints", hand_right.len(), 21)?;
        check_exact_count("left hand keypoints", hand_left.len(), 21)?;
        Ok(Self {
            pose: pose.into_boxed_slice(),
            foot: foot.into_boxed_slice(),
            face: face.into_boxed_slice(),
            hand_right: hand_right.into_boxed_slice(),
            hand_left: hand_left.into_boxed_slice(),
        })
    }

    pub fn pose(&self) -> &[NativePoseKeypoint] {
        &self.pose
    }

    pub fn foot(&self) -> &[NativePoseKeypoint] {
        &self.foot
    }

    pub fn face(&self) -> &[NativePoseKeypoint] {
        &self.face
    }

    pub fn hand_right(&self) -> &[NativePoseKeypoint] {
        &self.hand_right
    }

    pub fn hand_left(&self) -> &[NativePoseKeypoint] {
        &self.hand_left
    }

    fn validate(&self) -> Result<(), NativeMediaPayloadError> {
        check_exact_count("body pose keypoints", self.pose.len(), 18)?;
        check_exact_count("foot keypoints", self.foot.len(), 6)?;
        check_exact_count("face pose keypoints", self.face.len(), 70)?;
        check_exact_count("right hand keypoints", self.hand_right.len(), 21)?;
        check_exact_count("left hand keypoints", self.hand_left.len(), 21)?;
        for point in self
            .pose
            .iter()
            .chain(self.foot.iter())
            .chain(self.face.iter())
            .chain(self.hand_right.iter())
            .chain(self.hand_left.iter())
        {
            NativePoseKeypoint::checked(point.x, point.y, point.score)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativePoseFrame {
    canvas_width: u32,
    canvas_height: u32,
    people: Box<[NativePosePerson]>,
}

impl NativePoseFrame {
    pub fn checked(
        canvas_width: u32,
        canvas_height: u32,
        people: Vec<NativePosePerson>,
    ) -> Result<Self, NativeMediaPayloadError> {
        require_image_size(canvas_height, canvas_width)?;
        check_count("people per pose frame", people.len(), MAX_ITEMS)?;
        for person in &people {
            person.validate()?;
        }
        Ok(Self {
            canvas_width,
            canvas_height,
            people: people.into_boxed_slice(),
        })
    }

    pub const fn canvas_width(&self) -> u32 {
        self.canvas_width
    }

    pub const fn canvas_height(&self) -> u32 {
        self.canvas_height
    }

    pub fn people(&self) -> &[NativePosePerson] {
        &self.people
    }
}

#[derive(Clone, Debug)]
pub struct NativePoseKeypointPayload {
    frames: Box<[NativePoseFrame]>,
    semantic_digest_sha256: [u8; 32],
    resident_bytes: u64,
}

impl NativePoseKeypointPayload {
    pub const SOURCE_TYPE_ID: &'static str = "POSE_KEYPOINT";

    pub fn checked(frames: Vec<NativePoseFrame>) -> Result<Self, NativeMediaPayloadError> {
        check_count("pose frames", frames.len(), MAX_FRAMES)?;
        for frame in &frames {
            require_image_size(frame.canvas_height, frame.canvas_width)?;
            for person in frame.people.iter() {
                person.validate()?;
            }
        }
        let frames = frames.into_boxed_slice();
        let (semantic_digest_sha256, resident_bytes) = project_pose_keypoints(&frames)?;
        Ok(Self {
            frames,
            semantic_digest_sha256,
            resident_bytes,
        })
    }

    pub fn frames(&self) -> &[NativePoseFrame] {
        &self.frames
    }

    pub const fn semantic_digest_sha256(&self) -> &[u8; 32] {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn validate(&self) -> Result<(), NativeMediaPayloadError> {
        let (digest, resident_bytes) = project_pose_keypoints(&self.frames)?;
        require_projection(
            self.semantic_digest_sha256,
            digest,
            self.resident_bytes,
            resident_bytes,
        )
    }
}

#[derive(Clone, Debug)]
pub struct NativeTracksPayload {
    track_path: Tensor,
    track_visibility: Tensor,
    semantic_digest_sha256: [u8; 32],
    resident_bytes: u64,
}

impl NativeTracksPayload {
    pub const SOURCE_TYPE_ID: &'static str = "TRACKS";

    pub fn checked(
        track_path: Tensor,
        track_visibility: Tensor,
    ) -> Result<Self, NativeMediaPayloadError> {
        validate_tracks(&track_path, &track_visibility)?;
        let (semantic_digest_sha256, resident_bytes) =
            project_tracks(&track_path, &track_visibility)?;
        Ok(Self {
            track_path,
            track_visibility,
            semantic_digest_sha256,
            resident_bytes,
        })
    }

    pub const fn track_path(&self) -> &Tensor {
        &self.track_path
    }

    pub const fn track_visibility(&self) -> &Tensor {
        &self.track_visibility
    }

    pub const fn semantic_digest_sha256(&self) -> &[u8; 32] {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn validate(&self) -> Result<(), NativeMediaPayloadError> {
        validate_tracks(&self.track_path, &self.track_visibility)?;
        let (digest, resident_bytes) = project_tracks(&self.track_path, &self.track_visibility)?;
        require_projection(
            self.semantic_digest_sha256,
            digest,
            self.resident_bytes,
            resident_bytes,
        )
    }
}

#[derive(Clone, Debug)]
pub struct NativeSam3TrackDataPayload {
    packed_masks: Option<Tensor>,
    frame_count: u64,
    scores: Box<[f64]>,
    original_height: u32,
    original_width: u32,
    semantic_digest_sha256: [u8; 32],
    resident_bytes: u64,
}

impl NativeSam3TrackDataPayload {
    pub const SOURCE_TYPE_ID: &'static str = "SAM3_TRACK_DATA";

    pub fn checked(
        packed_masks: Option<Tensor>,
        frame_count: u64,
        scores: Vec<f64>,
        original_height: u32,
        original_width: u32,
    ) -> Result<Self, NativeMediaPayloadError> {
        require_image_size(original_height, original_width)?;
        validate_sam3_track_data(packed_masks.as_ref(), frame_count, &scores)?;
        let scores = scores.into_boxed_slice();
        let (semantic_digest_sha256, resident_bytes) = project_sam3_track_data(
            packed_masks.as_ref(),
            frame_count,
            &scores,
            original_height,
            original_width,
        )?;
        Ok(Self {
            packed_masks,
            frame_count,
            scores,
            original_height,
            original_width,
            semantic_digest_sha256,
            resident_bytes,
        })
    }

    pub const fn packed_masks(&self) -> Option<&Tensor> {
        self.packed_masks.as_ref()
    }

    pub const fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn scores(&self) -> &[f64] {
        &self.scores
    }

    pub const fn original_height(&self) -> u32 {
        self.original_height
    }

    pub const fn original_width(&self) -> u32 {
        self.original_width
    }

    pub const fn semantic_digest_sha256(&self) -> &[u8; 32] {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn validate(&self) -> Result<(), NativeMediaPayloadError> {
        require_image_size(self.original_height, self.original_width)?;
        validate_sam3_track_data(self.packed_masks.as_ref(), self.frame_count, &self.scores)?;
        let (digest, resident_bytes) = project_sam3_track_data(
            self.packed_masks.as_ref(),
            self.frame_count,
            &self.scores,
            self.original_height,
            self.original_width,
        )?;
        require_projection(
            self.semantic_digest_sha256,
            digest,
            self.resident_bytes,
            resident_bytes,
        )
    }
}

#[derive(Debug, Error)]
pub enum NativeMediaPayloadError {
    #[error("{field} contains a non-finite value")]
    NonFinite { field: &'static str },
    #[error("{0} must be non-negative")]
    Negative(&'static str),
    #[error("{0} must be in the inclusive range 0..=1")]
    NotProbability(&'static str),
    #[error("{field} contains {actual} items, maximum {maximum}")]
    TooMany {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    #[error("{field} contains {actual} items, expected exactly {expected}")]
    WrongCardinality {
        field: &'static str,
        actual: usize,
        expected: usize,
    },
    #[error("{0} must contain 1..={MAX_TEXT_BYTES} non-control UTF-8 bytes")]
    InvalidText(&'static str),
    #[error("{0} dimensions must both be nonzero")]
    InvalidImageSize(&'static str),
    #[error("{0} has reversed extents")]
    InvalidExtent(&'static str),
    #[error("blendshape {index} is `{actual}`, expected `{expected}`")]
    UnexpectedBlendshape {
        index: usize,
        expected: &'static str,
        actual: String,
    },
    #[error("face connection {0:?} is invalid")]
    InvalidFaceConnection([u16; 2]),
    #[error("face connection is duplicated")]
    DuplicateFaceConnection,
    #[error("face connection-set name is duplicated")]
    DuplicateConnectionSet,
    #[error("TRACKS tensor descriptors do not match the source contract")]
    InvalidTracksShape,
    #[error("SAM3_TRACK_DATA fields do not match the source contract")]
    InvalidSam3TrackData,
    #[error("payload resident-byte accounting overflowed")]
    ResidentBytesOverflow,
    #[error("payload digest or resident-byte projection changed")]
    ProjectionChanged,
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    DescriptorEncoding(#[from] serde_json::Error),
}

fn project_bounding_boxes(
    frames: &[Box<[NativeBoundingBox]>],
) -> Result<([u8; 32], u64), NativeMediaPayloadError> {
    check_count("bounding box frames", frames.len(), MAX_FRAMES)?;
    let mut projection =
        Projection::new::<NativeBoundingBoxPayload>(b"sim.comfy.media.bounding-box.v1")?;
    projection.hash_len(frames.len())?;
    projection.add_allocation::<Box<[NativeBoundingBox]>>(frames.len())?;
    for frame in frames {
        check_count("bounding boxes per frame", frame.len(), MAX_ITEMS)?;
        projection.hash_len(frame.len())?;
        projection.add_allocation::<NativeBoundingBox>(frame.len())?;
        for bounding_box in frame.iter() {
            bounding_box.validate()?;
            projection.hash_f64(bounding_box.x);
            projection.hash_f64(bounding_box.y);
            projection.hash_f64(bounding_box.width);
            projection.hash_f64(bounding_box.height);
            projection.hash_optional_text(bounding_box.label.as_deref())?;
            projection.hash_optional_f64(bounding_box.score);
        }
    }
    Ok(projection.finish())
}

fn project_face_landmarks(
    image_height: u32,
    image_width: u32,
    frames: &[Box<[NativeFaceLandmark]>],
    connection_sets: &[NativeFaceConnectionSet],
) -> Result<([u8; 32], u64), NativeMediaPayloadError> {
    require_image_size(image_height, image_width)?;
    check_count("face landmark frames", frames.len(), MAX_FRAMES)?;
    check_count("face connection sets", connection_sets.len(), MAX_ITEMS)?;
    let mut projection =
        Projection::new::<NativeFaceLandmarksPayload>(b"sim.comfy.media.face-landmarks.v1")?;
    projection.hasher.update(image_height.to_le_bytes());
    projection.hasher.update(image_width.to_le_bytes());
    projection.hash_len(frames.len())?;
    projection.add_allocation::<Box<[NativeFaceLandmark]>>(frames.len())?;
    for frame in frames {
        check_count("faces per frame", frame.len(), MAX_ITEMS)?;
        projection.hash_len(frame.len())?;
        projection.add_allocation::<NativeFaceLandmark>(frame.len())?;
        for face in frame.iter() {
            face.validate()?;
            for value in face.bbox_xyxy {
                projection.hash_f64(value);
            }
            projection.hash_len(face.blendshapes.len())?;
            projection.add_allocation::<NativeFaceBlendshape>(face.blendshapes.len())?;
            for blendshape in face.blendshapes.iter() {
                projection.hash_text(&blendshape.name)?;
                projection.hash_f64(blendshape.score);
            }
            projection.hash_len(face.landmarks_xy.len())?;
            projection.add_allocation::<NativePoint2>(face.landmarks_xy.len())?;
            for point in face.landmarks_xy.iter() {
                projection.hash_f64(point.x);
                projection.hash_f64(point.y);
            }
            projection.hash_len(face.landmarks_3d.len())?;
            projection.add_allocation::<NativePoint3>(face.landmarks_3d.len())?;
            for point in face.landmarks_3d.iter() {
                projection.hash_f64(point.x);
                projection.hash_f64(point.y);
                projection.hash_f64(point.z);
            }
            projection.hash_f64(face.presence);
            projection.hash_f64(face.score);
            for value in face.transformation_matrix {
                projection.hash_f64(value);
            }
        }
    }
    projection.hash_len(connection_sets.len())?;
    projection.add_allocation::<NativeFaceConnectionSet>(connection_sets.len())?;
    let mut names = BTreeSet::new();
    for connection_set in connection_sets {
        if !names.insert(connection_set.name()) {
            return Err(NativeMediaPayloadError::DuplicateConnectionSet);
        }
        projection.hash_text(&connection_set.name)?;
        projection.hash_len(connection_set.edges.len())?;
        projection.add_allocation::<[u16; 2]>(connection_set.edges.len())?;
        let mut previous = None;
        for edge in connection_set.edges.iter() {
            if usize::from(edge[0]) >= FACE_LANDMARK_COUNT
                || usize::from(edge[1]) >= FACE_LANDMARK_COUNT
                || edge[0] == edge[1]
            {
                return Err(NativeMediaPayloadError::InvalidFaceConnection(*edge));
            }
            if previous.is_some_and(|previous| previous >= *edge) {
                return Err(NativeMediaPayloadError::DuplicateFaceConnection);
            }
            projection.hasher.update(edge[0].to_le_bytes());
            projection.hasher.update(edge[1].to_le_bytes());
            previous = Some(*edge);
        }
    }
    Ok(projection.finish())
}

fn project_pose_keypoints(
    frames: &[NativePoseFrame],
) -> Result<([u8; 32], u64), NativeMediaPayloadError> {
    check_count("pose frames", frames.len(), MAX_FRAMES)?;
    let mut projection =
        Projection::new::<NativePoseKeypointPayload>(b"sim.comfy.media.pose-keypoint.v1")?;
    projection.hash_len(frames.len())?;
    projection.add_allocation::<NativePoseFrame>(frames.len())?;
    for frame in frames {
        require_image_size(frame.canvas_height, frame.canvas_width)?;
        projection.hasher.update(frame.canvas_width.to_le_bytes());
        projection.hasher.update(frame.canvas_height.to_le_bytes());
        projection.hash_len(frame.people.len())?;
        projection.add_allocation::<NativePosePerson>(frame.people.len())?;
        for person in frame.people.iter() {
            person.validate()?;
            for points in [
                person.pose.as_ref(),
                person.foot.as_ref(),
                person.face.as_ref(),
                person.hand_right.as_ref(),
                person.hand_left.as_ref(),
            ] {
                projection.hash_len(points.len())?;
                projection.add_allocation::<NativePoseKeypoint>(points.len())?;
                for point in points {
                    projection.hash_f64(point.x);
                    projection.hash_f64(point.y);
                    projection.hash_f64(point.score);
                }
            }
        }
    }
    Ok(projection.finish())
}

fn validate_tracks(
    track_path: &Tensor,
    track_visibility: &Tensor,
) -> Result<(), NativeMediaPayloadError> {
    let path = track_path.descriptor();
    let visibility = track_visibility.descriptor();
    let path_shape = path.shape();
    let visibility_shape = visibility.shape();
    if path.dtype() != DType::F32
        || visibility.dtype() != DType::Bool
        || path_shape.len() != 3
        || path_shape.get(2) != Some(&2)
        || visibility_shape.len() != 2
        || path_shape.get(0) != visibility_shape.first()
        || path_shape.get(1) != visibility_shape.get(1)
        || path.device() != visibility.device()
        || path.stream() != visibility.stream()
        || !path.is_contiguous()?
        || !visibility.is_contiguous()?
    {
        return Err(NativeMediaPayloadError::InvalidTracksShape);
    }
    track_path.contiguous_bytes()?;
    track_visibility.contiguous_bytes()?;
    Ok(())
}

fn project_tracks(
    track_path: &Tensor,
    track_visibility: &Tensor,
) -> Result<([u8; 32], u64), NativeMediaPayloadError> {
    let mut projection = Projection::new::<NativeTracksPayload>(b"sim.comfy.media.tracks.v1")?;
    projection.hash_tensor(track_path)?;
    projection.hash_tensor(track_visibility)?;
    projection.add_tensor_storages([track_path, track_visibility])?;
    Ok(projection.finish())
}

fn validate_sam3_track_data(
    packed_masks: Option<&Tensor>,
    frame_count: u64,
    scores: &[f64],
) -> Result<(), NativeMediaPayloadError> {
    check_count("SAM3 scores", scores.len(), MAX_ITEMS)?;
    for score in scores {
        require_probability("SAM3 object score", *score)?;
    }
    match packed_masks {
        Some(packed_masks) => {
            let descriptor = packed_masks.descriptor();
            let shape = descriptor.shape();
            let object_count = shape.get(1).copied();
            if descriptor.dtype() != DType::U8
                || shape.len() != 4
                || shape.first().copied() != Some(frame_count)
                || object_count.and_then(|count| usize::try_from(count).ok()) != Some(scores.len())
                || shape.get(2).copied() == Some(0)
                || shape.get(3).copied() == Some(0)
                || !descriptor.is_contiguous()?
            {
                return Err(NativeMediaPayloadError::InvalidSam3TrackData);
            }
            packed_masks.contiguous_bytes()?;
        }
        None if !scores.is_empty() => {
            return Err(NativeMediaPayloadError::InvalidSam3TrackData);
        }
        None => {}
    }
    Ok(())
}

fn project_sam3_track_data(
    packed_masks: Option<&Tensor>,
    frame_count: u64,
    scores: &[f64],
    original_height: u32,
    original_width: u32,
) -> Result<([u8; 32], u64), NativeMediaPayloadError> {
    let mut projection =
        Projection::new::<NativeSam3TrackDataPayload>(b"sim.comfy.media.sam3-track-data.v1")?;
    projection.hasher.update(frame_count.to_le_bytes());
    projection.hasher.update(original_height.to_le_bytes());
    projection.hasher.update(original_width.to_le_bytes());
    projection.hash_len(scores.len())?;
    projection.add_allocation::<f64>(scores.len())?;
    for score in scores {
        projection.hash_f64(*score);
    }
    match packed_masks {
        Some(packed_masks) => {
            projection.hasher.update([1]);
            projection.hash_tensor(packed_masks)?;
            projection.add_tensor_storages([packed_masks])?;
        }
        None => projection.hasher.update([0]),
    }
    Ok(projection.finish())
}

struct Projection {
    hasher: Sha256,
    resident_bytes: u64,
}

impl Projection {
    fn new<Payload>(domain: &[u8]) -> Result<Self, NativeMediaPayloadError> {
        let mut hasher = Sha256::new();
        hasher.update(domain);
        hasher.update([0]);
        Ok(Self {
            hasher,
            resident_bytes: u64::try_from(mem::size_of::<Payload>())
                .map_err(|_| NativeMediaPayloadError::ResidentBytesOverflow)?,
        })
    }

    fn hash_len(&mut self, length: usize) -> Result<(), NativeMediaPayloadError> {
        self.hasher.update(
            u64::try_from(length)
                .map_err(|_| NativeMediaPayloadError::ResidentBytesOverflow)?
                .to_le_bytes(),
        );
        Ok(())
    }

    fn hash_f64(&mut self, value: f64) {
        self.hasher.update(value.to_bits().to_le_bytes());
    }

    fn hash_optional_f64(&mut self, value: Option<f64>) {
        match value {
            Some(value) => {
                self.hasher.update([1]);
                self.hash_f64(value);
            }
            None => self.hasher.update([0]),
        }
    }

    fn hash_text(&mut self, value: &str) -> Result<(), NativeMediaPayloadError> {
        self.hash_len(value.len())?;
        self.hasher.update(value.as_bytes());
        self.add_bytes(value.len())
    }

    fn hash_optional_text(&mut self, value: Option<&str>) -> Result<(), NativeMediaPayloadError> {
        match value {
            Some(value) => {
                self.hasher.update([1]);
                self.hash_text(value)
            }
            None => {
                self.hasher.update([0]);
                Ok(())
            }
        }
    }

    fn hash_tensor(&mut self, tensor: &Tensor) -> Result<(), NativeMediaPayloadError> {
        let descriptor = serde_json::to_vec(tensor.descriptor())?;
        self.hash_len(descriptor.len())?;
        self.hasher.update(descriptor);
        let bytes = tensor.contiguous_bytes()?;
        self.hash_len(bytes.len())?;
        self.hasher.update(bytes);
        Ok(())
    }

    fn add_allocation<Value>(&mut self, length: usize) -> Result<(), NativeMediaPayloadError> {
        let bytes = mem::size_of::<Value>()
            .checked_mul(length)
            .ok_or(NativeMediaPayloadError::ResidentBytesOverflow)?;
        self.add_bytes(bytes)
    }

    fn add_bytes(&mut self, bytes: usize) -> Result<(), NativeMediaPayloadError> {
        let bytes =
            u64::try_from(bytes).map_err(|_| NativeMediaPayloadError::ResidentBytesOverflow)?;
        self.resident_bytes = self
            .resident_bytes
            .checked_add(bytes)
            .ok_or(NativeMediaPayloadError::ResidentBytesOverflow)?;
        Ok(())
    }

    fn add_tensor_storages<'a>(
        &mut self,
        tensors: impl IntoIterator<Item = &'a Tensor>,
    ) -> Result<(), NativeMediaPayloadError> {
        let mut seen = BTreeSet::<u64>::new();
        for tensor in tensors {
            let storage_id: StorageId = tensor.storage_id();
            if seen.insert(storage_id.get()) {
                self.resident_bytes = self
                    .resident_bytes
                    .checked_add(tensor.storage_byte_len())
                    .ok_or(NativeMediaPayloadError::ResidentBytesOverflow)?;
            }
        }
        Ok(())
    }

    fn finish(self) -> ([u8; 32], u64) {
        (self.hasher.finalize().into(), self.resident_bytes)
    }
}

fn require_projection(
    actual_digest: [u8; 32],
    expected_digest: [u8; 32],
    actual_resident_bytes: u64,
    expected_resident_bytes: u64,
) -> Result<(), NativeMediaPayloadError> {
    if actual_digest != expected_digest || actual_resident_bytes != expected_resident_bytes {
        return Err(NativeMediaPayloadError::ProjectionChanged);
    }
    Ok(())
}

fn require_finite(field: &'static str, value: f64) -> Result<(), NativeMediaPayloadError> {
    if !value.is_finite() {
        return Err(NativeMediaPayloadError::NonFinite { field });
    }
    Ok(())
}

fn require_non_negative(field: &'static str, value: f64) -> Result<(), NativeMediaPayloadError> {
    require_finite(field, value)?;
    if value < 0.0 {
        return Err(NativeMediaPayloadError::Negative(field));
    }
    Ok(())
}

fn require_probability(field: &'static str, value: f64) -> Result<(), NativeMediaPayloadError> {
    require_finite(field, value)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(NativeMediaPayloadError::NotProbability(field));
    }
    Ok(())
}

fn check_count(
    field: &'static str,
    actual: usize,
    maximum: usize,
) -> Result<(), NativeMediaPayloadError> {
    if actual > maximum {
        return Err(NativeMediaPayloadError::TooMany {
            field,
            actual,
            maximum,
        });
    }
    Ok(())
}

fn check_exact_count(
    field: &'static str,
    actual: usize,
    expected: usize,
) -> Result<(), NativeMediaPayloadError> {
    if actual != expected {
        return Err(NativeMediaPayloadError::WrongCardinality {
            field,
            actual,
            expected,
        });
    }
    Ok(())
}

fn checked_text(field: &'static str, value: String) -> Result<String, NativeMediaPayloadError> {
    if value.is_empty() || value.len() > MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(NativeMediaPayloadError::InvalidText(field));
    }
    Ok(value)
}

fn require_image_size(height: u32, width: u32) -> Result<(), NativeMediaPayloadError> {
    if height == 0 || width == 0 {
        return Err(NativeMediaPayloadError::InvalidImageSize("image"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_tensor::{
        CancellationToken, CpuWorkspaceAuthority, DeviceId, ExecutionContext, StreamId,
        TensorDescriptor,
    };
    use std::{error::Error, mem};

    fn tensor(shape: Vec<u64>, dtype: DType, bytes: Vec<u8>) -> Result<Tensor, Box<dyn Error>> {
        let descriptor =
            TensorDescriptor::contiguous(shape, dtype, DeviceId::CPU, StreamId::DEFAULT)?;
        let byte_count = u64::try_from(bytes.len())?;
        let (backend, authority) =
            CpuWorkspaceAuthority::create_backend(byte_count.saturating_add(64))?;
        let cancellation = CancellationToken::default();
        let context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority.authorize_workspace(0)?,
            rng_phase: None,
            cancellation: &cancellation,
        };
        let (tensor, _) = backend.upload_bytes(descriptor, &bytes, &context)?;
        Ok(tensor)
    }

    fn keypoints(count: usize) -> Result<Vec<NativePoseKeypoint>, NativeMediaPayloadError> {
        (0..count)
            .map(|index| {
                let coordinate = u32::try_from(index)
                    .map(f64::from)
                    .map_err(|_| NativeMediaPayloadError::ResidentBytesOverflow)?;
                NativePoseKeypoint::checked(coordinate, coordinate + 0.5, 0.75)
            })
            .collect()
    }

    fn pose_person() -> Result<NativePosePerson, NativeMediaPayloadError> {
        NativePosePerson::checked(
            keypoints(18)?,
            keypoints(6)?,
            keypoints(70)?,
            keypoints(21)?,
            keypoints(21)?,
        )
    }

    fn face() -> Result<NativeFaceLandmark, NativeMediaPayloadError> {
        let blendshapes = MEDIAPIPE_FACE_BLENDSHAPE_NAMES
            .iter()
            .map(|name| NativeFaceBlendshape::checked((*name).to_owned(), 0.25))
            .collect::<Result<Vec<_>, _>>()?;
        let point_2d = NativePoint2::checked(12.0, 24.0)?;
        let point_3d = NativePoint3::checked(1.0, 2.0, 3.0)?;
        NativeFaceLandmark::checked(
            [10.0, 20.0, 30.0, 40.0],
            blendshapes,
            vec![point_2d; FACE_LANDMARK_COUNT],
            vec![point_3d; FACE_LANDMARK_COUNT],
            -0.5,
            0.9,
            [0.0; 16],
        )
    }

    #[test]
    fn bounding_boxes_preserve_nested_source_shape_and_exact_accounting()
    -> Result<(), Box<dyn Error>> {
        let first =
            NativeBoundingBox::checked(-2.0, 3.0, 10.0, 20.0, Some("face".to_owned()), Some(0.75))?;
        let second = NativeBoundingBox::checked(0.0, 0.0, 0.0, 0.0, None, None)?;
        let payload = NativeBoundingBoxPayload::checked(vec![vec![first], vec![second]])?;

        payload.validate()?;
        assert_eq!(payload.frames().len(), 2);
        assert_eq!(payload.frames()[0][0].label(), Some("face"));
        let expected = u64::try_from(mem::size_of::<NativeBoundingBoxPayload>())?
            + u64::try_from(2 * mem::size_of::<Box<[NativeBoundingBox]>>())?
            + u64::try_from(2 * mem::size_of::<NativeBoundingBox>())?
            + 4;
        assert_eq!(payload.resident_bytes(), expected);

        let changed = NativeBoundingBoxPayload::checked(vec![vec![NativeBoundingBox::checked(
            -2.0,
            3.0,
            11.0,
            20.0,
            Some("face".to_owned()),
            Some(0.75),
        )?]])?;
        assert_ne!(
            payload.semantic_digest_sha256(),
            changed.semantic_digest_sha256()
        );
        assert!(NativeBoundingBox::checked(0.0, 0.0, -1.0, 1.0, None, None).is_err());
        assert!(NativeBoundingBox::checked(0.0, 0.0, 1.0, 1.0, None, Some(1.1)).is_err());
        Ok(())
    }

    #[test]
    fn face_landmarks_enforce_mediapipe_cardinalities_and_topology() -> Result<(), Box<dyn Error>> {
        let connection =
            NativeFaceConnectionSet::checked("face_oval".to_owned(), vec![[10, 20], [0, 1]])?;
        let payload =
            NativeFaceLandmarksPayload::checked(480, 640, vec![vec![face()?]], vec![connection])?;

        payload.validate()?;
        assert_eq!(payload.image_height(), 480);
        assert_eq!(payload.frames()[0][0].landmarks_xy().len(), 478);
        assert_eq!(payload.frames()[0][0].blendshapes().len(), 52);
        assert_eq!(payload.connection_sets()[0].edges(), &[[0, 1], [10, 20]]);
        assert!(
            payload.resident_bytes() > u64::try_from(mem::size_of::<NativeFaceLandmarksPayload>())?
        );

        let mut wrong_blendshapes = MEDIAPIPE_FACE_BLENDSHAPE_NAMES
            .iter()
            .map(|name| NativeFaceBlendshape::checked((*name).to_owned(), 0.0))
            .collect::<Result<Vec<_>, _>>()?;
        wrong_blendshapes[0] = NativeFaceBlendshape::checked("neutral".to_owned(), 0.0)?;
        assert!(
            NativeFaceLandmark::checked(
                [0.0, 0.0, 1.0, 1.0],
                wrong_blendshapes,
                vec![NativePoint2::checked(0.0, 0.0)?; FACE_LANDMARK_COUNT],
                vec![NativePoint3::checked(0.0, 0.0, 0.0)?; FACE_LANDMARK_COUNT],
                0.0,
                1.0,
                [0.0; 16],
            )
            .is_err()
        );
        assert!(NativeFaceConnectionSet::checked("all".to_owned(), vec![[1, 1]]).is_err());
        assert!(NativeFaceLandmarksPayload::checked(0, 640, vec![], vec![]).is_err());
        Ok(())
    }

    #[test]
    fn pose_keypoints_enforce_openpose_frame_shape() -> Result<(), Box<dyn Error>> {
        let frame = NativePoseFrame::checked(1920, 1080, vec![pose_person()?])?;
        let payload = NativePoseKeypointPayload::checked(vec![frame])?;

        payload.validate()?;
        let person = &payload.frames()[0].people()[0];
        assert_eq!(person.pose().len(), 18);
        assert_eq!(person.foot().len(), 6);
        assert_eq!(person.face().len(), 70);
        assert_eq!(person.hand_right().len(), 21);
        assert_eq!(person.hand_left().len(), 21);
        assert!(
            NativePosePerson::checked(
                keypoints(17)?,
                keypoints(6)?,
                keypoints(70)?,
                keypoints(21)?,
                keypoints(21)?,
            )
            .is_err()
        );
        assert!(NativePoseFrame::checked(0, 1080, vec![]).is_err());
        assert!(NativePoseKeypoint::checked(0.0, 0.0, f64::NAN).is_err());
        Ok(())
    }

    #[test]
    fn tracks_require_exact_tensor_roles_and_account_storage() -> Result<(), Box<dyn Error>> {
        let path = tensor(vec![2, 3, 2], DType::F32, vec![0; 2 * 3 * 2 * 4])?;
        let visibility = tensor(vec![2, 3], DType::Bool, vec![1; 2 * 3])?;
        let expected_storage = path.storage_byte_len() + visibility.storage_byte_len();
        let payload = NativeTracksPayload::checked(path, visibility)?;

        payload.validate()?;
        assert_eq!(payload.track_path().descriptor().shape(), &[2, 3, 2]);
        assert_eq!(
            payload.resident_bytes(),
            u64::try_from(mem::size_of::<NativeTracksPayload>())? + expected_storage
        );

        let wrong_path = tensor(vec![2, 3, 3], DType::F32, vec![0; 2 * 3 * 3 * 4])?;
        let visibility = tensor(vec![2, 3], DType::Bool, vec![1; 2 * 3])?;
        assert!(NativeTracksPayload::checked(wrong_path, visibility).is_err());
        Ok(())
    }

    #[test]
    fn sam3_track_data_accepts_empty_and_packed_source_variants() -> Result<(), Box<dyn Error>> {
        let empty = NativeSam3TrackDataPayload::checked(None, 3, vec![], 720, 1280)?;
        empty.validate()?;
        assert!(empty.packed_masks().is_none());

        let packed = tensor(vec![3, 2, 8, 16], DType::U8, vec![0; 3 * 2 * 8 * 16])?;
        let packed_bytes = packed.storage_byte_len();
        let payload =
            NativeSam3TrackDataPayload::checked(Some(packed), 3, vec![1.0, 0.25], 720, 1280)?;
        payload.validate()?;
        assert_eq!(payload.frame_count(), 3);
        assert_eq!(payload.scores(), &[1.0, 0.25]);
        assert_eq!(
            payload.resident_bytes(),
            u64::try_from(mem::size_of::<NativeSam3TrackDataPayload>())?
                + u64::try_from(2 * mem::size_of::<f64>())?
                + packed_bytes
        );

        let wrong_objects = tensor(vec![3, 2, 8, 16], DType::U8, vec![0; 3 * 2 * 8 * 16])?;
        assert!(
            NativeSam3TrackDataPayload::checked(Some(wrong_objects), 3, vec![1.0], 720, 1280,)
                .is_err()
        );
        assert!(NativeSam3TrackDataPayload::checked(None, 3, vec![1.0], 720, 1280).is_err());
        Ok(())
    }

    #[test]
    fn semantic_digests_are_allocation_independent() -> Result<(), Box<dyn Error>> {
        let construct = || -> Result<NativePoseKeypointPayload, NativeMediaPayloadError> {
            NativePoseKeypointPayload::checked(vec![NativePoseFrame::checked(
                512,
                512,
                vec![pose_person()?],
            )?])
        };
        let first = construct()?;
        let second = construct()?;
        assert_eq!(
            first.semantic_digest_sha256(),
            second.semantic_digest_sha256()
        );
        assert_eq!(first.resident_bytes(), second.resident_bytes());
        assert_eq!(NativeBoundingBoxPayload::SOURCE_TYPE_ID, "BOUNDING_BOX");
        assert_eq!(NativeFaceLandmarksPayload::SOURCE_TYPE_ID, "FACE_LANDMARKS");
        assert_eq!(NativePoseKeypointPayload::SOURCE_TYPE_ID, "POSE_KEYPOINT");
        assert_eq!(
            NativeSam3TrackDataPayload::SOURCE_TYPE_ID,
            "SAM3_TRACK_DATA"
        );
        assert_eq!(NativeTracksPayload::SOURCE_TYPE_ID, "TRACKS");
        Ok(())
    }
}
