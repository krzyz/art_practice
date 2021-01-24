use druid::{commands, AppDelegate, Command, DelegateCtx, Env, Handled, Target};

use std::fs;
use std::sync::Arc;

use crate::data::*;

pub struct Delegate;

impl AppDelegate<ProgramData> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut ProgramData,
        _env: &Env,
    ) -> Handled {

        if let Some(file_info) = cmd.get(commands::OPEN_FILE) {
            data.config.current_directory = Arc::new(Some(file_info.path().to_path_buf()));

            data.prepare_images(true);

            ctx.submit_command(STOP_AUTO_STEP);

            return Handled::Yes;
        }
        Handled::No
    }
}
