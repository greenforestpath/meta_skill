//! Skill bundler for packaging and distribution

pub mod blob;
pub mod github;
pub mod install;
pub mod local_safety;
pub mod manifest;
pub mod package;
pub mod registry;

pub use blob::BlobStore;
pub use install::{install, install_with_options, InstallOptions, InstallReport};
pub use registry::{BundleRegistry, InstallSource, InstalledBundle, ParsedSource};
pub use local_safety::{
    detect_conflicts, detect_modifications, hash_directory, hash_file, ConflictDetail,
    ConflictStrategy, FileStatus, ModificationStatus, ModificationSummary, ResolutionResult,
    SkillModificationReport,
};
pub use manifest::{
    BundleDependency, BundleInfo, BundleManifest, BundleSignature, BundledSkill, Ed25519Signer,
    Ed25519Verifier, SignatureVerifier,
};
pub use package::{missing_blobs, Bundle, BundleBlob, BundlePackage};
