use druid::{
    widget::{Controller, Image},
    EventCtx,
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
        _env: &Env,
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
                    AutoStepState::Paused(ref mut auto_step_data)
                    | AutoStepState::Playing(ref mut auto_step_data) => {
                        if let Some(time_left) = auto_step_data.time_left {
                            auto_step_data.time_left = Duration::from_secs_f32(time_left)
                                .checked_sub(now - self.start_time.unwrap())
                                .map(|d| d.as_secs_f32());
                        } else {
                            auto_step_data.set_next_image(data.images_paths.as_slice());
                            auto_step_data.step_forward(data.schedule.as_slice());
                            auto_step_data.time_left = Some(
                                auto_step_data
                                    .get_current_duration(data.schedule.as_slice())
                                    .as_secs_f32(),
                            );
                        }

                        self.start_time = Some(now);
                        self.timer_id = ctx.request_timer(Duration::from_millis(20));
                    }
                    AutoStepState::Stopped => (),
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
                        AutoStepState::Playing(AutoStepData::new(data))
                    }
                    AutoStepState::Playing(ref mut auto_step_data) => {
                        if let Some(time_left) = auto_step_data.time_left {
                            auto_step_data.time_left = Duration::from_secs_f32(time_left)
                                .checked_sub(now - self.start_time.unwrap())
                                .map(|d| d.as_secs_f32());
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
            }
            _ => (),
        }

        child.event(ctx, event, data, env)
    }
}
