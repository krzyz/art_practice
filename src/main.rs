#![feature(bool_to_option)]
#![feature(try_blocks)]
#![feature(duration_zero)]

use druid::{AppLauncher, LocalizedString, PlatformError, WindowDesc};

mod controllers;
mod data;
mod delegate;
mod view;

use data::ProgramData;
use delegate::Delegate;
use view::ui_builder;

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("Art practice").with_placeholder("Art practice"))
        .with_min_size((1280., 720.));

    Ok(AppLauncher::with_window(main_window)
        .delegate(Delegate)
        .use_simple_logger()
        .launch(ProgramData::new())
        .expect("launch failed"))
}
