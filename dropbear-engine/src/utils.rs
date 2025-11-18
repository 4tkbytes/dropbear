//! Utilities and helper functions for the dropbear renderer.

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::Path;

pub const EUCA_SCHEME: &str = "euca://";

/// Converts any supported resource reference into the canonical `euca://` form.
///
/// The function trims whitespace, normalizes path separators, ensures the scheme
/// prefix is present, and collapses redundant separators. Legacy strings without
/// the scheme are accepted and automatically upgraded to the canonical form.
///
/// # Examples
/// ```ignore
/// let canonical = dropbear_engine::utils::canonicalize_euca_uri("textures/diffuse.png").unwrap();
/// assert_eq!(canonical, "euca://textures/diffuse.png");
/// ```
pub fn canonicalize_euca_uri(uri: &str) -> anyhow::Result<String> {
    let trimmed = uri.trim();
    if trimmed.is_empty() {
        anyhow::bail!("euca URI cannot be empty");
    }

    let normalized = trimmed.replace('\\', "/");
    let (had_scheme, without_scheme) = if let Some(rest) = normalized.strip_prefix(EUCA_SCHEME) {
        (true, rest)
    } else {
        (false, normalized.as_str())
    };

    let stripped = without_scheme.trim_start_matches('/');
    if stripped.is_empty() {
        anyhow::bail!("euca URI '{}' must contain a resource path", uri);
    }

    let clean = stripped
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    if clean.is_empty() {
        anyhow::bail!("euca URI '{}' must contain a resource path", uri);
    }

    if !had_scheme {
        log::debug!(
            "Canonicalized legacy resource reference '{}' to '{}{}'",
            uri,
            EUCA_SCHEME,
            clean
        );
    }

    Ok(format!("{EUCA_SCHEME}{clean}"))
}

pub fn relative_path_from_euca<'a>(uri: &'a str) -> anyhow::Result<&'a str> {
    let without_scheme = uri.strip_prefix(EUCA_SCHEME).unwrap_or(uri);

    let stripped = without_scheme.trim_start_matches('/');
    if stripped.is_empty() {
        anyhow::bail!("euca URI '{}' must contain a resource path", uri);
    }

    Ok(stripped)
}

/// An enum that contains the different types that a resource reference can possibly be.
///
/// # Example
/// ```rust
/// use dropbear_engine::utils::{ResourceReferenceType, ResourceReference};
///
/// let resource_ref = ResourceReference::from_reference(
///     ResourceReferenceType::File("euca://models/cube.obj".to_string())
/// );
/// assert_eq!(resource_ref.as_path().unwrap(), "models/cube.obj");
/// ```
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, bincode::Decode, bincode::Encode,
)]
pub enum ResourceReferenceType {
    /// The default type; Specifies there being no resource reference type.
    /// Typically creates errors, so watch out!
    None,

    /// A file type. The [`String`] is the reference from the project or the runtime executable.
    File(String),

    /// The content in bytes. Sometimes, there is a model that is loaded into memory through the
    /// [`include_bytes!`] macro, this type stores it.
    Bytes(Vec<u8>),

    /// A simple plane. Some of the types in [`ResourceReferenceType`] can be simple, just as a signal
    /// to load that entity as a plane or another type.
    ///
    /// In specifics, the plane (as from [`crate::procedural::plane::PlaneBuilder`]) is a model that
    /// has meshes and a textured material, but is created "in house" (during runtime instead of loaded).
    Plane,
    
    /// A cube.
    Cube,
}

impl Default for ResourceReferenceType {
    fn default() -> Self {
        Self::None
    }
}

/// A struct used to "point" to the resource relative to
/// the executable directory or the project directory.
///
/// # Example
/// `/home/tk/project/resources/models/cube.obj` is the file path to `cube.obj`.
///
/// The resource reference will be `models/cube.obj`.
///
/// - If ran in the editor, it translates to `/home/tk/project/resources/models/cube.obj`.
///
/// - In the runtime (with redback-runtime), it
///   translates to `/home/tk/Downloads/Maze/resources/models/cube.obj`
///   _(assuming the executable is at `/home/tk/Downloads/Maze/maze_runner.exe`)_.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceReference {
    pub ref_type: ResourceReferenceType,
}

impl Display for ResourceReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.ref_type)
    }
}

impl ResourceReference {
    /// Creates an empty `ResourceReference` struct.
    pub fn new() -> Self {
        Self {
            ref_type: ResourceReferenceType::None,
        }
    }

    /// Creates a new `ResourceReference` from bytes
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Self {
        Self {
            ref_type: ResourceReferenceType::Bytes(bytes.as_ref().to_vec()),
        }
    }

    pub fn from_reference(ref_type: ResourceReferenceType) -> Self {
        match ref_type {
            ResourceReferenceType::File(reference) => {
                let canonical = canonicalize_euca_uri(&reference)
                    .unwrap_or_else(|err| panic!("Invalid euca URI '{}': {}", reference, err));
                Self {
                    ref_type: ResourceReferenceType::File(canonical),
                }
            }
            other => Self { ref_type: other },
        }
    }

    /// Creates a [`ResourceReference`] directly from an euca URI (e.g. `euca://models/cube.glb`).
    pub fn from_euca_uri(uri: impl AsRef<str>) -> anyhow::Result<Self> {
        let canonical = canonicalize_euca_uri(uri.as_ref())?;
        Ok(Self {
            ref_type: ResourceReferenceType::File(canonical),
        })
    }

    /// Returns the canonical euca URI for this reference if it points to a file asset.
    pub fn as_uri(&self) -> Option<&str> {
        match &self.ref_type {
            ResourceReferenceType::File(reference) => Some(reference.as_str()),
            _ => None,
        }
    }

    /// Returns the resource path relative to the `resources/` directory if this reference represents a file.
    pub fn relative_path(&self) -> Option<&str> {
        match &self.ref_type {
            ResourceReferenceType::File(reference) => relative_path_from_euca(reference).ok(),
            _ => None,
        }
    }

    /// Converts an euca URI string into a path relative to the `resources/` directory.
    pub fn relative_path_from_uri<'a>(uri: &'a str) -> anyhow::Result<&'a str> {
        relative_path_from_euca(uri)
    }

    /// Creates a `ResourceReference` from a full path by extracting the part after "resources/".
    ///
    /// # Examples
    /// ```
    /// use dropbear_engine::utils::ResourceReference;
    ///
    /// let path = "/home/tk/project/resources/models/cube.obj";
    /// let resource_ref = ResourceReference::from_path(path).unwrap();
    /// assert_eq!(resource_ref.as_path().unwrap(), "models/cube.obj");
    /// ```
    ///
    /// Returns `None` if the path doesn't contain "resources" or if the path after resources is empty.
    pub fn from_path(full_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = full_path.as_ref();

        let components: Vec<_> = path.components().collect();

        for (i, component) in components.iter().enumerate() {
            if let std::path::Component::Normal(name) = component
                && *name == "resources"
            {
                let remaining_components = &components[i + 1..];
                if remaining_components.is_empty() {
                    anyhow::bail!("Unable to locate any remaining components");
                }

                let resource_path = remaining_components
                    .iter()
                    .map(|c| match c {
                        std::path::Component::Normal(name) => name.to_str().unwrap_or(""),
                        _ => "",
                    })
                    .collect::<Vec<_>>()
                    .join("/");

                let canonical = canonicalize_euca_uri(&format!("{EUCA_SCHEME}{resource_path}"))?;

                return Ok(Self {
                    ref_type: ResourceReferenceType::File(canonical),
                });
            }
        }

        anyhow::bail!("Nothing here")
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.ref_type {
            ResourceReferenceType::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }
}

/// Neat lil macro to create a resource reference easier
#[macro_export]
macro_rules! resource {
    ($path:expr) => {
        ::dropbear_engine::utils::ResourceReference::from_euca_uri($path).expect("Invalid euca URI")
    };
}
