//! CLI command implementations
//!
//! Each subcommand has its own module with:
//! - Args struct for command-line arguments
//! - run() function to execute the command

use crate::app::AppContext;
use crate::cli::Commands;
use crate::error::Result;

pub mod alias;
pub mod build;
pub mod bundle;
pub mod config;
pub mod diff;
pub mod doctor;
pub mod edit;
pub mod fmt;
pub mod index;
pub mod init;
pub mod list;
pub mod load;
pub mod prune;
pub mod requirements;
pub mod search;
pub mod security;
pub mod show;
pub mod suggest;
pub mod test;
pub mod update;

/// Dispatch a command to its handler
pub fn run(ctx: &AppContext, command: &Commands) -> Result<()> {
    match command {
        Commands::Init(args) => init::run(ctx, args),
        Commands::Index(args) => index::run(ctx, args),
        Commands::Search(args) => search::run(ctx, args),
        Commands::Load(args) => load::run(ctx, args),
        Commands::Suggest(args) => suggest::run(ctx, args),
        Commands::Show(args) => show::run(ctx, args),
        Commands::List(args) => list::run(ctx, args),
        Commands::Edit(args) => edit::run(ctx, args),
        Commands::Fmt(args) => fmt::run(ctx, args),
        Commands::Diff(args) => diff::run(ctx, args),
        Commands::Alias(args) => alias::run(ctx, args),
        Commands::Requirements(args) => requirements::run(ctx, args),
        Commands::Build(args) => build::run(ctx, args),
        Commands::Bundle(args) => bundle::run(ctx, args),
        Commands::Update(args) => update::run(ctx, args),
        Commands::Doctor(args) => doctor::run(ctx, args),
        Commands::Prune(args) => prune::run(ctx, args),
        Commands::Config(args) => config::run(ctx, args),
        Commands::Security(args) => security::run(ctx, args),
        Commands::Test(args) => test::run(ctx, args),
    }
}
