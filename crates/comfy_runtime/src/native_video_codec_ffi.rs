use crate::CertifiedVideoCodecDependencyClosure;
use comfy_types::{CancellationError, CancellationToken};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NativeVideoCodecLoadError {
    #[error("native video codec loading was cancelled")]
    Cancelled,
    #[error("native video codec loading is unsupported for this target")]
    UnsupportedTarget,
    #[error("the certified native video codec closure is incomplete")]
    InvalidClosure,
    #[error("native video codec loader handle reservation failed")]
    ResourceExhausted,
    #[error("certified native video codec library {identity} could not be loaded: {reason}")]
    LibraryLoad { identity: String, reason: String },
    #[error("the loaded native video codec namespace failed binding proof: {0}")]
    BindingProof(String),
}

impl From<CancellationError> for NativeVideoCodecLoadError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

pub struct NativeVideoCodecLoad {
    loaded: LoadedVideoCodecLibraries,
    closure: CertifiedVideoCodecDependencyClosure,
}

impl NativeVideoCodecLoad {
    pub fn target(&self) -> &str {
        self.closure.target()
    }

    pub fn primary_catalog_sha256(&self) -> &str {
        self.closure.primary_catalog_sha256()
    }

    pub fn dependency_first_order(&self) -> &[String] {
        self.closure.dependency_first_order()
    }

    pub fn loaded_library_count(&self) -> usize {
        self.loaded.libraries.len()
    }
}

pub fn load_certified_video_codec_closure(
    closure: CertifiedVideoCodecDependencyClosure,
    cancellation: &CancellationToken,
) -> Result<NativeVideoCodecLoad, NativeVideoCodecLoadError> {
    cancellation.check()?;
    if closure.target() != "x86_64-unknown-linux-gnu"
        || !cfg!(all(
            target_os = "linux",
            target_arch = "x86_64",
            target_env = "gnu"
        ))
    {
        return Err(NativeVideoCodecLoadError::UnsupportedTarget);
    }
    let projection = VideoCodecLoadProjection::from_closure(&closure)?;
    let loaded = load_video_codec_projection(&projection, cancellation)?;
    cancellation.check()?;
    Ok(NativeVideoCodecLoad { loaded, closure })
}

struct VideoCodecLoadProjection {
    paths: BTreeMap<String, PathBuf>,
    sonames: BTreeMap<String, String>,
    needed: BTreeMap<String, BTreeSet<String>>,
    system_libraries: BTreeSet<String>,
    dependency_first_order: Vec<String>,
}

impl VideoCodecLoadProjection {
    fn from_closure(
        closure: &CertifiedVideoCodecDependencyClosure,
    ) -> Result<Self, NativeVideoCodecLoadError> {
        let paths = closure
            .retained_loader_paths()
            .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
        let dependency_first_order = closure.dependency_first_order().to_vec();
        if paths.len() != dependency_first_order.len() {
            return Err(NativeVideoCodecLoadError::InvalidClosure);
        }
        let mut sonames = closure
            .primary_libraries()
            .iter()
            .map(|(identity, library)| (identity.clone(), library.filename().to_owned()))
            .collect::<BTreeMap<_, _>>();
        for (identity, dependency) in closure.dependencies() {
            if sonames
                .insert(identity.clone(), dependency.filename().to_owned())
                .is_some()
            {
                return Err(NativeVideoCodecLoadError::InvalidClosure);
            }
        }
        if sonames.len() != paths.len()
            || dependency_first_order
                .iter()
                .any(|identity| !paths.contains_key(identity) || !sonames.contains_key(identity))
        {
            return Err(NativeVideoCodecLoadError::InvalidClosure);
        }
        let mut needed = sonames
            .keys()
            .map(|identity| (identity.clone(), BTreeSet::new()))
            .collect::<BTreeMap<_, _>>();
        for edge in closure.edges() {
            let required = sonames
                .get(edge.dependency())
                .cloned()
                .unwrap_or_else(|| edge.dependency().to_owned());
            needed
                .get_mut(edge.consumer())
                .ok_or(NativeVideoCodecLoadError::InvalidClosure)?
                .insert(required);
        }
        Ok(Self {
            paths,
            sonames,
            needed,
            system_libraries: closure.reviewed_system_libraries().clone(),
            dependency_first_order,
        })
    }
}

struct LoadedVideoCodecLibrary {
    identity: String,
    path: PathBuf,
    handle: std::ptr::NonNull<std::ffi::c_void>,
    namespace: libc::c_long,
}

struct LoadedVideoCodecLibraries {
    libraries: Vec<LoadedVideoCodecLibrary>,
    _thread_bound: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl Drop for LoadedVideoCodecLibraries {
    fn drop(&mut self) {
        while let Some(library) = self.libraries.pop() {
            close_loaded_library(library);
        }
    }
}

fn load_video_codec_projection(
    projection: &VideoCodecLoadProjection,
    cancellation: &CancellationToken,
) -> Result<LoadedVideoCodecLibraries, NativeVideoCodecLoadError> {
    load_video_codec_projection_with_check(projection, || cancellation.check())
}

fn load_video_codec_projection_with_check(
    projection: &VideoCodecLoadProjection,
    mut check_cancellation: impl FnMut() -> Result<(), CancellationError>,
) -> Result<LoadedVideoCodecLibraries, NativeVideoCodecLoadError> {
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
    {
        let _ = projection;
        let _ = &mut check_cancellation;
        Err(NativeVideoCodecLoadError::UnsupportedTarget)
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    {
        check_cancellation()?;
        if projection.paths.len() != projection.dependency_first_order.len()
            || projection.sonames.len() != projection.dependency_first_order.len()
            || projection.needed.len() != projection.dependency_first_order.len()
        {
            return Err(NativeVideoCodecLoadError::InvalidClosure);
        }
        let mut libraries = Vec::new();
        libraries
            .try_reserve_exact(projection.dependency_first_order.len())
            .map_err(|_| NativeVideoCodecLoadError::ResourceExhausted)?;
        let mut loaded = LoadedVideoCodecLibraries {
            libraries,
            _thread_bound: std::marker::PhantomData,
        };
        let mut namespace = None;
        for identity in &projection.dependency_first_order {
            check_cancellation()?;
            let path = projection
                .paths
                .get(identity)
                .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
            let library = open_loaded_library(identity, path, namespace)?;
            namespace = Some(library.namespace);
            loaded.libraries.push(library);
            check_cancellation()?;
        }
        prove_exact_loaded_bindings(&loaded.libraries, projection, &mut check_cancellation)?;
        check_cancellation()?;
        Ok(loaded)
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn open_loaded_library(
    identity: &str,
    path: &std::path::Path,
    namespace: Option<libc::c_long>,
) -> Result<LoadedVideoCodecLibrary, NativeVideoCodecLoadError> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};

    let path_string = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        NativeVideoCodecLoadError::LibraryLoad {
            identity: identity.to_owned(),
            reason: "retained loader path contains an interior NUL".to_owned(),
        }
    })?;
    let handle = unsafe {
        libc::dlmopen(
            namespace.unwrap_or(libc::LM_ID_NEWLM),
            path_string.as_ptr(),
            libc::RTLD_NOW | libc::RTLD_LOCAL,
        )
    };
    let handle =
        std::ptr::NonNull::new(handle).ok_or_else(|| NativeVideoCodecLoadError::LibraryLoad {
            identity: identity.to_owned(),
            reason: dynamic_loader_error(),
        })?;
    let mut actual_namespace = 0;
    let namespace_status = unsafe {
        libc::dlinfo(
            handle.as_ptr(),
            libc::RTLD_DI_LMID,
            std::ptr::addr_of_mut!(actual_namespace).cast(),
        )
    };
    if namespace_status != 0
        || namespace.is_none() && actual_namespace == libc::LM_ID_BASE
        || namespace.is_some_and(|expected| expected != actual_namespace)
    {
        let status = unsafe { libc::dlclose(handle.as_ptr()) };
        if status != 0 {
            eprintln!("native video codec loader failed to close a rejected handle");
        }
        return Err(NativeVideoCodecLoadError::BindingProof(format!(
            "isolated loader namespace could not be proven for {identity}"
        )));
    }
    Ok(LoadedVideoCodecLibrary {
        identity: identity.to_owned(),
        path: path.to_owned(),
        handle,
        namespace: actual_namespace,
    })
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn close_loaded_library(library: LoadedVideoCodecLibrary) {
    let status = unsafe { libc::dlclose(library.handle.as_ptr()) };
    if status != 0 {
        eprintln!(
            "native video codec loader failed to close {}",
            library.identity
        );
    }
    #[cfg(test)]
    if let Ok(mut log) = TEST_CLOSE_ORDER.lock() {
        log.push(library.identity);
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
fn close_loaded_library(library: LoadedVideoCodecLibrary) {
    let _ = library;
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn dynamic_loader_error() -> String {
    use std::ffi::CStr;

    let error = unsafe { libc::dlerror() };
    if error.is_null() {
        "dynamic loader returned no diagnostic".to_owned()
    } else {
        unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[repr(C)]
struct LoaderLinkMap {
    address: usize,
    name: *const libc::c_char,
    dynamic: *mut std::ffi::c_void,
    next: *mut LoaderLinkMap,
    previous: *mut LoaderLinkMap,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[repr(C)]
struct LoaderElfDynamic {
    tag: i64,
    value: u64,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn loaded_link_map(
    library: &LoadedVideoCodecLibrary,
) -> Result<*mut LoaderLinkMap, NativeVideoCodecLoadError> {
    let mut link_map: *mut LoaderLinkMap = std::ptr::null_mut();
    let status = unsafe {
        libc::dlinfo(
            library.handle.as_ptr(),
            libc::RTLD_DI_LINKMAP,
            std::ptr::addr_of_mut!(link_map).cast(),
        )
    };
    if status != 0 || link_map.is_null() {
        Err(NativeVideoCodecLoadError::BindingProof(format!(
            "loader returned no link map for {}",
            library.identity
        )))
    } else {
        Ok(link_map)
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
unsafe fn bounded_loaded_string(
    string_table: *const u8,
    offset: u64,
) -> Result<String, NativeVideoCodecLoadError> {
    let offset = usize::try_from(offset).map_err(|_| {
        NativeVideoCodecLoadError::BindingProof(
            "loaded dynamic string offset exceeds the address space".to_owned(),
        )
    })?;
    let start = unsafe { string_table.add(offset) };
    let mut length = 0;
    while length <= 255 {
        if unsafe { *start.add(length) } == 0 {
            let bytes = unsafe { std::slice::from_raw_parts(start, length) };
            return std::str::from_utf8(bytes).map(str::to_owned).map_err(|_| {
                NativeVideoCodecLoadError::BindingProof(
                    "loaded dynamic string is not UTF-8".to_owned(),
                )
            });
        }
        length += 1;
    }
    Err(NativeVideoCodecLoadError::BindingProof(
        "loaded dynamic string exceeds 255 bytes".to_owned(),
    ))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
unsafe fn loaded_dynamic_identity(
    link_map: *mut LoaderLinkMap,
) -> Result<(String, BTreeSet<String>), NativeVideoCodecLoadError> {
    let dynamic = unsafe { (*link_map).dynamic.cast::<LoaderElfDynamic>() };
    if dynamic.is_null() {
        return Err(NativeVideoCodecLoadError::BindingProof(
            "loaded object has no dynamic table".to_owned(),
        ));
    }
    let mut string_table = std::ptr::null();
    let mut soname_offset = None;
    let mut needed_offsets = Vec::new();
    let mut terminated = false;
    for index in 0..65_536 {
        let entry = unsafe { &*dynamic.add(index) };
        match entry.tag {
            0 => {
                terminated = true;
                break;
            }
            1 => needed_offsets.push(entry.value),
            5 => string_table = entry.value as *const u8,
            14 => soname_offset = Some(entry.value),
            _ => {}
        }
    }
    if !terminated || string_table.is_null() {
        return Err(NativeVideoCodecLoadError::BindingProof(
            "loaded object has an invalid dynamic string table".to_owned(),
        ));
    }
    let soname_offset = soname_offset.ok_or_else(|| {
        NativeVideoCodecLoadError::BindingProof("loaded object has no DT_SONAME".to_owned())
    })?;
    let soname = unsafe { bounded_loaded_string(string_table, soname_offset) }?;
    let mut needed = BTreeSet::new();
    for offset in needed_offsets {
        let dependency = unsafe { bounded_loaded_string(string_table, offset) }?;
        if !needed.insert(dependency) {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "{soname} repeats a DT_NEEDED entry"
            )));
        }
    }
    Ok((soname, needed))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn prove_exact_loaded_bindings(
    libraries: &[LoadedVideoCodecLibrary],
    projection: &VideoCodecLoadProjection,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLoadError> {
    use std::{ffi::CStr, os::unix::ffi::OsStrExt};

    let expected_paths = projection
        .paths
        .iter()
        .map(|(identity, path)| (path.as_os_str().as_bytes().to_vec(), identity.as_str()))
        .collect::<BTreeMap<_, _>>();
    let expected_by_soname = projection
        .sonames
        .iter()
        .map(|(identity, soname)| (soname.as_str(), identity.as_str()))
        .collect::<BTreeMap<_, _>>();
    if expected_by_soname.len() != projection.sonames.len() {
        return Err(NativeVideoCodecLoadError::InvalidClosure);
    }

    let mut maps_by_identity = BTreeMap::new();
    for library in libraries {
        check_cancellation()?;
        let expected_path = projection
            .paths
            .get(&library.identity)
            .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
        let link_map = loaded_link_map(library)?;
        let actual_name = unsafe { (*link_map).name };
        if actual_name.is_null() {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "loader returned no object path for {}",
                library.identity
            )));
        }
        let actual_path = unsafe { CStr::from_ptr(actual_name) }.to_bytes();
        if actual_path != expected_path.as_os_str().as_bytes()
            || library.path.as_os_str().as_bytes() != actual_path
        {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "{} did not resolve to its retained descriptor",
                library.identity
            )));
        }
        if maps_by_identity
            .insert(library.identity.as_str(), link_map)
            .is_some()
        {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "loader returned duplicate handle for {}",
                library.identity
            )));
        }
    }

    let mut head = *maps_by_identity
        .values()
        .next()
        .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
    let mut walked = BTreeSet::new();
    while !head.is_null() {
        check_cancellation()?;
        if !walked.insert(head as usize) || walked.len() > 4_096 {
            return Err(NativeVideoCodecLoadError::BindingProof(
                "loader namespace link map is cyclic or exceeds 4096 objects".to_owned(),
            ));
        }
        let previous = unsafe { (*head).previous };
        if previous.is_null() {
            break;
        }
        head = previous;
    }

    walked.clear();
    let mut observed_explicit = BTreeSet::new();
    let mut current = head;
    while !current.is_null() {
        check_cancellation()?;
        if !walked.insert(current as usize) || walked.len() > 4_096 {
            return Err(NativeVideoCodecLoadError::BindingProof(
                "loader namespace link map is cyclic or exceeds 4096 objects".to_owned(),
            ));
        }
        let name_pointer = unsafe { (*current).name };
        if name_pointer.is_null() {
            return Err(NativeVideoCodecLoadError::BindingProof(
                "loader namespace contains an unnamed object".to_owned(),
            ));
        }
        let loaded_path = unsafe { CStr::from_ptr(name_pointer) }.to_bytes();
        if let Some(identity) = expected_paths.get(loaded_path) {
            if !observed_explicit.insert(*identity) {
                return Err(NativeVideoCodecLoadError::BindingProof(format!(
                    "certified object {identity} appears more than once"
                )));
            }
        } else if !loaded_path.is_empty() {
            let basename = loaded_path
                .rsplit(|byte| *byte == b'/')
                .next()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .unwrap_or_default();
            if expected_by_soname.contains_key(basename) {
                return Err(NativeVideoCodecLoadError::BindingProof(format!(
                    "ambient object duplicates certified SONAME {basename}"
                )));
            }
            if !projection.system_libraries.contains(basename) {
                return Err(NativeVideoCodecLoadError::BindingProof(format!(
                    "loader namespace contains undeclared object {}",
                    String::from_utf8_lossy(loaded_path)
                )));
            }
        }
        current = unsafe { (*current).next };
    }
    if observed_explicit.len() != projection.paths.len() {
        return Err(NativeVideoCodecLoadError::BindingProof(
            "loader namespace omits a certified object".to_owned(),
        ));
    }

    for (identity, link_map) in maps_by_identity {
        check_cancellation()?;
        let (actual_soname, actual_needed) = unsafe { loaded_dynamic_identity(link_map) }?;
        let expected_soname = projection
            .sonames
            .get(identity)
            .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
        let expected_needed = projection
            .needed
            .get(identity)
            .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
        if &actual_soname != expected_soname || &actual_needed != expected_needed {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "loaded dynamic identity differs for {identity}"
            )));
        }
    }
    Ok(())
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
fn prove_exact_loaded_bindings(
    _libraries: &[LoadedVideoCodecLibrary],
    _projection: &VideoCodecLoadProjection,
    _check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLoadError> {
    Err(NativeVideoCodecLoadError::UnsupportedTarget)
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
static TEST_CLOSE_ORDER: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

#[cfg(all(test, target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(test, target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
mod tests {
    use super::*;
    use crate::{
        native_ffi_elf::inspect_elf64_dynamic_contract, trust::capture_native_library_image,
    };
    use std::{fs, process::Command};

    struct LoaderFixture {
        _directory: tempfile::TempDir,
        _retained: Vec<crate::trust::RetainedNativeLibraryImage>,
        projection: VideoCodecLoadProjection,
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "the Linux-only retained-loader test synchronously compiles two tiny ELF fixtures before dlmopen"
    )]
    fn fixture() -> Result<LoaderFixture, Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let dependency_source = directory.path().join("dependency.c");
        fs::write(
            &dependency_source,
            "int video_codec_dependency(void) { return 7; }\n",
        )?;
        let dependency = directory.path().join("libvideo_dependency.so.1");
        let output = Command::new("cc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-Wl,-soname,libvideo_dependency.so.1")
            .arg(&dependency_source)
            .arg("-o")
            .arg(&dependency)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "fixture dependency compiler failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let consumer_source = directory.path().join("consumer.c");
        fs::write(
            &consumer_source,
            "extern int video_codec_dependency(void);\nint video_codec_consumer(void) { return video_codec_dependency(); }\n",
        )?;
        let consumer = directory.path().join("libvideo_consumer.so.1");
        let output = Command::new("cc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-Wl,-soname,libvideo_consumer.so.1")
            .arg("-Wl,-z,defs")
            .arg(&consumer_source)
            .arg("-L")
            .arg(directory.path())
            .arg("-l:libvideo_dependency.so.1")
            .arg("-o")
            .arg(&consumer)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "fixture consumer compiler failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let cancellation = CancellationToken::default();
        let mut retained = Vec::new();
        let mut paths = BTreeMap::new();
        let mut sonames = BTreeMap::new();
        let mut needed = BTreeMap::new();
        for (identity, path, soname) in [
            ("dependency", dependency, "libvideo_dependency.so.1"),
            ("consumer", consumer, "libvideo_consumer.so.1"),
        ] {
            let bytes = fs::read(&path)?;
            let dynamic = inspect_elf64_dynamic_contract(&bytes, 62, &cancellation)?;
            let captured = capture_native_library_image(&path, &cancellation)?;
            let image = captured.seal(&format!("video-loader-{identity}"), &cancellation)?;
            paths.insert(identity.to_owned(), image.loader_path().to_path_buf());
            sonames.insert(identity.to_owned(), soname.to_owned());
            needed.insert(identity.to_owned(), dynamic.needed().clone());
            retained.push(image);
        }
        let package_sonames = sonames.values().cloned().collect::<BTreeSet<_>>();
        let system_libraries = needed
            .values()
            .flat_map(BTreeSet::iter)
            .filter(|name| !package_sonames.contains(*name))
            .cloned()
            .collect();
        Ok(LoaderFixture {
            _directory: directory,
            _retained: retained,
            projection: VideoCodecLoadProjection {
                paths,
                sonames,
                needed,
                system_libraries,
                dependency_first_order: vec!["dependency".to_owned(), "consumer".to_owned()],
            },
        })
    }

    fn reset_close_order() -> Result<(), Box<dyn std::error::Error>> {
        TEST_CLOSE_ORDER
            .lock()
            .map_err(|_| "video codec close-order mutex was poisoned")?
            .clear();
        Ok(())
    }

    fn close_order() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(TEST_CLOSE_ORDER
            .lock()
            .map_err(|_| "video codec close-order mutex was poisoned")?
            .clone())
    }

    #[test]
    fn retained_video_codec_loader_uses_one_isolated_exact_namespace()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = fixture()?;
        let loaded =
            load_video_codec_projection(&fixture.projection, &CancellationToken::default())?;
        assert_eq!(loaded.libraries.len(), 2);
        assert_eq!(loaded.libraries[0].identity, "dependency");
        assert_eq!(loaded.libraries[1].identity, "consumer");
        assert_eq!(loaded.libraries[0].namespace, loaded.libraries[1].namespace);
        drop(loaded);
        assert_eq!(close_order()?, ["consumer", "dependency"]);
        Ok(())
    }

    #[test]
    fn retained_video_codec_loader_rolls_back_binding_failure_in_reverse_order()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let mut fixture = fixture()?;
        fixture
            .projection
            .needed
            .get_mut("consumer")
            .ok_or("fixture consumer dependency set is missing")?
            .clear();
        assert!(matches!(
            load_video_codec_projection(&fixture.projection, &CancellationToken::default(),),
            Err(NativeVideoCodecLoadError::BindingProof(_))
        ));
        assert_eq!(close_order()?, ["consumer", "dependency"]);
        Ok(())
    }

    #[test]
    fn retained_video_codec_loader_discards_late_cancellation_and_retries_cleanly()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = fixture()?;
        let cancellation = CancellationToken::default();
        let mut checks = 0;
        assert!(matches!(
            load_video_codec_projection_with_check(&fixture.projection, || {
                checks += 1;
                if checks == 5 {
                    cancellation.cancel();
                }
                cancellation.check()
            }),
            Err(NativeVideoCodecLoadError::Cancelled)
        ));
        assert_eq!(close_order()?, ["consumer", "dependency"]);

        reset_close_order()?;
        let loaded =
            load_video_codec_projection(&fixture.projection, &CancellationToken::default())?;
        drop(loaded);
        assert_eq!(close_order()?, ["consumer", "dependency"]);
        Ok(())
    }
}
