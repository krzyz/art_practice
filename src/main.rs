#![feature(bool_to_option)]
#![feature(try_blocks)]
#![feature(duration_zero)]

use druid::{
    commands, widget::Label, AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Event,
    FileDialogOptions, Handled, ImageBuf, Lens, LocalizedString, PlatformError, Target, TimerToken,
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
const PAUSE_AUTO_STEP: Selector<()> = Selector::new("pause_auto_step");
const STOP_AUTO_STEP: Selector<()> = Selector::new("stop_auto_step");

struct UpdateImage;

type WrappedImageOption = Option<Arc<RwLock<ImageBuf>>>;

impl Controller<WrappedImageOption, Image> for UpdateImage {
    fn update(
        &mut self,
        child: &mut Image,
        ctx: &mut UpdateCtx<'_, '_>,
        old_data: &WrappedImageOption,
        data: &WrappedImageOption,
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

                if let Some(start_time) = self.start_time {
                    data.time_left =
                        Arc::new(data.time_left.unwrap().checked_sub(now - start_time));
                }

                if *data.time_left == None {
                    data.set_next_image();
                    data.step_forward();
                    data.time_left = Arc::new(data.get_current_duration());
                }

                self.start_time = Some(now);
                self.timer_id = ctx.request_timer(Duration::from_millis(20));
            }
            Event::Command(cmd) if cmd.is(START_AUTO_STEP) => {
                self.timer_id = ctx.request_timer(Duration::from_millis(20));
                self.start_time = Some(Instant::now());
                if *data.time_left == None {
                    data.time_left = Arc::new(data.get_current_duration());
                }
            }
            Event::Command(cmd) if cmd.is(PAUSE_AUTO_STEP) => {
                let now = Instant::now();
                self.start_time = None;
                self.timer_id = TimerToken::INVALID;

                if let Some(start_time) = self.start_time {
                    data.time_left =
                        Arc::new(data.time_left.unwrap().checked_sub(now - start_time));
                }
            }
            Event::Command(cmd) if cmd.is(STOP_AUTO_STEP) => {
                self.start_time = None;
                self.timer_id = TimerToken::INVALID;
                data.time_left = Arc::new(None);
            }
            _ => (),
        }

        child.event(ctx, event, data, env)
    }
}

struct Delegate;

#[derive(Clone, Data, Lens)]
/// The main model for a todo list application.
struct ProgramData {
    images_paths: Arc<Vec<PathBuf>>,
    current_image_id: Option<usize>,
    current_image: WrappedImageOption,
    playing: bool,
    schedule: Arc<Vec<(usize, Duration)>>,
    current: Option<(usize, usize)>,
    time_left: Arc<Option<Duration>>,
}

impl ProgramData {
    fn set_image_from_path(&mut self, path: &PathBuf) {
        self.current_image = ImageBuf::from_file(path)
            .ok()
            .map(|img| Arc::new(RwLock::new(img)));
    }

    fn set_image_id(&mut self, id: Option<usize>) {
        self.current_image_id = id;
        match id {
            Some(id) => {
                let image_path = &self.images_paths[id].clone();
                self.set_image_from_path(image_path);
            }
            None => self.current_image = None,
        }
    }

    fn set_next_image(&mut self) {
        if let Some(id) = self.current_image_id {
            if id < self.images_paths.len() - 1 {
                self.set_image_id(Some(id + 1));
            } else {
                self.set_image_id(Some(0));
            }
        }
    }

    fn step_forward(&mut self) {
        if let Some((big_step, small_step)) = self.current {
            let current_big_step_length = self.schedule[big_step].0;
            self.current = if small_step >= current_big_step_length - 1 {
                if big_step >= self.schedule.len() - 1 {
                    Some((0, 0))
                } else {
                    Some((big_step + 1, 0))
                }
            } else {
                Some((big_step, small_step + 1))
            }
        }
    }

    fn get_current_duration(&self) -> Option<Duration> {
        if let Some((big_step, _)) = self.current {
            Some(self.schedule[big_step].1)
        } else {
            None
        }
    }
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("Art practice").with_placeholder("Art practice"))
        .with_min_size((1280., 720.));
    let data: ProgramData = ProgramData {
        images_paths: Arc::new(vec![]),
        current_image_id: None,
        current_image: None,
        playing: false,
        schedule: Arc::new(vec![
            (5, Duration::from_secs(2)),
            (5, Duration::from_secs(4)),
        ]),
        current: None,
        time_left: Arc::new(None),
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
        if data.playing {
            "Pause".to_owned()
        } else {
            "Play".to_owned()
        }
    })
    .on_click(|ctx, data: &mut ProgramData, _| {
        if data.images_paths.len() > 0 && data.schedule.len() > 0 {
            if data.playing {
                ctx.submit_command(PAUSE_AUTO_STEP);
            } else {
                if data.current_image_id == None {
                    data.set_image_id(Some(0));
                    data.current = Some((0, 0));
                }
                ctx.submit_command(START_AUTO_STEP);
            }
            data.playing = !data.playing;
        }
    });

    let next = Button::new("Next").on_click(|_ctx, data: &mut ProgramData, _env| {
        data.set_next_image();
    });

    let stop = Button::new("Stop").on_click(|ctx, data: &mut ProgramData, _env| {
        ctx.submit_command(STOP_AUTO_STEP);
        data.set_image_id(None);
        data.playing = false;
        data.current = None;
    });

    let current =
        Label::new(|data: &ProgramData, _env: &Env| format!("Current: {:?}", data.current));

    let time = Label::new(|data: &ProgramData, _env: &Env| format!("Left: {:?}", data.time_left));

    let image = Image::new(ImageBuf::empty())
        .controller(UpdateImage)
        .lens(ProgramData::current_image)
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

            data.set_image_id(None);
            data.playing = false;
            ctx.submit_command(STOP_AUTO_STEP);

            return Handled::Yes;
        }
        Handled::No
    }
}
