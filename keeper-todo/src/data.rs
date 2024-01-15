use crate::cli::ShowSet;
use chrono::{Local, NaiveDate, TimeZone};
use keeper_util::color::{GREEN, RED, RESET, YELLOW};
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Schedule {
    pub timeslots: BTreeMap<usize, Vec<Task>>,
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

    pub fn add(&mut self, date: NaiveDate, desc: &str, hour: usize) {
        self.days
            .entry(date)
            .or_default()
            .timeslots
            .entry(hour)
            .or_default()
            .push(Task::new(desc.to_string()));
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
        print!("{}", KeeperDisplay::new(self, set));
    }

    pub fn render(&self) {}
}

struct KeeperDisplay<'a> {
    keeper: &'a Keeper,
    selection: ShowSet,
}

impl<'a> KeeperDisplay<'a> {
    pub fn new(keeper: &'a Keeper, selection: ShowSet) -> Self {
        Self { keeper, selection }
    }

    fn fmt_date(&self, f: &mut std::fmt::Formatter<'_>, date: NaiveDate) -> std::fmt::Result {
        writeln!(f, "{}", date.format("%d %b %Y"))?;

        let Some(Schedule { timeslots }) = self.keeper.days.get(&date) else {
            writeln!(f, "Empty")?;
            return Ok(());
        };

        for (time, tasklist) in timeslots.iter() {
            let all_done = tasklist.iter().all(|t| t.completed);
            // If this hour has passed. For example, if time = 10, then we are
            // at 11:00 o'clock or later.
            let past_due = Local
                .from_local_datetime(&date.and_hms_opt(*time as u32, 59, 59).unwrap())
                .unwrap()
                < Local::now();

            match (all_done, past_due) {
                (true, true) => write!(f, "{GREEN}[{time}]{RESET}")?,
                (true, false) => write!(f, "{GREEN}[{time}]{RESET}")?,
                (false, true) => write!(f, "{RED}[{time}]{RESET}")?,
                (false, false) => write!(f, "{YELLOW}[{time}]{RESET}")?,
            }

            for task in tasklist {
                match (task.completed, past_due) {
                    (true, true) => write!(f, " {GREEN}({RESET}{}{GREEN}){RESET}", task.desc)?,
                    (true, false) => write!(f, " {GREEN}({RESET}{}{GREEN}){RESET}", task.desc)?,
                    (false, true) => write!(f, " {RED}({RESET}{}{RED}){RESET}", task.desc)?,
                    (false, false) => write!(f, " ({})", task.desc)?,
                }
            }

            writeln!(f,)?;
        }
        Ok(())
    }
}

impl Display for KeeperDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.selection {
            ShowSet::Days(days) => {
                // avoid a double newline at the end
                let mut days = Local::now().date_naive().iter_days().take(days);
                let Some(date) = days.next() else { return Ok(()) };
                self.fmt_date(f, date)?;
                for date in days {
                    writeln!(f)?;
                    self.fmt_date(f, date)?;
                }
            }
            ShowSet::Date(date) => {
                self.fmt_date(f, date)?;
            }
        }
        Ok(())
    }
}
