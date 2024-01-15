use std::{collections::BTreeMap, fmt::Display, path::Path};

use chrono::{Local, NaiveDate, TimeZone};
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_text_mut, text_size};
use rusttype::{Font, Scale};
use serde::{Deserialize, Serialize};

use crate::cli::ShowSet;
use keeper_util::color::{GREEN, RED, RESET, YELLOW};

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
        // avoid extra newline
        print!("{}", KeeperDisplay::new(self, set, ColorStyle::Color));
    }

    pub fn render(&self, set: ShowSet, path: &Path) {
        let mut renderer = KeeperRenderer::new(self, set, NORD_BG);
        renderer.render();
        renderer.save(path);
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
        let mut yellow = YELLOW;
        let mut reset = RESET;
        if matches!(self.color, ColorStyle::NoColor) {
            green = "";
            red = "";
            yellow = "";
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

            match (all_done, past_due) {
                (true, true) => write!(f, "{green}[{time}]{reset}")?,
                (true, false) => write!(f, "{green}[{time}]{reset}")?,
                (false, true) => write!(f, "{red}[{time}]{reset}")?,
                (false, false) => write!(f, "{yellow}[{time}]{reset}")?,
            }

            for task in tasklist {
                match (task.completed, past_due) {
                    (true, true) => write!(f, " {green}({reset}{}{green}){reset}", task.desc)?,
                    (true, false) => write!(f, " {green}({reset}{}{green}){reset}", task.desc)?,
                    (false, true) => write!(f, " {red}({reset}{}{red}){reset}", task.desc)?,
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
        // These generics are necessary for type inference later on. Fails to compile without
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

        // 3 pts = 4 pixels
        let size = (effective_height / height).min(effective_width / width) * (4.0 / 3.0);
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

    pub fn save(&mut self, path: &Path) {
        self.image.save(path).expect("os L");
    }
}
