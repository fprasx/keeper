use std::{
    collections::BTreeMap,
    env,
    fmt::Display,
    fs::{self, DirBuilder, File},
    path::Path,
    process::{self, Stdio},
};

use keeper::{
    cli::{Command, ShowSet},
    color::{GREEN, RED, RESET},
};

use anyhow::{anyhow, Context};
use chrono::{Days, Local, NaiveDate};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    completed: bool,
    desc: String,
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.completed {
            write!(f, "{GREEN}({RESET}{}{GREEN}){RESET}", self.desc)
        } else {
            write!(f, "{RED}({RESET}{}{RED}){RESET}", self.desc)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Schedule {
    timeslots: BTreeMap<usize, Vec<Task>>,
}

impl Display for Schedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut print_newline = false; // To avoid printing an ending newline
        for (time, tasklist) in self.timeslots.iter() {
            if print_newline {
                writeln!(f)?;
            }
            print_newline = true;

            let remaining = tasklist.iter().any(|t| !t.completed);

            if remaining {
                write!(f, "{RED}[{time}]{RESET}")?;
            } else {
                write!(f, "{GREEN}[{time}]{RESET}")?;
            }

            for task in tasklist {
                write!(f, " {task}")?;
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Keeper {
    days: BTreeMap<NaiveDate, Schedule>,
}

fn main() -> anyhow::Result<()> {
    let mut keeper = load_data()?;

    let args = env::args();
    let command = Command::parse(args);
    match command {
        Command::Add { date, desc, hour } => {
            keeper
                .days
                .entry(date)
                .or_insert_with(Schedule::default)
                .timeslots
                .entry(hour)
                .or_insert_with(Default::default)
                .push(Task {
                    completed: false,
                    desc,
                });
        }
        Command::Mark { date, hour, index } => {
            if let Some(task) = keeper
                .days
                .get_mut(&date)
                .map(|schedule| &mut schedule.timeslots)
                .and_then(|slot| slot.get_mut(&hour))
                .and_then(|hour| hour.get_mut(index))
            {
                task.completed = true;
            }
        }
        Command::Show { set } => {
            match set {
                ShowSet::Days(days) => {
                    let mut print_newline = false; // to avoid printing an ending newline
                    for date in Local::now().date_naive().iter_days().take(days) {
                        if print_newline {
                            println!();
                        }
                        print_newline = true;

                        println!("{}", date.format("%d %b %Y"));
                        if let Some(schedule) = keeper.days.get(&date) {
                            println!("{schedule}");
                        } else {
                            println!("Empty");
                        }
                    }
                }
                ShowSet::Date(date) => {
                    println!("{}", date.format("%d %b %Y"));
                    if let Some(schedule) = keeper.days.get(&date) {
                        println!("{schedule}");
                    } else {
                        println!("Empty");
                    }
                }
            }
        }
    }

    commit_data(&keeper)?;

    Ok(())
}
