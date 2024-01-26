use chrono::{Local, NaiveDate};
use keeper_util::{
    color::{GREEN, RESET, YELLOW},
    current_version, error, fatal, parse_date,
};
use std::{env::Args, process};

#[derive(Debug, Clone, Copy)]
pub enum ShowSet {
    Days(usize),
    Date(NaiveDate),
}

#[derive(Debug)]
pub enum Command {
    Add {
        date: NaiveDate,
        hour: usize,
        desc: String,
    },
    Mark {
        date: NaiveDate,
        hour: usize,
        index: usize,
    },
    Change {
        date: NaiveDate,
        old_hour: usize,
        index: usize,
        new_hour: usize,
    },
    Show {
        set: ShowSet,
    },
    Render {
        set: ShowSet,
    },
}

pub fn help() -> ! {
    let version = current_version();
    // These colors are just too much fun
    println!(
        "\
keeper-todo ({version}) Felix Prasanna 2024
{YELLOW}help{RESET}:
    keeper-todo help
{YELLOW}add{RESET}:
    keeper-todo add {GREEN}date{RESET} hour desc
{YELLOW}mark{RESET}:
    keeper-todo mark {GREEN}date{RESET} hour.index
    keeper-todo mark {GREEN}date{RESET} hour
{YELLOW}change{RESET}:
    keeper-todo change {GREEN}date{RESET} hour.index new-hour
    keeper-todo change {GREEN}date{RESET} hour new-hour
{YELLOW}show{RESET}:
    keeper-todo show {GREEN}date{RESET}
    keeper-todo show {GREEN}count{RESET}
    keeper-todo show
{YELLOW}render{RESET}:
    keeper-todo render {GREEN}date{RESET} path
    keeper-todo render {GREEN}count{RESET} path

{YELLOW}terms{RESET}:
    date = {GREEN}(dd-mm-yy|today|tomorrow|yesterday){RESET}"
    );
    process::exit(0);
}

impl Command {
    pub fn parse(args: Args) -> Self {
        // First arg is program itself
        let mut args = args.skip(1);

        let Some(command) = args.next() else { help() };
        match command.as_str() {
            "add" => {
                let Some(date) = args.next() else {
                    fatal!("no date provided to add");
                };
                let date = parse_date(&date);
                let hour = match args.next() {
                    Some(hour) => {
                        let Ok(hour) = hour.parse() else {
                            fatal!("failed to parse hour");
                        };
                        if !(0..24).contains(&hour) {
                            fatal!("hour [{}] is not in 0..24", hour);
                        }
                        hour
                    }
                    None => fatal!("no hour provided to add"),
                };
                let Some(desc) = args.next() else {
                    fatal!("no desc provided to add");
                };
                Command::Add { date, hour, desc }
            }
            "mark" => {
                let Some(date) = args.next() else {
                    fatal!("no date provided to add");
                };
                let date = parse_date(&date);

                let Some(id) = args.next() else {
                    error!("no id provided to mark");
                    fatal!("expecting format [hour.index] or [hour]");
                };
                match id.split_once('.') {
                    Some((hour, index)) => {
                        let Ok(hour) = hour.parse() else {
                            fatal!("failed to parse hour from format [hour.index]");
                        };
                        let Ok(index) = index.parse() else {
                            fatal!("failed to parse index from format [hour.index]");
                        };
                        if !(0..24).contains(&hour) {
                            fatal!("hour [{}] is not in 0..24", hour);
                        }
                        Command::Mark { date, hour, index }
                    }
                    None => {
                        let Ok(hour) = id.parse() else {
                            fatal!("failed to parse hour from format [hour]");
                        };
                        if !(0..24).contains(&hour) {
                            fatal!("hour [{}] is not in 0..24", hour);
                        }
                        Command::Mark {
                            date,
                            hour,
                            index: 0,
                        }
                    }
                }
            }
            "change" => {
                let Some(date) = args.next() else {
                    fatal!("no date provided to change");
                };
                let date = parse_date(&date);

                let Some(id) = args.next() else {
                    error!("no id provided to change");
                    fatal!("expecting format [hour.index] or [hour]");
                };
                let (old_hour, index) = match id.split_once('.') {
                    Some((hour, index)) => {
                        let Ok(hour) = hour.parse() else {
                            fatal!("failed to parse hour from format [hour.index]");
                        };
                        let Ok(index) = index.parse() else {
                            fatal!("failed to parse index from format [hour.index]");
                        };
                        if !(0..24).contains(&hour) {
                            fatal!("hour [{}] is not in 0..24", hour);
                        }
                        (hour, index)
                    }
                    None => {
                        let Ok(hour) = id.parse() else {
                            fatal!("failed to parse hour from format [hour]");
                        };
                        if !(0..24).contains(&hour) {
                            fatal!("hour [{}] is not in 0..24", hour);
                        }
                        (hour, 0)
                    }
                };
                let Some(new_hour) = args.next() else {
                    fatal!("no new-hour provided to change");
                };
                let Ok(new_hour) = new_hour.parse() else {
                    fatal!("failed to parse new-hour from format [new-hour]");
                };
                if !(0..24).contains(&new_hour) {
                    fatal!("new-hour [{}] is not in 0..24", new_hour);
                }
                Self::Change {
                    date,
                    old_hour,
                    index,
                    new_hour,
                }
            }
            "show" => {
                // if no argument provided interpret as today
                let Some(set) = args.next() else {
                    return Self::Show {
                        set: ShowSet::Date(Local::now().date_naive()),
                    };
                };

                if let Ok(days) = set.parse() {
                    // First try to parse as number
                    Self::Show {
                        set: ShowSet::Days(days),
                    }
                } else {
                    // Then try to parse as date
                    let date = parse_date(&set);
                    Self::Show {
                        set: ShowSet::Date(date),
                    }
                }
            }
            "render" => {
                // if no argument provided interpret as today
                let Some(set) = args.next() else {
                    fatal!("no date/count provided to render");
                };

                let set = if let Ok(days) = set.parse() {
                    // First try to parse as number
                    ShowSet::Days(days)
                } else {
                    // Then try to parse as date
                    ShowSet::Date(parse_date(&set))
                };

                Self::Render { set }
            }
            _ => help(),
        }
    }
}
