use druid::{commands, AppDelegate, Command, DelegateCtx, Env, Handled, Target};

use rand::seq::SliceRandom;
use rand::thread_rng;
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

            ctx.submit_command(STOP_AUTO_STEP);

            return Handled::Yes;
        }
        Handled::No
    }
}
