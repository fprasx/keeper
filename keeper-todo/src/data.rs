use std::{collections::BTreeMap, fmt::Display, path::Path, process};

use anyhow::Context;
use chrono::{Local, NaiveDate, TimeZone};
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_text_mut, text_size};
use rusttype::{Font, Scale};
use serde::{Deserialize, Serialize};

use crate::cli::ShowSet;
use keeper_util::{
    color::{BLUE, GREEN, RED, RESET},
    fatal, info,
};

const HOME: &str = env!("HOME");
// Not on the $PATH that cron uses, so we hardcode it here
const FD: &str = concat!(env!("HOME"), "/.cargo/bin/fd");

const SCREEN_HEIGHT: u32 = 956;
const SCREEN_WIDTH: u32 = 1470;
const Y_START: u32 = 35;
const Y_PAD: u32 = 20;
const X_PAD: u32 = 35;
const CHAR_HEIGHT_TO_WIDTH: f32 = 1.9;
const NORD_BG: Rgb<u8> = Rgb([0x2e, 0x34, 0x40]);
const NORD_GREEN: Rgb<u8> = Rgb([0xa3, 0xbe, 0x8c]);
const NORD_RED: Rgb<u8> = Rgb([0xbf, 0x61, 0x6a]);
const NORD_WHITE: Rgb<u8> = Rgb([0xd8, 0xde, 0xe9]);
const NORD_BLUE: Rgb<u8> = Rgb([0x81, 0xa1, 0xc1]);

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

    pub fn add(&mut self, date: NaiveDate, desc: &str, hour: usize) -> anyhow::Result<()> {
        self.days
            .entry(date)
            .or_default()
            .timeslots
            .entry(hour)
            .or_default()
            .push(Task::new(desc.to_string()));

        self.render(ShowSet::Date(Local::now().date_naive()))
    }

    pub fn change(
        &mut self,
        date: NaiveDate,
        old_hour: usize,
        index: usize,
        new_hour: usize,
    ) -> anyhow::Result<()> {
        let Some(day) = self.days.get_mut(&date) else {
            fatal!("no tasks today");
        };
        let Some(tasks) = day.timeslots.get_mut(&old_hour) else {
            fatal!("no task at hour [{old_hour}]");
        };
        if index > tasks.len() {
            fatal!("index {index} is too large for hour {old_hour}");
        }
        let task = tasks.remove(index);

        // delete old_hour tasks vec if empty
        if tasks.is_empty() {
            day.timeslots.remove(&old_hour);
        }

        info!("moved '{}' from {old_hour} to {new_hour}", task.desc);
        day.timeslots.entry(new_hour).or_default().push(task);

        self.render(ShowSet::Date(Local::now().date_naive()))
    }

    pub fn mark(&mut self, date: NaiveDate, hour: usize, index: usize) -> anyhow::Result<()> {
        if let Some(task) = self
            .days
            .get_mut(&date)
            .map(|schedule| &mut schedule.timeslots)
            .and_then(|slot| slot.get_mut(&hour))
            .and_then(|hour| hour.get_mut(index))
        {
            task.mark_complete();
            info!("marked '{}' complete", task.desc);
        }

        self.render(ShowSet::Date(Local::now().date_naive()))
    }

    pub fn show(&self, set: ShowSet) {
        // avoid extra newline
        print!("{}", KeeperDisplay::new(self, set, ColorStyle::Color));
    }

    pub fn render(&self, set: ShowSet) -> anyhow::Result<()> {
        // delete old wall papers
        let wallpapers_dir = &format!("{HOME}/.local/share/keeper/wallpapers/");
        process::Command::new(FD)
            .args([
                "--type",
                "file",
                "--extension",
                "png",
                "--absolute-path",
                "--no-ignore",
                ".",
                wallpapers_dir,
                "-x",
                "rm",
            ])
            .output()
            .with_context(|| format!("failed to delete old wallpapers from {wallpapers_dir}"))?;

        // create new one
        let today = Local::now();
        let wallpaper_file = format!(
            "{HOME}/.local/share/keeper/wallpapers/wallpaper-{}.png",
            today.format("%y-%m-%d-%H-%M-%S")
        );

        let mut renderer = KeeperRenderer::new(self, set, NORD_BG);
        renderer.render();
        renderer
            .save(wallpaper_file.as_ref())
            .with_context(|| format!("failed to save new wallpaper to {wallpaper_file:?}"))?;

        // set new wallpaper
        process::Command::new("automator")
            .args([
                "-i",
                &wallpaper_file,
                &format!("{HOME}/.local/share/keeper/wp.workflow"),
            ])
            .output()
            .context("automator workflow failed")?;

        Ok(())
    }
}

enum ColorStyle {
    Color,
    NoColor,
}

struct KeeperDisplay<'a> {
    keeper: &'a Keeper,
    selection: ShowSet,
    color: ColorStyle,
}

impl<'a> KeeperDisplay<'a> {
    pub fn new(keeper: &'a Keeper, selection: ShowSet, color: ColorStyle) -> Self {
        Self {
            keeper,
            selection,
            color,
        }
    }

    fn fmt_day(&self, f: &mut std::fmt::Formatter<'_>, date: NaiveDate) -> std::fmt::Result {
        let mut green = GREEN;
        let mut red = RED;
        let mut blue = BLUE;
        let mut reset = RESET;
        if matches!(self.color, ColorStyle::NoColor) {
            green = "";
            red = "";
            blue = "";
            reset = "";
        }

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

            let bracket_color = match (all_done, past_due) {
                (true, true) => green,
                (true, false) => green,
                (false, true) => red,
                (false, false) => blue,
            };
            write!(f, "{bracket_color}[{time}]{reset}")?;

            for task in tasklist {
                let color = match (task.completed, past_due) {
                    (true, true) => green,
                    (true, false) => green,
                    (false, true) => red,
                    (false, false) => reset,
                };
                write!(f, " {color}({reset}{}{color}){reset}", task.desc)?
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
                let mut days = Local::now().date_naive().iter_days().take(days);
                let Some(date) = days.next() else {
                    return Ok(());
                };

                // avoid a double newline at the end
                self.fmt_day(f, date)?;
                for date in days {
                    writeln!(f)?;
                    self.fmt_day(f, date)?;
                }
            }
            ShowSet::Date(date) => {
                self.fmt_day(f, date)?;
            }
        }
        Ok(())
    }
}

struct KeeperRenderer<'a> {
    keeper: &'a Keeper,
    selection: ShowSet,
    image: ImageBuffer<Rgb<u8>, Vec<u8>>,
    font: Font<'static>,
    scale: Scale,
    xpos: i32,
    ypos: i32,
}

impl<'a> KeeperRenderer<'a> {
    pub fn new(keeper: &'a Keeper, selection: ShowSet, background_color: Rgb<u8>) -> Self {
        let image = ImageBuffer::from_pixel(SCREEN_WIDTH, SCREEN_HEIGHT, background_color);

        let font = Font::try_from_vec(Vec::from(include_bytes!("../iosevka-regular.ttc") as &[u8]))
            .expect("font is valid");

        let displayed = KeeperDisplay::new(keeper, selection, ColorStyle::NoColor).to_string();
        let height = displayed.lines().count() as f32;
        let width = (displayed
            .lines()
            .map(|l| l.len())
            .max()
            .expect("there is at least one non-empty line") as f32)
            / CHAR_HEIGHT_TO_WIDTH;

        // Take padding into account
        let effective_height = (image.height() - 2 * Y_PAD - Y_START) as f32;
        // Subtracting another 20 pixels prevents noticeable left-right asymmetry for some reason
        let effective_width = (image.width() - 2 * X_PAD - 20) as f32;

        let size = (effective_height / height).min(effective_width / width);
        let scale = Scale { x: size, y: size };

        Self {
            keeper,
            selection,
            image,
            font,
            scale,
            xpos: X_PAD as i32,
            ypos: (Y_PAD + Y_START) as i32,
        }
    }

    /// Add a string containing **NO NEWLINES**.
    fn render_literal(&mut self, color: Rgb<u8>, literal: &str) {
        draw_text_mut(
            &mut self.image,
            color,
            self.xpos,
            self.ypos,
            self.scale,
            &self.font,
            literal,
        );
        let (x, _) = text_size(self.scale, &self.font, literal);
        self.xpos += x;
    }

    fn render_newline(&mut self) {
        let (_, y) = text_size(self.scale, &self.font, "[]");
        self.xpos = X_PAD as i32;
        self.ypos += y;
    }

    pub fn render_day(&mut self, day: NaiveDate) {
        // Date header
        self.render_literal(NORD_WHITE, &format!("{}", day.format("%d %b %Y")));
        self.render_newline();

        let Some(Schedule { timeslots }) = self.keeper.days.get(&day) else {
            self.render_literal(NORD_WHITE, "Empty");
            return;
        };

        for (time, tasklist) in timeslots.iter() {
            let all_done = tasklist.iter().all(|t| t.completed);
            // If this hour has passed. For example, if time = 10, then we are
            // at 11:00 o'clock or later.
            let past_due = Local
                .from_local_datetime(&day.and_hms_opt(*time as u32, 59, 59).unwrap())
                .unwrap()
                < Local::now();

            let bracket_color = match (all_done, past_due) {
                (true, true) => NORD_GREEN,
                (true, false) => NORD_GREEN,
                (false, true) => NORD_RED,
                (false, false) => NORD_BLUE,
            };

            // Draw the [time]
            self.render_literal(bracket_color, &format!("[{time}]"));

            for task in tasklist {
                let paren_color = match (task.completed, past_due) {
                    (true, true) => NORD_GREEN,
                    (true, false) => NORD_GREEN,
                    (false, true) => NORD_RED,
                    (false, false) => NORD_WHITE,
                };
                self.render_literal(paren_color, " (");
                self.render_literal(NORD_WHITE, &task.desc);
                self.render_literal(paren_color, ")");
            }
            self.render_newline();
        }
    }

    pub fn render(&mut self) {
        match self.selection {
            ShowSet::Days(days) => {
                let mut days = Local::now().date_naive().iter_days().take(days);

                // avoid a double newline at the end
                if let Some(date) = days.next() {
                    self.render_day(date);
                };

                for date in days {
                    self.render_day(date);
                }
            }
            ShowSet::Date(date) => {
                self.render_day(date);
            }
        }
    }

    pub fn save(&mut self, path: &Path) -> anyhow::Result<()> {
        self.image
            .save(path)
            .with_context(|| format!("failed to save image to {path:?}"))
    }
}
