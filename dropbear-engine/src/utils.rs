//! Utilities and helper functions for the dropbear renderer.

use std::fmt::{Display, Formatter};
use std::{fs, io};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

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
    resource_ref_path: String,
    bytes: Vec<u8>
}

impl Display for ResourceReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resource_ref_path)
    }
}

impl ResourceReference {
    /// Creates a new `ResourceReference` struct.
    pub fn new(resource_path: impl Into<String>) -> Self {
        Self {
            resource_ref_path: resource_path.into(),
            bytes: Vec::new()
        }
    }
    
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            resource_ref_path: String::from(""),
            bytes
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
    /// assert_eq!(resource_ref.as_str(), "models/cube.obj");
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
                        resource_ref_path: resource_path,
                        bytes: Vec::new()
                    });
                }
            }
        }

        None
    }

    /// Get the raw resource reference path
    pub fn as_str(&self) -> &str {
        self.resource_ref_path.as_str()
    }

    /// Creates a PathBuf relative to the given project path.
    ///
    /// If the project_path has a parent directory, it uses that parent + "resources" + resource_path.
    /// Otherwise, it uses project_path + resource_path directly.
    pub fn to_project_path(&self, project_path: impl AsRef<Path>) -> PathBuf {
        let path = project_path.as_ref();
        log::debug!("Parent path: {}", path.display());
        path.join("resources").join(self.resource_ref_path.as_str())
    }

    /// Creates a PathBuf that points to the resource relative to the executable directory.
    ///
    /// Returns an error if the executable path cannot be determined.
    pub fn to_executable_path(&self) -> anyhow::Result<PathBuf> {
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not get executable directory"))?;
        Ok(exe_dir.join("resources").join(self.resource_ref_path.as_str()))
    }

    /// Creates a PathBuf that points to the resource, with fallback logic.
    ///
    /// First tries to resolve relative to executable, then falls back to current directory + resources.
    pub fn resolve_path(&self) -> PathBuf {
        if let Ok(exe_path) = self.to_executable_path() {
            if exe_path.exists() {
                return exe_path;
            }
        }

        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("resources")
            .join(self.resource_ref_path.as_str())
    }

    /// Check if the resource exists when resolved via executable path
    pub fn exists_in_executable_dir(&self) -> bool {
        self.to_executable_path()
            .map(|path| path.exists())
            .unwrap_or(false)
    }

    /// Check if the resource exists when resolved via project path
    pub fn exists_in_project_dir(&self, project_path: impl AsRef<Path>) -> bool {
        self.to_project_path(project_path).exists()
    }

    /// Check if the resource exists using the fallback resolution logic
    pub fn exists(&self) -> bool {
        self.resolve_path().exists()
    }

    /// Read the resource as a string using fallback resolution
    pub fn read_to_string(&self) -> io::Result<String> {
        fs::read_to_string(self.resolve_path())
    }

    /// Read the resource as bytes using fallback resolution
    pub fn read_to_bytes(&self) -> io::Result<Vec<u8>> {
        fs::read(self.resolve_path())
    }

    /// Read from a specific resolved path (project or executable)
    pub fn read_to_string_from_project(&self, project_path: impl AsRef<Path>) -> io::Result<String> {
        fs::read_to_string(self.to_project_path(project_path))
    }

    /// Read from the executable directory
    pub fn read_to_string_from_executable(&self) -> anyhow::Result<String> {
        let path = self.to_executable_path()?;
        Ok(fs::read_to_string(path)?)
    }

    /// Get the file name of the resource
    pub fn file_name(&self) -> Option<&str> {
        Path::new(self.resource_ref_path.as_str()).file_name()?.to_str()
    }

    /// Get the file extension of the resource
    pub fn extension(&self) -> Option<&str> {
        Path::new(self.resource_ref_path.as_str()).extension()?.to_str()
    }
}

/// Neat lil macro to create a resource reference easier
#[macro_export]
macro_rules! resource {
    ($path:literal) => {
        ResourceReference::new($path)
    };
}