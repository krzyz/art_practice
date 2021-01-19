#![feature(bool_to_option)]
use druid::widget::{Align, Button, Flex};
use druid::{
    commands, AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, FileDialogOptions,
    Handled, LocalizedString, PlatformError, Target, Widget, WindowDesc,
};

use std::fs;
use std::sync::Arc;

struct Delegate;

#[derive(Clone, Data)]
/// The main model for a todo list application.
struct ProgramData {
    images_paths: Arc<Vec<String>>,
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("Art practice").with_placeholder("Art practice"));
    let data: ProgramData = ProgramData {
        images_paths: Arc::new(vec![]),
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

    let mut col = Flex::column();
    col.add_child(open);
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
            let image_paths: Vec<_> = fs::read_dir(file_info.path())
                .expect("Unable to open chosen directory")
                .into_iter()
                .filter(|r| r.is_ok())
                .map(|r| r.unwrap().path())
                .filter(|r| {
                    r.extension()
                        .and_then(|ext| image_exts.contains(&ext.to_str().unwrap()).then_some(true))
                        .is_some()
                })
                .collect();

            for path in image_paths {
                println!("{}", path.display());
            }

            return Handled::Yes;
        }
        Handled::No
    }
}
