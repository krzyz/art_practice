#![feature(bool_to_option)]
use druid::{
    commands, AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Event, FileDialogOptions,
    Handled, ImageBuf, Lens, LocalizedString, PlatformError, Target, TimerToken, UpdateCtx, Widget,
    WidgetExt, WindowDesc,
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
    this_duration: Duration,
}

impl AutoStepControl {
    fn new() -> Self {
        AutoStepControl {
            timer_id: TimerToken::INVALID,
            start_time: None,
            this_duration: Duration::from_secs(5),
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
                data.set_next_image();
                self.timer_id = ctx.request_timer(Duration::from_secs(5));
                self.this_duration = Duration::from_secs(5);
                self.start_time = Some(Instant::now());
            }
            Event::Command(cmd) if cmd.is(START_AUTO_STEP) => {
                self.timer_id = ctx.request_timer(self.this_duration);
                self.start_time = Some(Instant::now());
            }
            Event::Command(cmd) if cmd.is(PAUSE_AUTO_STEP) => {
                if let Some(start_time) = self.start_time {
                    self.this_duration = self.this_duration - (Instant::now() - start_time);
                }
                self.timer_id = TimerToken::INVALID;
                self.start_time = None;
            }
            Event::Command(cmd) if cmd.is(STOP_AUTO_STEP) => {
                self.this_duration = Duration::from_secs(5);
                self.timer_id = TimerToken::INVALID;
                self.start_time = None;
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
        if data.playing {
            ctx.submit_command(PAUSE_AUTO_STEP);
        } else {
            ctx.submit_command(START_AUTO_STEP);
        }
        data.playing = !data.playing;
    });

    let next = Button::new("Next").on_click(|_ctx, data: &mut ProgramData, _env| {
        data.set_next_image();
    });

    let image = Image::new(ImageBuf::empty())
        .controller(UpdateImage)
        .lens(ProgramData::current_image)
        .fix_size(1024., 600.);

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(open)
                .with_child(play)
                .with_child(next),
        )
        .with_child(image)
        .center()
        .controller(AutoStepControl::new())
}

impl AppDelegate<ProgramData> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
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

            if data.images_paths.len() > 0 {
                data.set_image_id(Some(0));
            } else {
                data.set_image_id(None);
            }

            return Handled::Yes;
        }
        Handled::No
    }
}
