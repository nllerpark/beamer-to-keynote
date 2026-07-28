use std::{
    collections::HashSet,
    env,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) const REQUIRED_PATHS: [&str; 7] = [
    "/Library/TeX/texbin",
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/bin",
    "/usr/sbin",
    "/sbin",
];

const GHOSTSCRIPT_LIBRARY_PATHS: [&str; 4] = [
    "/opt/homebrew/opt/ghostscript/lib/libgs.dylib",
    "/usr/local/opt/ghostscript/lib/libgs.dylib",
    "/opt/local/lib/libgs.dylib",
    "/usr/local/lib/libgs.dylib",
];

fn augmented_path(existing: Option<&OsStr>) -> OsString {
    let mut seen = HashSet::new();
    let paths = REQUIRED_PATHS
        .iter()
        .map(PathBuf::from)
        .chain(existing.into_iter().flat_map(env::split_paths))
        .filter(|path| seen.insert(path.clone()))
        .collect::<Vec<_>>();

    env::join_paths(paths).unwrap_or_else(|_| OsString::from(REQUIRED_PATHS.join(":")))
}

pub(crate) fn configure_tex_process(command: &mut Command, home_directory: &Path) {
    command
        .env("PATH", augmented_path(env::var_os("PATH").as_deref()))
        .env("HOME", home_directory);
}

pub(crate) fn system_tool_candidates(name: &str) -> impl Iterator<Item = PathBuf> {
    REQUIRED_PATHS
        .iter()
        .map(PathBuf::from)
        .chain(
            env::var_os("PATH")
                .into_iter()
                .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>()),
        )
        .map(move |directory| directory.join(name))
}

pub(crate) fn ghostscript_library_path() -> Option<PathBuf> {
    env::var_os("LIBGS")
        .map(PathBuf::from)
        .into_iter()
        .chain(GHOSTSCRIPT_LIBRARY_PATHS.into_iter().map(PathBuf::from))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::{GHOSTSCRIPT_LIBRARY_PATHS, REQUIRED_PATHS, augmented_path};
    use std::{env, ffi::OsStr, path::PathBuf};

    #[test]
    fn augments_gui_path_in_stable_order_without_duplicates() {
        let existing = OsStr::new("/custom/bin:/usr/bin:/Library/TeX/texbin");
        let actual = env::split_paths(&augmented_path(Some(existing))).collect::<Vec<_>>();
        let mut expected = REQUIRED_PATHS.iter().map(PathBuf::from).collect::<Vec<_>>();
        expected.push(PathBuf::from("/custom/bin"));

        assert_eq!(actual, expected);
    }

    #[test]
    fn checks_homebrew_ghostscript_before_legacy_locations() {
        assert_eq!(
            GHOSTSCRIPT_LIBRARY_PATHS[0],
            "/opt/homebrew/opt/ghostscript/lib/libgs.dylib"
        );
    }
}
