//! Meta-skills (composed slice bundles).

pub mod manager;
pub mod parser;
pub mod registry;
pub mod types;

pub use manager::{
    ConditionContext, MetaSkillLoadResult, MetaSkillManager, ResolvedSlice, SkipReason,
    SkippedSlice,
};
pub use parser::MetaSkillParser;
pub use registry::{MetaSkillQuery, MetaSkillRegistry, MetaSkillRegistryStats};
pub use types::{
    MetaDisclosureLevel, MetaSkill, MetaSkillDoc, MetaSkillHeader, MetaSkillMetadata,
    MetaSkillSliceRef, PinStrategy, SliceCondition,
};
