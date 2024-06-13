use crate::debug::line_renderer::DebugLine;
use crate::debug::DebugController;
use crate::voxel_loader::VoxelLoader;
use log::debug;
use octa_force::anyhow::Result;
use octa_force::controls::Controls;
use octa_force::glam::{vec3, vec4, Mat4, Vec3};

impl DebugController {
    pub fn update_rotation_debug(
        &mut self,
        voxel_loader: &mut VoxelLoader,
        controls: &Controls,
    ) -> Result<()> {
        if controls.t {
            voxel_loader.reload();
        }

        let (_, rot) = voxel_loader.find_model("Test")?;
        let num: u8 = rot.into();
        debug!("{:?}", num);

        let mat: Mat4 = rot.into();

        let x = vec3(1.0, 0.0, 0.0);
        let y = vec3(0.0, 1.0, 0.0);
        let z = vec3(0.0, 0.0, 1.0);

        let rx = mat.transform_vector3(x);
        let ry = mat.transform_vector3(y);
        let rz = mat.transform_vector3(z);

        self.add_lines(vec![
            DebugLine::new(Vec3::ZERO, rx, vec4(1.0, 0.0, 0.0, 1.0)),
            DebugLine::new(Vec3::ZERO, ry, vec4(0.0, 1.0, 0.0, 1.0)),
            DebugLine::new(Vec3::ZERO, rz, vec4(0.0, 0.0, 1.0, 1.0)),
        ]);

        self.text_renderer.push_texts()?;
        self.line_renderer.push_lines()?;

        Ok(())
    }
}
