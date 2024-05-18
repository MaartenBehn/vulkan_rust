use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::str::FromStr;

use octa_force::anyhow::Ok;
use octa_force::glam::Vec3;

const MOVEMENT_FILE_PATH: &str = "movement.txt";

pub struct MovementDebug {
    file: Option<File>,
    lines: Vec<String>,
}

impl MovementDebug {
    pub fn new(read: bool) -> Result<Self> {
        if !read {
            let file = File::create(MOVEMENT_FILE_PATH)?;

            return Ok(MovementDebug {
                file: Some(file),
                lines: Vec::new(),
            });
        } else {
            let file = File::open(MOVEMENT_FILE_PATH)?;
            let reader = BufReader::new(file);

            let mut lines = Vec::new();
            for line in reader.lines() {
                lines.push(line?);
            }

            return Ok(MovementDebug { file: None, lines });
        }
    }

    pub fn write(&mut self, camera: &Camera) -> Result<()> {
        self.file.as_ref().unwrap().write_fmt(format_args!(
            "{:?} {:?} {:?} {:?} {:?} {:?}\n",
            camera.position.x,
            camera.position.y,
            camera.position.z,
            camera.direction.x,
            camera.direction.y,
            camera.direction.z,
        ))?;

        Ok(())
    }

    pub fn read(&mut self, camera: &mut Camera, index: usize) -> Result<()> {
        if self.lines.len() < index {
            return Ok(());
        }

        let line = &self.lines[index];
        let parts: Vec<&str> = line.split(" ").collect();

        let pos = Vec3::new(
            f32::from_str(parts[0])?,
            f32::from_str(parts[1])?,
            f32::from_str(parts[2])?,
        );
        let dir = Vec3::new(
            f32::from_str(parts[3])?,
            f32::from_str(parts[4])?,
            f32::from_str(parts[5])?,
        );

        camera.position = pos;
        camera.direction = dir;

        Ok(())
    }
}
