use druid::{Data, ImageBuf, Lens};

use druid::Selector;

use directories::ProjectDirs;
use ron::de::from_reader;
use ron::ser::{to_writer_pretty, PrettyConfig};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::{create_dir_all, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{RwLock, Arc};

use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::rngs::ThreadRng;

pub fn get_cache_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "Real Complexity", "Art Practice").map(|proj_dirs| {
        proj_dirs
            .config_dir()
            .join(Path::new("config.ron"))
            .to_path_buf()
    })
}

pub const START_AUTO_STEP: Selector<()> = Selector::new("start_auto_step");
pub const STOP_AUTO_STEP: Selector<()> = Selector::new("stop_auto_step");

#[derive(Clone, Data, Lens, Serialize, Deserialize)]
pub struct Config {
    pub current_directory: Arc<Option<PathBuf>>,
    pub schedule: Arc<Vec<(usize, usize)>>,
}

impl Config {
    pub fn new() -> Self {
        let cached_config: Option<Config> = get_cache_path()
            .map(|path| File::open(path).ok().map(|f| from_reader(f).ok()).flatten())
            .flatten();

        if let Some(config) = cached_config {
            config
        } else {
            Config {
                current_directory: Arc::new(None),
                schedule: Arc::new(vec![(5, 30), (5, 60)]),
            }
        }
    }
    pub fn try_save(&self) -> io::Result<()> {
        let pretty = PrettyConfig::new()
            .with_depth_limit(2)
            .with_separate_tuple_members(true)
            .with_enumerate_arrays(true);

        get_cache_path()
            .map(|path| {
                create_dir_all(
                    path.parent()
                        .ok_or(io::Error::new(io::ErrorKind::Other, "unable to create dir"))?,
                )?;
                File::create(path).and_then(|f| {
                    to_writer_pretty(f, self, pretty)
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "can't save"))
                })
            })
            .ok_or(io::Error::new(io::ErrorKind::Other, "oh no!"))?
    }
}

#[derive(Clone, Data, Lens)]
/// The main model for a todo list application.
pub struct ProgramData {
    pub images_paths: Arc<Vec<PathBuf>>,
    pub config: Config,
    pub state: AutoStepState,
    pub rng: Arc<RwLock<ThreadRng>>,
}

impl ProgramData {
    pub fn new() -> Self {
        let mut data = ProgramData {
            images_paths: Arc::new(vec![]),
            config: Config::new(),
            state: AutoStepState::Stopped,
            rng: Arc::new(RwLock::new(thread_rng())),
        };
        data.prepare_images(true);
        data
    }

    pub fn prepare_images(&mut self, reload: bool) {
        let image_exts = ["gif", "jpg", "jpeg", "png", "bmp"];

        if reload {
            if let Some(dir_path) = (*self.config.current_directory).clone() {
                let images_paths: Vec<_> = fs::read_dir(dir_path.as_path())
                    .expect("Unable to open chosen directory")
                    .into_iter()
                    .filter(|r| r.is_ok())
                    .map(|r| r.unwrap().path())
                    .filter(|r| {
                        r.extension()
                            .map_or(false, |ext| image_exts.contains(&ext.to_str().unwrap()))
                    })
                    .collect();

                self.images_paths = Arc::new(images_paths);
            }
        }

        let mut images_paths = (*self.images_paths).clone();
        images_paths.shuffle(&mut *self.rng.write().unwrap());
        self.images_paths = Arc::new(images_paths);
    }
}

#[derive(Clone, Data)]
pub enum AutoStepState {
    Stopped,
    Paused(AutoStepData),
    Playing(AutoStepData),
}

impl AutoStepState {
    pub fn get_data(&self) -> Option<&AutoStepData> {
        use AutoStepState::*;

        match self {
            Stopped => None,
            Paused(data) | Playing(data) => Some(data),
        }
    }

    pub fn get_data_mut(&mut self) -> Option<&mut AutoStepData> {
        use AutoStepState::*;

        match self {
            Stopped => None,
            Paused(data) | Playing(data) => Some(data),
        }
    }
}

#[derive(Clone, Data, Lens)]
pub struct AutoStepData {
    pub current_image_id: usize,
    pub current_image: Arc<ImageBuf>,
    pub current: (usize, usize),
    pub time_left: Option<f64>,
}

impl AutoStepData {
    pub fn new(data: &ProgramData) -> Self {
        let id = 0;
        AutoStepData {
            current_image_id: id,
            current_image: Arc::new(ImageBuf::from_file(&data.images_paths[id]).unwrap()),
            current: (0, 0),
            time_left: Some(data.config.schedule[0].1 as f64),
        }
    }

    pub fn set_image_from_path(&mut self, path: &PathBuf) {
        self.current_image = Arc::new(ImageBuf::from_file(path).unwrap());
    }

    pub fn set_image_id(&mut self, images_paths: &[PathBuf], id: usize) {
        self.current_image_id = id;
        self.set_image_from_path(&images_paths[id]);
    }

    pub fn set_next_image(&mut self, images_paths: &[PathBuf]) -> bool {
        let mut end = false;
        let id = self.current_image_id;
        if id < images_paths.len() - 1 {
            self.set_image_id(images_paths, id + 1);
        } else {
            self.set_image_id(images_paths, 0);
            end = true;
        }

        end
    }

    pub fn step_forward(&mut self, schedule: &[(usize, usize)]) {
        let (big_step, small_step) = self.current;

        let current_big_step_length = schedule[big_step].0;
        self.current = if small_step >= current_big_step_length - 1 {
            if big_step >= schedule.len() - 1 {
                (0, 0)
            } else {
                (big_step + 1, 0)
            }
        } else {
            (big_step, small_step + 1)
        };
    }

    pub fn step_forward_block(&mut self, schedule: &[(usize, usize)]) {
        let (big_step, _) = self.current;

        self.current = if big_step >= schedule.len() - 1 {
            (0, 0)
        } else {
            (big_step + 1, 0)
        };
    }

    pub fn get_current_duration(&self, schedule: &[(usize, usize)]) -> usize {
        schedule[self.current.0].1
    }
}
