use std::{env, path::Path};

use anyhow::Context;
use keeper_todo::{cli::Command, data::Keeper};
use keeper_util::DataManager;

const DATA_PATH: &str = concat!(env!("HOME"), "/.local/share/keeper/data.ron");

fn main() -> anyhow::Result<()> {
    let dm = DataManager::<Keeper>::new(Path::new(DATA_PATH))?;
    let mut keeper = dm.load_data()?;

    let args = env::args();
    let command = Command::parse(args);
    match command {
        Command::Add {
            date,
            ref desc,
            hour,
        } => {
            keeper.add(date, desc, hour).context("add command failed")?;
        }
        Command::Mark { date, hour, index } => {
            keeper
                .mark(date, hour, index)
                .context("mark command failed")?;
        }
        Command::Change {
            date,
            old_hour,
            index,
            new_hour,
        } => {
            keeper
                .change(date, old_hour, index, new_hour)
                .context("change command failed")?;
        }
        Command::Show { set } => {
            keeper.show(set);
        }
        Command::Render { set } => {
            keeper.render(set).context("render command failed")?;
        }
    }

    keeper.order();
    dm.commit_data(&keeper, &format!("{command:?}"))?;

    Ok(())
}
