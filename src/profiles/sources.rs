use std::path::{Component, Path, PathBuf};

use crate::utils::paths::{
    get_common_path, get_encrypted_common_path, get_encrypted_profiles_path, get_profiles_path,
};

use super::ActiveProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLayer {
    Common,
    Profile,
}

impl std::fmt::Display for SourceLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceLayer::Common => write!(f, "common"),
            SourceLayer::Profile => write!(f, "profile"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedSource {
    pub path: PathBuf,
    pub layer: SourceLayer,
}

impl ActiveProfile {
    pub fn resolve_source(&self, source_path: &str) -> Option<ResolvedSource> {
        if !is_valid_source_path(source_path) {
            return None;
        }

        let candidates = self.get_candidate_sources(source_path);

        for (path, layer) in candidates {
            if path.exists() {
                return Some(ResolvedSource { path, layer });
            }
        }

        None
    }

    pub fn get_candidate_sources(&self, source_path: &str) -> Vec<(PathBuf, SourceLayer)> {
        if !is_valid_source_path(source_path) {
            return Vec::new();
        }

        let mut candidates = Vec::new();

        if let Some(profile_name) = &self.name {
            candidates.push((
                get_profiles_path(profile_name).join(source_path),
                SourceLayer::Profile,
            ));
        }

        candidates.push((get_common_path().join(source_path), SourceLayer::Common));
        candidates
    }

    pub fn get_all_resolved_sources(&self, source_path: &str) -> Vec<ResolvedSource> {
        self.get_candidate_sources(source_path)
            .into_iter()
            .filter(|(path, _)| path.exists())
            .map(|(path, layer)| ResolvedSource { path, layer })
            .collect()
    }

    pub fn resolve_encrypted_source(&self, source_path: &str) -> Option<ResolvedSource> {
        if !is_valid_source_path(source_path) {
            return None;
        }

        let candidates = self.get_candidate_encrypted_sources(source_path);

        for (path, layer) in candidates {
            if path.exists() {
                return Some(ResolvedSource { path, layer });
            }
        }

        None
    }

    pub fn get_candidate_encrypted_sources(
        &self,
        source_path: &str,
    ) -> Vec<(PathBuf, SourceLayer)> {
        if !is_valid_source_path(source_path) {
            return Vec::new();
        }

        let mut candidates = Vec::new();

        if let Some(profile_name) = &self.name {
            candidates.push((
                get_encrypted_profiles_path(profile_name).join(source_path),
                SourceLayer::Profile,
            ));
        }

        candidates.push((
            get_encrypted_common_path().join(source_path),
            SourceLayer::Common,
        ));

        candidates
    }
}

fn is_valid_source_path(source_path: &str) -> bool {
    if source_path.is_empty() {
        return false;
    }

    let path = Path::new(source_path);
    !path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
}
