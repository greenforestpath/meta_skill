//! Meta-skills (composed slice bundles).

pub mod parser;
pub mod registry;
pub mod types;

pub use parser::MetaSkillParser;
pub use registry::{MetaSkillQuery, MetaSkillRegistry, MetaSkillRegistryStats};
pub use types::{
    MetaDisclosureLevel, MetaSkill, MetaSkillDoc, MetaSkillHeader, MetaSkillMetadata,
    MetaSkillSliceRef, PinStrategy, SliceCondition,
};
