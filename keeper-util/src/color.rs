pub const RESET: &str = "\x1b[0m";
pub const BLACK: &str = "\x1b[0;30m";
pub const RED: &str = "\x1b[0;31m";
pub const GREEN: &str = "\x1b[0;32m";
pub const YELLOW: &str = "\x1b[0;33m";
pub const BLUE: &str = "\x1b[0;34m";
pub const PURPLE: &str = "\x1b[0;35m";
pub const CYAN: &str = "\x1b[0;36m";
pub const WHITE: &str = "\x1b[0;37m";

#[macro_export]
macro_rules! red {
    ($($t:tt),+ $(,)?) => {{
        print!("{}", $crate::color::RED);
        print!($($t),+);
        print!("{}", $crate::color::RESET);
        println!();
    }};
}

#[macro_export]
macro_rules! green {
    ($($t:tt),+ $(,)?) => {{
        print!("{}", $crate::color::GREEN);
        print!($($t),+);
        print!("{}", $crate::color::RESET);
        println!();
    }};
}
