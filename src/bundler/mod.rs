//! Skill bundler for packaging and distribution

pub mod blob;
pub mod github;
pub mod install;
pub mod manifest;
pub mod package;

pub use blob::BlobStore;
pub use install::{install, install_with_options, InstallOptions, InstallReport};
pub use manifest::{
    BundleDependency, BundleInfo, BundleManifest, BundleSignature, BundledSkill, Ed25519Verifier,
    SignatureVerifier,
};
pub use package::{missing_blobs, Bundle, BundleBlob, BundlePackage};
