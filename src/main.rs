use std::{
    env,
    fs::{self, DirBuilder, File},
    path::Path,
    process::{self, Stdio},
};

use keeper::{cli::Command, data::Keeper};

use anyhow::{anyhow, Context};

const DATA_DIR: &str = concat!(env!("HOME"), "/.config/keeper/");
const DATA_PATH: &str = concat!(env!("HOME"), "/.config/keeper/data.ron");

fn init_storage() -> anyhow::Result<()> {
    // check for DATA_DIR
    if !Path::new(DATA_DIR).exists() {
        DirBuilder::new()
            .recursive(true)
            .create(DATA_DIR)
            .with_context(|| format!("failed to create data directory at {}", DATA_DIR))?;
    }

    // check for DATA_PATH
    if !Path::new(DATA_PATH).exists() {
        File::create(DATA_PATH)
            .with_context(|| format!("failed to create data file at {}", DATA_PATH))?;

        // Init file to empty keeper
        let keeper = Keeper::default();
        let ron = ron::ser::to_string_pretty(&keeper, Default::default())
            .context("failed to serialize RON")?;

        fs::write(DATA_PATH, ron)
            .with_context(|| format!("failed to write RON back to {DATA_PATH}"))?;
    }

    // init git repo fi need be
    if process::Command::new("git")
        .args(["-C", DATA_DIR, "status"])
        .stdout(Stdio::null())
        .status()
        .context("problem starting process for git status")?
        .code()
        .ok_or_else(|| anyhow!("git status returned no exit code (terminated by signal)"))?
        != 0
    {
        process::Command::new("git")
            .args(["-C", DATA_DIR, "init"])
            .status()
            .with_context(|| format!("failed to run git init in {DATA_DIR}"))?;
    }

    Ok(())
}

/// Load current keeper.
fn load_data() -> anyhow::Result<Keeper> {
    init_storage().context("failed to load storage")?;

    let contents = fs::read_to_string(DATA_PATH)
        .with_context(|| format!("failed to read from {DATA_PATH}"))?;

    ron::from_str(&contents).context("failed to deserialize RON")
}

/// Commit a new keeper.
fn commit_data(data: &Keeper) -> anyhow::Result<()> {
    init_storage().context("failed to load storage")?;

    let ron =
        ron::ser::to_string_pretty(data, Default::default()).context("failed to serialize RON")?;

    fs::write(DATA_PATH, ron)
        .with_context(|| format!("failed to write RON back to {DATA_PATH}"))?;

    process::Command::new("git")
        .stdout(Stdio::null())
        .args(["-C", DATA_DIR, "add", DATA_PATH])
        .status()
        .context("failed to run git add")?;

    process::Command::new("git")
        .stdout(Stdio::null())
        .args(["-C", DATA_DIR, "commit", "-m", "--allow-empty-message"])
        .status()
        .context("failed to run git add")?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut keeper = load_data()?;

    let args = env::args();
    let command = Command::parse(args);
    match command {
        Command::Add { date, desc, hour } => {
            keeper.add(date, desc, hour);
        }
        Command::Mark { date, hour, index } => {
            keeper.mark(date, hour, index);
        }
        Command::Show { set } => {
            keeper.show(set);
        }
    }

    keeper.order();
    commit_data(&keeper)?;

    Ok(())
}
