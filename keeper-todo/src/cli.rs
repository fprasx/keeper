use chrono::{Local, NaiveDate};
use keeper_util::{
    color::{GREEN, RESET, YELLOW},
    error, fatal,
    parse_date, current_version
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
    Show {
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
    keeper-todo add {GREEN}(dd-mm-yy|today|tomorrow){RESET} hour desc
{YELLOW}mark{RESET}:
    keeper-todo mark {GREEN}(dd-mm-yy|today|tomorrow){RESET} {GREEN}(hour.index|hour){RESET}
{YELLOW}show{RESET}:
    keeper-todo show {GREEN}(count|today|tomorrow){RESET}
    keeper-todo show"
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
            _ => help(),
        }
    }
}
