use druid::{
    lens,
    widget::{Button, FillStrat, Flex, Image, Label, List, Tabs, TextBox},
    Command, Env, FileDialogOptions, ImageBuf, LensExt, Target, Widget, WidgetExt,
};

use std::{path::PathBuf, sync::Arc};

use crate::data::{AutoStepState, Config, ProgramData, START_AUTO_STEP, STOP_AUTO_STEP};
use crate::{
    controllers::{AutoStepControl, UpdateImage},
    data::{TOGGLE_BW, TOGGLE_MIRROR},
};

pub fn ui_builder() -> impl Widget<ProgramData> {
    let presentation = presentation_ui_builder();
    let configuration = configuration_ui_builder();

    Tabs::new()
        .with_tab("Player", presentation)
        .with_tab("Config", configuration)
}

pub fn configuration_ui_builder() -> impl Widget<ProgramData> {
    let open_dialog_options = FileDialogOptions::new()
        .select_directories()
        .name_label("Target")
        .title("Choose images")
        .button_text("Open");

    let current_dir_label = Label::new(
        |data: &Arc<Option<PathBuf>>, _: &Env| format! {"Current directory: {}", Option::as_ref(&data).map(|x| x.to_str().unwrap()).unwrap_or("None")},
    ).lens(ProgramData::config.then(Config::current_directory));

    let open = Button::new("Change").on_click(move |ctx, _, _| {
        ctx.submit_command(Command::new(
            druid::commands::SHOW_OPEN_PANEL,
            open_dialog_options.clone(),
            Target::Auto,
        ))
    });

    let schedule_ui = schedule_ui_builder().lens(ProgramData::config.then(Config::schedule));

    Flex::column()
        .with_child(Flex::row().with_child(current_dir_label).with_child(open))
        .with_child(schedule_ui)
}

pub fn schedule_ui_builder() -> impl Widget<Arc<Vec<(usize, usize)>>> {
    Flex::column()
        .with_child(
            Flex::row()
                .with_child(
                    Button::new("Add")
                        .on_click(|_, data: &mut Arc<Vec<(usize, usize)>>, _| {
                            let mut new_schedule: Vec<_> = (**data).clone();
                            new_schedule
                                .push(new_schedule.as_slice().last().unwrap_or(&(5, 30)).clone());
                            *data = Arc::new(new_schedule);
                        })
                        .padding(5.),
                )
                .with_child(
                    Button::new("Remove")
                        .on_click(|_, data: &mut Arc<Vec<(usize, usize)>>, _| {
                            let mut new_schedule: Vec<_> = (**data).clone();
                            new_schedule.pop();
                            *data = Arc::new(new_schedule);
                        })
                        .padding(5.),
                ),
        )
        .with_child(List::new(|| {
            Flex::row()
                .with_child(TextBox::new().lens(lens::Identity.map(
                    |(x, _): &(usize, usize)| x.to_string(),
                    |(x, _): &mut (usize, usize), y: String| *x = y.parse::<usize>().unwrap_or(*x),
                )))
                .with_child(TextBox::new().lens(lens::Identity.map(
                    |(_, x): &(usize, usize)| x.to_string(),
                    |(_, x): &mut (usize, usize), y: String| *x = y.parse::<usize>().unwrap_or(*x),
                )))
                .with_child(Label::new("s"))
        }))
}

pub fn presentation_ui_builder() -> impl Widget<ProgramData> {
    let play = Button::new(|data: &ProgramData, _: &Env| match data.state {
        AutoStepState::Playing(_) => "Pause".to_owned(),
        _ => "Play".to_owned(),
    })
    .on_click(|ctx, data: &mut ProgramData, _| {
        if data.images_paths.len() > 0 && data.config.schedule.len() > 0 {
            ctx.submit_command(START_AUTO_STEP);
        }
    });

    let reload = Button::new("Reload").on_click(|_ctx, data: &mut ProgramData, _env| {
        let mut end = false;
        if let Some(auto_step_data) = data.state.get_data_mut() {
            end = auto_step_data.set_next_image(data.images_paths.as_slice());
            auto_step_data.time_left =
                Some(auto_step_data.get_current_duration(&data.config.schedule) as f64);
        }

        if end {
            data.prepare_images(false);
        }

        data.reset_transformations();
    });

    let skip = Button::new("Skip").on_click(|_ctx, data: &mut ProgramData, _env| {
        let mut end = false;
        if let Some(auto_step_data) = data.state.get_data_mut() {
            end = auto_step_data.set_next_image(data.images_paths.as_slice());
            auto_step_data.step_forward(&data.config.schedule);
            auto_step_data.time_left =
                Some(auto_step_data.get_current_duration(&data.config.schedule) as f64);
        }

        if end {
            data.prepare_images(false);
        }

        data.reset_transformations();
    });

    let skip_block = Button::new("Skip block").on_click(|_ctx, data: &mut ProgramData, _env| {
        if let Some(auto_step_data) = data.state.get_data_mut() {
            auto_step_data.set_next_image(data.images_paths.as_slice());
            auto_step_data.step_forward_block(&data.config.schedule);
            auto_step_data.time_left =
                Some(auto_step_data.get_current_duration(&data.config.schedule) as f64);
        }
        data.reset_transformations();
    });

    let stop = Button::new("Stop").on_click(|ctx, _data: &mut ProgramData, _env| {
        ctx.submit_command(STOP_AUTO_STEP);
    });

    let black_and_white = Button::new("B/W").on_click(|ctx, _data: &mut ProgramData, _env| {
        ctx.submit_command(TOGGLE_BW);
    });

    let mirrored = Button::new("Mirror").on_click(|ctx, _data: &mut ProgramData, _env| {
        ctx.submit_command(TOGGLE_MIRROR);
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
    })
    .fix_width(50.0);

    let image = Image::new(ImageBuf::empty())
        .fill_mode(FillStrat::Contain)
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
        ));

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(play)
                .with_child(reload)
                .with_child(skip)
                .with_child(skip_block)
                .with_child(stop)
                .with_child(black_and_white)
                .with_child(mirrored)
                .with_child(current)
                .with_child(time),
        )
        .with_flex_child(image, 1.0)
        .center()
        .controller(AutoStepControl::new())
}
