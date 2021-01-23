#![feature(bool_to_option)]
#![feature(try_blocks)]
#![feature(duration_zero)]

use druid::{
    commands, widget::Label, AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Event,
    FileDialogOptions, Handled, ImageBuf, Lens, LensExt, LocalizedString, PlatformError, Target, TimerToken,
    UpdateCtx, Widget, WidgetExt, WindowDesc,
};
use druid::{
    widget::{Button, Controller, Flex, Image},
    EventCtx, Selector,
};

use rand::seq::SliceRandom;
use rand::thread_rng;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{fs, time::Instant};

const START_AUTO_STEP: Selector<()> = Selector::new("start_auto_step");
const STOP_AUTO_STEP: Selector<()> = Selector::new("stop_auto_step");

struct UpdateImage;

type WrappedImage = Arc<RwLock<ImageBuf>>;

impl Controller<Option<WrappedImage>, Image> for UpdateImage {
    fn update(
        &mut self,
        child: &mut Image,
        ctx: &mut UpdateCtx<'_, '_>,
        old_data: &Option<WrappedImage>,
        data: &Option<WrappedImage>,
        _env: &Env,
    ) {
        match (old_data, data) {
            (Some(_), None) => {
                child.set_image_data(ImageBuf::empty());
                ctx.request_paint();
            }
            (_, Some(new_image)) => {
                child.set_image_data(new_image.read().unwrap().clone());
                ctx.request_paint();
            }
            (None, None) => (),
        };
    }
}

struct AutoStepControl {
    timer_id: TimerToken,
    start_time: Option<Instant>,
}

impl AutoStepControl {
    fn new() -> Self {
        AutoStepControl {
            timer_id: TimerToken::INVALID,
            start_time: None,
        }
    }
}

impl<W: Widget<ProgramData>> Controller<ProgramData, W> for AutoStepControl {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut ProgramData,
        env: &Env,
    ) {
        match event {
            Event::Timer(id) if id == &self.timer_id => {
                let now = Instant::now();
                match data.state {
                    AutoStepState::Paused(ref mut auto_step_data) | AutoStepState::Playing(ref mut auto_step_data) => {
                        if let Some(time_left) = auto_step_data.time_left {
                            auto_step_data.time_left = Duration::from_secs_f32(time_left).checked_sub(now - self.start_time.unwrap()).map(|d| d.as_secs_f32());
                        } else {
                            auto_step_data.set_next_image(data.images_paths.as_slice());
                            auto_step_data.step_forward(data.schedule.as_slice());
                            auto_step_data.time_left = Some(auto_step_data.get_current_duration(data.schedule.as_slice()).as_secs_f32());
                        }

                        self.start_time = Some(now);
                        self.timer_id = ctx.request_timer(Duration::from_millis(20));
                    },
                    AutoStepState::Stopped => ()
                }
            }
            Event::Command(cmd) if cmd.is(START_AUTO_STEP) => {
                let now = Instant::now();

                data.state = match data.state.clone() {
                    AutoStepState::Paused(auto_step_data) => {
                        self.timer_id = ctx.request_timer(Duration::from_millis(20));
                        self.start_time = Some(now);
                        AutoStepState::Playing(auto_step_data)
                    },
                    AutoStepState::Stopped => {
                        self.timer_id = ctx.request_timer(Duration::from_millis(20));
                        self.start_time = Some(now);
                        AutoStepState::Playing(AutoStepData::new(data))
                    },
                    AutoStepState::Playing(ref mut auto_step_data) => {
                        if let Some(time_left) = auto_step_data.time_left {
                            auto_step_data.time_left = Duration::from_secs_f32(time_left).checked_sub(now - self.start_time.unwrap()).map(|d| d.as_secs_f32());
                        }

                        self.timer_id = TimerToken::INVALID;
                        self.start_time = None;
                        AutoStepState::Paused(auto_step_data.clone())
                    },
                }

            }
            Event::Command(cmd) if cmd.is(STOP_AUTO_STEP) => {
                data.state = AutoStepState::Stopped;
                self.start_time = None;
                self.timer_id = TimerToken::INVALID;
            }
            _ => (),
        }

        child.event(ctx, event, data, env)
    }
}

struct Delegate;

#[derive(Clone, Data, Lens)]
struct AutoStepData {
    current_image_id: usize,
    current_image: WrappedImage,
    current: (usize, usize),
    time_left: Option<f32>,
}


#[derive(Clone, Data)]
enum AutoStepState {
    Stopped,
    Paused(AutoStepData),
    Playing(AutoStepData),
}

impl AutoStepState {
    fn get_data(&self) -> Option<&AutoStepData> {
        use AutoStepState::*;

        match self {
            Stopped => None,
            Paused(data) | Playing(data) => Some(data)
        }
    }

    fn get_data_mut(&mut self) -> Option<&mut AutoStepData> {
        use AutoStepState::*;

        match self {
            Stopped => None,
            Paused(data) | Playing(data) => Some(data)
        }
    }
}

#[derive(Clone, Data, Lens)]
/// The main model for a todo list application.
struct ProgramData {
    images_paths: Arc<Vec<PathBuf>>,
    schedule: Arc<Vec<(usize, Duration)>>,
    state: AutoStepState,
}

impl AutoStepData {
    fn new(data: &ProgramData) -> Self {
        let id = 0;
        AutoStepData {
            current_image_id: id,
            current_image: Arc::new(RwLock::new(ImageBuf::from_file(&data.images_paths[id]).unwrap())),
            current: (0, 0),
            time_left: Some(data.schedule[0].1.as_secs_f32()),
        }
    }

    fn set_image_from_path(&mut self, path: &PathBuf) {
        self.current_image = Arc::new(RwLock::new(ImageBuf::from_file(path).unwrap()));
    }

    fn set_image_id(&mut self, images_paths: &[PathBuf], id: usize) {
        self.current_image_id = id;
        self.set_image_from_path(&images_paths[id]);
    }

    fn set_next_image(&mut self, images_paths: &[PathBuf]) {
        let id = self.current_image_id;
        if id < images_paths.len() - 1 {
            self.set_image_id(images_paths, id + 1);
        } else {
            self.set_image_id(images_paths, 0);
        }
    }

    fn step_forward(&mut self, schedule: &[(usize, Duration)]) {
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

    fn get_current_duration(&self, schedule: &[(usize, Duration)]) -> Duration {
        schedule[self.current.0].1
    }
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("Art practice").with_placeholder("Art practice"))
        .with_min_size((1280., 720.));
    let data: ProgramData = ProgramData {
        images_paths: Arc::new(vec![]),
        schedule: Arc::new(vec![
            (5, Duration::from_secs(2)),
            (5, Duration::from_secs(4)),
        ]),
        state: AutoStepState::Stopped,
    };

    Ok(AppLauncher::with_window(main_window)
        .delegate(Delegate)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed"))
}

fn ui_builder() -> impl Widget<ProgramData> {
    let open_dialog_options = FileDialogOptions::new()
        .select_directories()
        .name_label("Target")
        .title("Choose images")
        .button_text("Open");

    let open = Button::new("Select directory").on_click(move |ctx, _, _| {
        ctx.submit_command(Command::new(
            druid::commands::SHOW_OPEN_PANEL,
            open_dialog_options.clone(),
            Target::Auto,
        ))
    });

    let play = Button::new(|data: &ProgramData, _: &Env| {
        match data.state {
            AutoStepState::Playing(_) => "Pause".to_owned(),
            _ => "Play".to_owned(),
        }
    })
    .on_click(|ctx, data: &mut ProgramData, _| {
        if data.images_paths.len() > 0 && data.schedule.len() > 0 {
            ctx.submit_command(START_AUTO_STEP);
        }
    });


    let next = Button::new("Next").on_click(|_ctx, data: &mut ProgramData, _env| {
        if let Some(auto_step_data) = data.state.get_data_mut() {
            auto_step_data.set_next_image(data.images_paths.as_slice());
        }
    });

    let stop = Button::new("Stop").on_click(|ctx, _data: &mut ProgramData, _env| {
        ctx.submit_command(STOP_AUTO_STEP);
    });

    let current =
        Label::new(|data: &ProgramData, _env: &Env| 
            format!("Current: {}", data.state.get_data().map_or("None".to_owned(), |data| format!("{:?}", data.current)))
        );

    let time = Label::new(|data: &ProgramData, _env: &Env|
            format!("Left: {:.2}", data.state.get_data().map_or(0., |data| data.time_left.unwrap_or(0.)))
    );


    let image = Image::new(ImageBuf::empty())
        .controller(UpdateImage)
        .lens(ProgramData::state.map(
            |x| x.get_data().map_or(None, |data| Some(data.current_image.clone())),
            |x, y| {
                if let Some(auto_step_data) = x.get_data_mut() {
                    auto_step_data.current_image = y.unwrap_or(Arc::new(RwLock::new(ImageBuf::empty())))
                }
            }))
        .fix_size(1024., 600.);

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(open)
                .with_child(play)
                .with_child(next)
                .with_child(stop)
                .with_child(current)
                .with_child(time),
        )
        .with_child(image)
        .center()
        .controller(AutoStepControl::new())
}

impl AppDelegate<ProgramData> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut ProgramData,
        _env: &Env,
    ) -> Handled {
        let image_exts = ["gif", "jpg", "jpeg", "png", "bmp"];

        if let Some(file_info) = cmd.get(commands::OPEN_FILE) {
            let mut images_paths: Vec<_> = fs::read_dir(file_info.path())
                .expect("Unable to open chosen directory")
                .into_iter()
                .filter(|r| r.is_ok())
                .map(|r| r.unwrap().path())
                .filter(|r| {
                    r.extension()
                        .map_or(false, |ext| image_exts.contains(&ext.to_str().unwrap()))
                })
                .collect();

            images_paths.shuffle(&mut thread_rng());

            data.images_paths = Arc::new(images_paths);

            ctx.submit_command(STOP_AUTO_STEP);

            return Handled::Yes;
        }
        Handled::No
    }
}
