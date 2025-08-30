//! Utilities and helper functions for the dropbear renderer.

use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// An enum that contains the different types that a resource reference can possibly be.
///
/// # Example
/// ```rust
/// use dropbear_engine::utils::{ResourceReferenceType, ResourceReference};
///
/// let resource_ref = ResourceReference::from_reference(
///     ResourceReferenceType::File("models/cube.obj".to_string()
/// ));
/// assert_eq!(resource_ref.as_path().unwrap(), "models/cube.obj");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// In specifics, the plane (as from [`crate::starter::plane::PlaneBuilder`]) is a model that
    /// has meshes and a textured material, but is created "in house" (during runtime instead of loaded).
    Plane,
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
/// - If ran in the editor, it translates to
/// `/home/tk/project/resources/models/cube.obj`.
///
/// - In the runtime (with redback-runtime), it
/// translates to `/home/tk/Downloads/Maze/resources/models/cube.obj`
/// _(assuming the executable is at `/home/tk/Downloads/Maze/maze_runner.exe`)_.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceReference {
    pub ref_type: ResourceReferenceType
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
            ref_type: ResourceReferenceType::None
        }
    }
    
    /// Creates a new `ResourceReference` from bytes
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            ref_type: ResourceReferenceType::Bytes(bytes)
        }
    }

    pub fn from_reference(ref_type: ResourceReferenceType) -> Self {
        Self {
            ref_type
        }
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
    pub fn from_path(full_path: impl AsRef<Path>) -> Option<Self> {
        let path = full_path.as_ref();

        let components: Vec<_> = path.components().collect();

        for (i, component) in components.iter().enumerate() {
            if let std::path::Component::Normal(name) = component {
                if *name == "resources" {
                    let remaining_components = &components[i + 1..];
                    if remaining_components.is_empty() {
                        return None;
                    }

                    let resource_path = remaining_components
                        .iter()
                        .map(|c| match c {
                            std::path::Component::Normal(name) => name.to_str().unwrap_or(""),
                            _ => "",
                        })
                        .collect::<Vec<_>>()
                        .join("/");

                    return Some(Self {
                        ref_type: ResourceReferenceType::File(resource_path),
                    });
                }
            }
        }

        None
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.ref_type {
            ResourceReferenceType::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn as_path(&self) -> Option<&str> {
        match &self.ref_type {
            ResourceReferenceType::File(path) => Some(path.as_str()),
            _ => None,
        }
    }

    /// Converts a [`ResourceReference`] to an [`Option<PathBuf>`].
    ///
    /// Returns None if the Resource Reference is not a [`ResourceReferenceType::File`]
    pub fn to_project_path(&self, project_path: impl AsRef<Path>) -> Option<PathBuf> {
        let path = project_path.as_ref();
        log::debug!("Parent path: {}", path.display());
        match &self.ref_type {
            ResourceReferenceType::File(reference) => {
                Some(path.join("resources").join(reference.as_str()))
            }
            _ => None,
        }

    }

    /// Creates a PathBuf that points to the resource relative to the executable directory.
    pub fn to_executable_path(&self) -> anyhow::Result<PathBuf> {
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent().ok_or(anyhow::anyhow!("Cannot resolve executable path"))?;
        match &self.ref_type {
            ResourceReferenceType::File(file) => {
                Ok(exe_dir.join("resources").join(file.as_str()))
            }
            _ => Err(anyhow::anyhow!("Cannot resolve executable path")),
        }
    }

    /// Creates a PathBuf that points to the resource, with fallback logic.
    ///
    /// First tries to resolve relative to executable, then falls back to current directory + resources.
    ///
    /// Returns an error of the ResourceReferenceType is not of type [`ResourceReferenceType::File`]
    pub fn resolve_path(&self) -> anyhow::Result<PathBuf> {
        match &self.ref_type {
            ResourceReferenceType::None => {anyhow::bail!("Cannot resolve ResourceReferenceType::None")}
            ResourceReferenceType::Bytes(_) => {anyhow::bail!("Cannot resolve bytes")}
            ResourceReferenceType::File(path) => {
                if let Ok(exe_path) = self.to_executable_path() {
                    if exe_path.exists() {
                        return Ok(exe_path);
                    }
                }

                Ok(std::env::current_dir()?
                    .join("resources")
                    .join(path.as_str()))
            }
            _ => {anyhow::bail!("Cannot resolve ResourceReferenceType::Plane")}
        }

    }
}

/// Neat lil macro to create a resource reference easier
#[macro_export]
macro_rules! resource {
    ($path:literal) => {
        ResourceReference::from_reference(ResourceReferenceType::File($path))
    };
}