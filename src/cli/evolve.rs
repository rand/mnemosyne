//! Memory evolution command (importance recalibration, link decay, archival, consolidation)

use anyhow::Context;
use clap::Subcommand;
use mnemosyne_core::{
    error::Result,
    evolution::{
        ArchivalJob, ConsolidationJob, EvolutionJob, ImportanceRecalibrator, JobConfig,
        LinkDecayJob,
    },
    ConnectionMode, LibsqlStorage,
};
use std::sync::Arc;
use std::time::Duration;

use super::helpers::get_default_db_path;

#[derive(Subcommand)]
pub enum EvolveJob {
    /// Run importance recalibration job
    Importance {
        /// Batch size (max memories to process)
        #[arg(short, long, default_value = "100")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Run link decay job
    Links {
        /// Batch size (max links to process)
        #[arg(short, long, default_value = "100")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Run archival job
    Archival {
        /// Batch size (max memories to archive)
        #[arg(short, long, default_value = "50")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Run consolidation job (detect and merge duplicates)
    Consolidation {
        /// Batch size (max memories to check)
        #[arg(short, long, default_value = "100")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Run all evolution jobs
    All {
        /// Batch size for each job
        #[arg(short, long, default_value = "100")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },
}

/// Handle evolution command
pub async fn handle(job: EvolveJob, global_db_path: Option<String>) -> Result<()> {
    // Determine database path
    let db_path = match &job {
        EvolveJob::Importance { database, .. }
        | EvolveJob::Links { database, .. }
        | EvolveJob::Archival { database, .. }
        | EvolveJob::Consolidation { database, .. }
        | EvolveJob::All { database, .. } => database
            .clone()
            .or_else(|| global_db_path)
            .unwrap_or_else(|| get_default_db_path().to_string_lossy().to_string()),
    };

    // Initialize storage
    let storage = Arc::new(
        LibsqlStorage::new(ConnectionMode::Local(db_path.into()))
            .await
            .context("Failed to initialize storage")?,
    );

    match job {
        EvolveJob::Importance { batch_size, .. } => {
            println!("Running importance recalibration job...");
            let job = ImportanceRecalibrator::new(storage.clone());
            let config = JobConfig {
                enabled: true,
                interval: Duration::from_secs(0),
                batch_size,
                max_duration: Duration::from_secs(300), // 5 minutes
            };

            match job.run(&config).await {
                Ok(report) => {
                    println!(" Importance recalibration complete:");
                    println!("  Memories processed: {}", report.memories_processed);
                    println!("  Changes made: {}", report.changes_made);
                    println!("  Errors: {}", report.errors);
                    println!("  Duration: {:?}", report.duration);
                    Ok(())
                }
                Err(e) => {
                    eprintln!(" Importance recalibration failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        EvolveJob::Links { batch_size, .. } => {
            println!("Running link decay job...");
            let job = LinkDecayJob::new(storage.clone());
            let config = JobConfig {
                enabled: true,
                interval: Duration::from_secs(0),
                batch_size,
                max_duration: Duration::from_secs(300), // 5 minutes
            };

            match job.run(&config).await {
                Ok(report) => {
                    println!(" Link decay complete:");
                    println!("  Links processed: {}", report.memories_processed);
                    println!("  Changes made: {}", report.changes_made);
                    println!("  Errors: {}", report.errors);
                    println!("  Duration: {:?}", report.duration);
                    Ok(())
                }
                Err(e) => {
                    eprintln!(" Link decay failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        EvolveJob::Archival { batch_size, .. } => {
            println!("Running archival job...");
            let job = ArchivalJob::new(storage.clone());
            let config = JobConfig {
                enabled: true,
                interval: Duration::from_secs(0),
                batch_size,
                max_duration: Duration::from_secs(300), // 5 minutes
            };

            match job.run(&config).await {
                Ok(report) => {
                    println!(" Archival complete:");
                    println!("  Memories processed: {}", report.memories_processed);
                    println!("  Changes made: {}", report.changes_made);
                    println!("  Errors: {}", report.errors);
                    println!("  Duration: {:?}", report.duration);
                    Ok(())
                }
                Err(e) => {
                    eprintln!(" Archival failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        EvolveJob::Consolidation { batch_size, .. } => {
            println!("Running consolidation job...");
            let job = ConsolidationJob::new(storage.clone());
            let config = JobConfig {
                enabled: true,
                interval: Duration::from_secs(0),
                batch_size,
                max_duration: Duration::from_secs(300), // 5 minutes
            };

            match job.run(&config).await {
                Ok(report) => {
                    println!(" Consolidation complete:");
                    println!("  Memories processed: {}", report.memories_processed);
                    println!("  Changes made: {}", report.changes_made);
                    println!("  Errors: {}", report.errors);
                    println!("  Duration: {:?}", report.duration);
                    Ok(())
                }
                Err(e) => {
                    eprintln!(" Consolidation failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        EvolveJob::All { batch_size, .. } => {
            println!("Running all evolution jobs...");
            println!();

            let config = JobConfig {
                enabled: true,
                interval: Duration::from_secs(0),
                batch_size,
                max_duration: Duration::from_secs(300), // 5 minutes per job
            };

            // 1. Importance recalibration
            println!("1/3: Importance recalibration...");
            let importance_job = ImportanceRecalibrator::new(storage.clone());
            match importance_job.run(&config).await {
                Ok(report) => {
                    println!(
                        "   {} memories processed, {} updated",
                        report.memories_processed, report.changes_made
                    );
                }
                Err(e) => {
                    eprintln!("   Failed: {}", e);
                }
            }
            println!();

            // 2. Link decay
            println!("2/3: Link decay...");
            let link_job = LinkDecayJob::new(storage.clone());
            match link_job.run(&config).await {
                Ok(report) => {
                    println!(
                        "   {} links processed, {} updated",
                        report.memories_processed, report.changes_made
                    );
                }
                Err(e) => {
                    eprintln!("   Failed: {}", e);
                }
            }
            println!();

            // 3. Archival
            println!("3/3: Archival...");
            let archival_job = ArchivalJob::new(storage.clone());
            match archival_job.run(&config).await {
                Ok(report) => {
                    println!(
                        "   {} memories processed, {} archived",
                        report.memories_processed, report.changes_made
                    );
                }
                Err(e) => {
                    eprintln!("   Failed: {}", e);
                }
            }
            println!();

            println!("All evolution jobs complete!");
            Ok(())
        }
    }
}
