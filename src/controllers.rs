use druid::{
    widget::{Controller, Image},
    Data, EventCtx,
};
use druid::{Env, Event, ImageBuf, TimerToken, UpdateCtx, Widget};

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crate::data::*;

pub struct UpdateImage;

impl Controller<Option<Arc<ImageBuf>>, Image> for UpdateImage {
    fn update(
        &mut self,
        child: &mut Image,
        ctx: &mut UpdateCtx<'_, '_>,
        old_data: &Option<Arc<ImageBuf>>,
        data: &Option<Arc<ImageBuf>>,
        env: &Env,
    ) {
        match (old_data, data) {
            (Some(_), None) => {
                child.set_image_data(ImageBuf::empty());
                ctx.request_paint();
            }
            (_, Some(new_image)) => {
                child.set_image_data((**new_image).clone());
                ctx.request_paint();
            }
            (None, None) => (),
        };

        child.update(ctx, old_data, data, env);
    }
}

pub struct AutoStepControl {
    pub timer_id: TimerToken,
    pub start_time: Option<Instant>,
}

impl AutoStepControl {
    pub fn new() -> Self {
        AutoStepControl {
            timer_id: TimerToken::INVALID,
            start_time: None,
        }
    }
}

impl<W: Widget<ProgramData>> Controller<ProgramData, W> for AutoStepControl {
    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx<'_, '_>,
        old_data: &ProgramData,
        data: &ProgramData,
        env: &Env,
    ) {
        if !old_data.config.same(&data.config) {
            data.config.try_save().ok();
        }
        child.update(ctx, old_data, data, env);
    }

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
                let mut end = false;
                match data.state {
                    AutoStepState::Paused(ref mut auto_step_data)
                    | AutoStepState::Playing(ref mut auto_step_data) => {
                        if let Some(time_left) = auto_step_data.time_left {
                            auto_step_data.time_left = Duration::from_secs_f64(time_left)
                                .checked_sub(now - self.start_time.unwrap())
                                .map(|d| d.as_secs_f64());
                        } else {
                            end = auto_step_data.set_next_image(data.images_paths.as_slice());
                            auto_step_data.step_forward(data.config.schedule.as_slice());
                            auto_step_data.time_left = Some(
                                auto_step_data.get_current_duration(data.config.schedule.as_slice())
                                    as f64,
                            );
                            data.reset_transformations();
                        }

                        self.start_time = Some(now);
                        self.timer_id = ctx.request_timer(Duration::from_millis(20));
                    }
                    AutoStepState::Stopped => (),
                }
                if end {
                    data.prepare_images(false);
                    data.reset_transformations();
                }
            }
            Event::Command(cmd) if cmd.is(START_AUTO_STEP) => {
                let now = Instant::now();

                data.state = match data.state.clone() {
                    AutoStepState::Paused(auto_step_data) => {
                        self.timer_id = ctx.request_timer(Duration::from_millis(20));
                        self.start_time = Some(now);
                        AutoStepState::Playing(auto_step_data)
                    }
                    AutoStepState::Stopped => {
                        self.timer_id = ctx.request_timer(Duration::from_millis(20));
                        self.start_time = Some(now);
                        data.reset_transformations();
                        AutoStepState::Playing(AutoStepData::new(data))
                    }
                    AutoStepState::Playing(ref mut auto_step_data) => {
                        if let Some(time_left) = auto_step_data.time_left {
                            auto_step_data.time_left = Duration::from_secs_f64(time_left)
                                .checked_sub(now - self.start_time.unwrap())
                                .map(|d| d.as_secs_f64());
                        }

                        self.timer_id = TimerToken::INVALID;
                        self.start_time = None;
                        AutoStepState::Paused(auto_step_data.clone())
                    }
                }
            }
            Event::Command(cmd) if cmd.is(STOP_AUTO_STEP) => {
                data.state = AutoStepState::Stopped;
                self.start_time = None;
                self.timer_id = TimerToken::INVALID;
                data.prepare_images(false);
                data.reset_transformations();
            }
            Event::Command(cmd) if cmd.is(TOGGLE_BW) => {
                data.black_and_white = !data.black_and_white;
                if let Some(state_data) = data.state.get_data_mut() {
                    if data.black_and_white {
                        state_data.make_bw();
                    } else {
                        state_data.restore_image(false, data.mirrored);
                    }
                }
            }
            Event::Command(cmd) if cmd.is(TOGGLE_MIRROR) => {
                data.mirrored = !data.mirrored;
                if let Some(state_data) = data.state.get_data_mut() {
                    if data.mirrored {
                        state_data.mirror();
                    } else {
                        state_data.restore_image(data.black_and_white, false);
                    }
                }
            }
            _ => (),
        }

        child.event(ctx, event, data, env)
    }
}
