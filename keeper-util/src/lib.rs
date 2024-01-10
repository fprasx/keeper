pub mod color;

#[macro_export]
macro_rules! error {
    ($($t:tt),+ $(,)?) => {
        {
            print!("{}ERROR{} ", $crate::color::RED, $crate::color::RESET);
            println!($($t),+);
        }
    };
}

#[macro_export]
macro_rules! fatal {
    ($($t:tt),+ $(,)?) => {
        {
            print!("{}ERROR{} ", $crate::color::RED, $crate::color::RESET);
            println!($($t),+);
            ::std::process::exit(1);
        }
    };
}

use std::{
    fs::{self, DirBuilder, File},
    marker::PhantomData,
    path::{Path, PathBuf},
    process::{self, Stdio},
};

use anyhow::{anyhow, ensure, Context};
use serde::{Deserialize, Serialize};

pub struct DataManager<T> {
    data_path: PathBuf,
    _unserialized_type: PhantomData<T>,
}

impl<T> DataManager<T>
where
    T: Default + Serialize + for<'a> Deserialize<'a>,
{
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        ensure!(path.parent().is_some(), "path does not have a parent");
        ensure!(path.to_str().is_some(), "path is not valid unicode");
        Ok(Self {
            data_path: PathBuf::from(path),
            _unserialized_type: PhantomData,
        })
    }

    fn data_dir(&self) -> &Path {
        self.data_path.parent().unwrap()
    }

    fn data_dir_str(&self) -> &str {
        self.data_dir().to_str().unwrap()
    }

    fn data_filename_str(&self) -> &str {
        self.data_path.to_str().unwrap()
    }

    fn init_storage(&self) -> anyhow::Result<()> {
        // check for DATA_DIR
        if !self.data_dir().exists() {
            DirBuilder::new()
                .recursive(true)
                .create(self.data_dir())
                .with_context(|| {
                    format!("failed to create data directory at {}", self.data_dir_str())
                })?;
        }

        // check for DATA_PATH
        if !Path::new(self.data_dir()).exists() {
            File::create(self.data_dir()).with_context(|| {
                format!("failed to create data file at {}", self.data_filename_str())
            })?;

            // Init file to empty keeper
            let keeper = T::default();
            let ron = ron::ser::to_string_pretty(&keeper, Default::default())
                .context("failed to serialize RON")?;

            fs::write(self.data_dir(), ron).with_context(|| {
                format!("failed to write RON back to {}", self.data_filename_str())
            })?;
        }

        // init git repo fi need be
        if process::Command::new("git")
            .args(["-C", self.data_dir_str(), "status"])
            .stdout(Stdio::null())
            .status()
            .context("problem starting process for git status")?
            .code()
            .ok_or_else(|| anyhow!("git status returned no exit code (terminated by signal)"))?
            != 0
        {
            process::Command::new("git")
                .args(["-C", self.data_filename_str(), "init"])
                .status()
                .with_context(|| format!("failed to run git init in {}", self.data_dir_str()))?;
        }

        Ok(())
    }

    /// Load current keeper.
    pub fn load_data(&self) -> anyhow::Result<T> {
        self.init_storage().context("failed to load storage")?;

        let contents = fs::read_to_string(&self.data_path)
            .with_context(|| format!("failed to read from {}", self.data_filename_str()))?;

        ron::from_str(&contents).context("failed to deserialize RON")
    }

    /// Commit a new keeper.
    pub fn commit_data(&self, data: &T, commit_message: &str) -> anyhow::Result<()> {
        self.init_storage().context("failed to load storage")?;

        let ron = ron::ser::to_string_pretty(data, Default::default())
            .context("failed to serialize RON")?;

        fs::write(&self.data_path, ron)
            .with_context(|| format!("failed to write RON back to {}", self.data_filename_str()))?;

        process::Command::new("git")
            .stdout(Stdio::null())
            .args(["-C", self.data_dir_str(), "add", self.data_filename_str()])
            .status()
            .context("failed to run git add")?;

        process::Command::new("git")
            .stdout(Stdio::null())
            .args(["-C", self.data_dir_str(), "commit", "-m", commit_message])
            .status()
            .context("failed to run git add")?;

        Ok(())
    }
}
