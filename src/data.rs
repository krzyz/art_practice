use druid::{Data, ImageBuf, Lens};

use druid::Selector;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub const START_AUTO_STEP: Selector<()> = Selector::new("start_auto_step");
pub const STOP_AUTO_STEP: Selector<()> = Selector::new("stop_auto_step");

#[derive(Clone, Data, Lens)]
/// The main model for a todo list application.
pub struct ProgramData {
    pub images_paths: Arc<Vec<PathBuf>>,
    pub schedule: Arc<Vec<(usize, Duration)>>,
    pub state: AutoStepState,
}

impl ProgramData {
    pub fn new() -> Self {
        ProgramData {
            images_paths: Arc::new(vec![]),
            schedule: Arc::new(vec![
                (5, Duration::from_secs(2)),
                (5, Duration::from_secs(4)),
            ]),
            state: AutoStepState::Stopped,
        }
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
    pub time_left: Option<f32>,
}

impl AutoStepData {
    pub fn new(data: &ProgramData) -> Self {
        let id = 0;
        AutoStepData {
            current_image_id: id,
            current_image: Arc::new(ImageBuf::from_file(&data.images_paths[id]).unwrap()),
            current: (0, 0),
            time_left: Some(data.schedule[0].1.as_secs_f32()),
        }
    }

    pub fn set_image_from_path(&mut self, path: &PathBuf) {
        self.current_image = Arc::new(ImageBuf::from_file(path).unwrap());
    }

    pub fn set_image_id(&mut self, images_paths: &[PathBuf], id: usize) {
        self.current_image_id = id;
        self.set_image_from_path(&images_paths[id]);
    }

    pub fn set_next_image(&mut self, images_paths: &[PathBuf]) {
        let id = self.current_image_id;
        if id < images_paths.len() - 1 {
            self.set_image_id(images_paths, id + 1);
        } else {
            self.set_image_id(images_paths, 0);
        }
    }

    pub fn step_forward(&mut self, schedule: &[(usize, Duration)]) {
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
        }
    }

    pub fn get_current_duration(&self, schedule: &[(usize, Duration)]) -> Duration {
        schedule[self.current.0].1
    }
}
