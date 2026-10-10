use std::{fs, io, path::Path};

pub(crate) fn profile_cache_size(profile: &Path) -> io::Result<u64> {
    let cache = profile.join("Cache");
    let metadata = match fs::symlink_metadata(&cache) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    if is_link(&metadata) || !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid browser cache directory",
        ));
    }
    let started = std::time::Instant::now();
    let mut directories = vec![cache];
    let mut bytes = 0_u64;
    while let Some(directory) = directories.pop() {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        for entry in entries {
            if started.elapsed() > std::time::Duration::from_secs(30) {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Browser cache measurement timed out",
                ));
            }
            let path = entry?.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            if is_link(&metadata) {
                continue;
            }
            if metadata.is_dir() {
                directories.push(path);
            } else if metadata.is_file() {
                bytes = bytes.checked_add(metadata.len()).ok_or_else(|| {
                    io::Error::other("Browser cache size exceeded the byte limit")
                })?;
            }
        }
    }
    Ok(bytes)
}

fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    struct Profile(PathBuf);

    impl Profile {
        fn new() -> Self {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "threadline-cache-size-{}-{timestamp}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("isolated test profile");
            Self(path)
        }
    }

    impl Drop for Profile {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove isolated test profile");
        }
    }

    #[test]
    fn counts_nested_resource_cache_files_without_saved_data() {
        let profile = Profile::new();
        fs::create_dir_all(profile.0.join("Cache/Cache_Data")).unwrap();
        fs::create_dir_all(profile.0.join("Local Storage")).unwrap();
        fs::write(profile.0.join("Cache/index"), [0; 8]).unwrap();
        fs::write(profile.0.join("Cache/Cache_Data/resource"), [0; 37]).unwrap();
        fs::write(profile.0.join("Cookies"), [0; 500]).unwrap();
        fs::write(profile.0.join("Local Storage/preferences"), [0; 900]).unwrap();
        assert_eq!(profile_cache_size(&profile.0).unwrap(), 45);
    }

    #[test]
    fn missing_and_empty_cache_are_zero() {
        let profile = Profile::new();
        assert_eq!(profile_cache_size(&profile.0).unwrap(), 0);
        fs::create_dir(profile.0.join("Cache")).unwrap();
        assert_eq!(profile_cache_size(&profile.0).unwrap(), 0);
    }

    #[test]
    fn invalid_cache_directory_returns_an_error() {
        let profile = Profile::new();
        fs::write(profile.0.join("Cache"), [0; 5]).unwrap();
        assert!(profile_cache_size(&profile.0).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn links_inside_cache_are_not_followed() {
        let profile = Profile::new();
        fs::create_dir(profile.0.join("Cache")).unwrap();
        fs::write(profile.0.join("Cookies"), [0; 500]).unwrap();
        fs::write(profile.0.join("Cache/resource"), [0; 37]).unwrap();
        std::os::unix::fs::symlink(profile.0.join("Cookies"), profile.0.join("Cache/link"))
            .unwrap();
        std::os::unix::fs::symlink(&profile.0, profile.0.join("Cache/loop")).unwrap();
        assert_eq!(profile_cache_size(&profile.0).unwrap(), 37);
    }
}
