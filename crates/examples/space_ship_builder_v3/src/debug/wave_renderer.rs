use crate::ship::Ship;
use crate::ship_mesh::ShipMesh;
use octa_force::anyhow::Result;

struct DebugWaveRenderer {
    mesh: ShipMesh<66, 64>,
}

impl DebugWaveRenderer {
    pub fn new(image_len: usize) -> Result<Self> {
        Ok(DebugWaveRenderer {
            mesh: ShipMesh::new::<16>(image_len)?,
        })
    }

    pub fn update(ship: &Ship) -> Result<Self> {
        Ok(DebugWaveRenderer {
            mesh: ShipMesh::new::<16>(image_len)?,
        })
    }
}
