use std::{env, path::Path};

use keeper_todo::{cli::Command, data::Keeper};
use keeper_util::DataManager;

const DATA_PATH: &str = concat!(env!("HOME"), "/.config/keeper/data.ron");

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
            keeper.add(date, desc, hour);
        }
        Command::Mark { date, hour, index } => {
            keeper.mark(date, hour, index);
        }
        Command::Show { set } => {
            keeper.show(set);
        }
        Command::Render { set, ref path } => {
            keeper.render(set, path);
        }
    }

    keeper.order();
    dm.commit_data(&keeper, &format!("{command:?}"))?;

    Ok(())
}
