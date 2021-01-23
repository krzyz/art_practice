use druid::{
    widget::{Button, Flex, Image, Label},
    Command, Env, FileDialogOptions, ImageBuf, LensExt, Target, Widget, WidgetExt,
};

use std::sync::Arc;

use crate::controllers::{AutoStepControl, UpdateImage};
use crate::data::{AutoStepState, ProgramData, START_AUTO_STEP, STOP_AUTO_STEP};

pub fn ui_builder() -> impl Widget<ProgramData> {
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

    let play = Button::new(|data: &ProgramData, _: &Env| match data.state {
        AutoStepState::Playing(_) => "Pause".to_owned(),
        _ => "Play".to_owned(),
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

    let current = Label::new(|data: &ProgramData, _env: &Env| {
        format!(
            "Current: {}",
            data.state
                .get_data()
                .map_or("None".to_owned(), |data| format!("{:?}", data.current))
        )
    });

    let time = Label::new(|data: &ProgramData, _env: &Env| {
        format!(
            "Left: {:.2}",
            data.state
                .get_data()
                .map_or(0., |data| data.time_left.unwrap_or(0.))
        )
    });

    let image = Image::new(ImageBuf::empty())
        .controller(UpdateImage)
        .lens(ProgramData::state.map(
            |x| {
                x.get_data()
                    .map_or(None, |data| Some(data.current_image.clone()))
            },
            |x, y| {
                if let Some(auto_step_data) = x.get_data_mut() {
                    auto_step_data.current_image = y.unwrap_or(Arc::new(ImageBuf::empty()))
                }
            },
        ))
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
