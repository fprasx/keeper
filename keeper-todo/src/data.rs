use crate::{
    cli::ShowSet,
    color::{GREEN, RED, RESET},
};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display};

#[derive(Serialize, Deserialize, Debug)]
pub struct Task {
    completed: bool,
    desc: String,
}

impl Task {
    pub fn new(desc: String) -> Self {
        Self {
            completed: false,
            desc,
        }
    }

    pub fn mark_complete(&mut self) {
        self.completed = true;
    }
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
pub struct Schedule {
    pub timeslots: BTreeMap<usize, Vec<Task>>,
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
pub struct Keeper {
    pub days: BTreeMap<NaiveDate, Schedule>,
}

impl Keeper {
    pub fn order(&mut self) {
        for schedule in self.days.values_mut() {
            for slot in schedule.timeslots.values_mut() {
                slot.sort_by_key(|task| task.completed)
            }
        }
    }

    pub fn add(&mut self, date: NaiveDate, desc: String, hour: usize) {
        self.days
            .entry(date)
            .or_default()
            .timeslots
            .entry(hour)
            .or_default()
            .push(Task::new(desc));
    }

    pub fn mark(&mut self, date: NaiveDate, hour: usize, index: usize) {
        if let Some(task) = self
            .days
            .get_mut(&date)
            .map(|schedule| &mut schedule.timeslots)
            .and_then(|slot| slot.get_mut(&hour))
            .and_then(|hour| hour.get_mut(index))
        {
            task.mark_complete()
        }
    }

    pub fn show(&self, set: ShowSet) {
        match set {
            ShowSet::Days(days) => {
                let mut print_newline = false; // to avoid printing an ending newline
                for date in Local::now().date_naive().iter_days().take(days) {
                    if print_newline {
                        println!();
                    }
                    print_newline = true;

                    println!("{}", date.format("%d %b %Y"));
                    if let Some(schedule) = self.days.get(&date) {
                        println!("{schedule}");
                    } else {
                        println!("Empty");
                    }
                }
            }
            ShowSet::Date(date) => {
                println!("{}", date.format("%d %b %Y"));
                if let Some(schedule) = self.days.get(&date) {
                    println!("{schedule}");
                } else {
                    println!("Empty");
                }
            }
        }
    }
}
