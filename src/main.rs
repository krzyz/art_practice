#![feature(bool_to_option)]
use druid::widget::{Align, Button, Controller, Flex, Image};
use druid::{
    commands, AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, FileDialogOptions,
    Handled, ImageBuf, Lens, LocalizedString, PlatformError, Target, UpdateCtx, Widget, WidgetExt,
    WindowDesc,
};

use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

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
            },
            (_, Some(new_image)) => {
                child.set_image_data(new_image.read().unwrap().clone());
                ctx.request_paint();
            },
            (None, None) => (),
        };
    }
}

struct Delegate;

#[derive(Clone, Data, Lens)]
/// The main model for a todo list application.
struct ProgramData {
    images_paths: Arc<Vec<PathBuf>>,
    current_image: Option<Arc<RwLock<ImageBuf>>>,
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("Art practice").with_placeholder("Art practice"))
        .with_min_size((1280., 720.));
    let data: ProgramData = ProgramData {
        images_paths: Arc::new(vec![]),
        current_image: None,
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

    let open = Button::new("Select directories").on_click(move |ctx, _, _| {
        ctx.submit_command(Command::new(
            druid::commands::SHOW_OPEN_PANEL,
            open_dialog_options.clone(),
            Target::Auto,
        ))
    });

    let image = Image::new(ImageBuf::empty())
        .controller(UpdateImage)
        .lens(ProgramData::current_image)
        .fix_size(1024., 600.);

    let mut col = Flex::column();
    col.add_child(open);
    col.add_child(image);
    Align::centered(col)
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
        let image_exts = ["jpg", "jpeg", "png", "bmp"];

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

            println!(
                "{:?}",
                data.current_image
                    .as_ref()
                    .map(|x| x.read().unwrap().raw_pixels()[0])
            );

            if data.images_paths.len() > 0 {
                let image_path = &data.images_paths[0];
                data.current_image = ImageBuf::from_file(image_path)
                    .ok()
                    .map(|img| Arc::new(RwLock::new(img)));
            }

            return Handled::Yes;
        }
        Handled::No
    }
}
