use std::collections::{BTreeSet, BinaryHeap, VecDeque};
use std::time::Duration;

use app::anyhow::{bail, Result};
use app::glam::{ivec3, mat2, mat3, uvec3, vec2, vec3, IVec3, Mat2, UVec3, Vec4Swizzles};
use app::log;
use app::vulkan::Context;

use crate::math::{get_neigbor_offsets, to_1d, to_1d_i, to_3d};
use crate::node::{NodeController, NodeID};
use crate::rotation::Rot;
use crate::ship_mesh::ShipMesh;

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Default, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct Cell {
    pub m_id: NodeID,
    pub id: NodeID,
}

pub struct Ship {
    pub size: UVec3,
    pub max_index: isize,

    pub cells: Vec<Cell>,
    pub m_prop_indices: VecDeque<usize>,
    pub prop_indices: VecDeque<usize>,
    pub collapses_per_tick: usize,
    pub fully_collapsed: bool,

    pub mesh: ShipMesh,
    pub m_indices: Vec<usize>,
}

impl Ship {
    pub fn new(context: &Context, node_controller: &NodeController) -> Result<Ship> {
        let size = uvec3(100, 100, 1);
        let max_index = (size.x * size.y * size.z) as usize;

        let mesh = ShipMesh::new(context, max_index)?;

        let mut ship = Ship {
            size,
            max_index: max_index as isize,
            cells: vec![Cell::default(); (size.x * size.y * size.z) as usize],
            m_prop_indices: VecDeque::new(),
            prop_indices: VecDeque::new(),
            collapses_per_tick: 2,
            fully_collapsed: false,
            mesh,
            m_indices: Vec::new(),
        };

        ship.place_node(uvec3(5, 5, 0), NodeID::new(3, Rot::default()))?;

        Ok(ship)
    }

    pub fn tick(&mut self, node_controller: &NodeController, deltatime: Duration) -> Result<()> {
        if self.prop_indices.is_empty() {
            return Ok(());
        }

        log::info!("{:?} {:?}", self.collapses_per_tick, deltatime);

        if self.fully_collapsed {
            if deltatime < MIN_TICK_LENGTH && self.collapses_per_tick < usize::MAX / 2 {
                self.collapses_per_tick *= 2;
            } else if deltatime > MAX_TICK_LENGTH && self.collapses_per_tick > 4 {
                self.collapses_per_tick /= 2;
            }
        }
        self.fully_collapsed = true;

        let mut changed_indices = Vec::new();
        for _ in 0..(self.collapses_per_tick) {
            let i = self.collapse(node_controller);

            if i.is_some() {
                changed_indices.push(i.unwrap())
            }

            if self.prop_indices.is_empty() {
                self.fully_collapsed = false;
                break;
            }
        }

        //self.print_ship();

        self.mesh.update(&self.cells, self.size, &changed_indices)?;

        Ok(())
    }

    pub fn get_cell(&self, pos: UVec3) -> Result<Cell> {
        self.get_cell_i(pos.as_ivec3())
    }

    pub fn get_cell_i(&self, pos: IVec3) -> Result<Cell> {
        let index = to_1d_i(pos, self.size.as_ivec3());

        if index < 0 || index >= self.max_index {
            bail!("Pos not in ship")
        }

        Ok(self.cells[index as usize])
    }

    pub fn place_node(&mut self, pos: UVec3, id: NodeID) -> Result<()> {
        log::info!("Place: {pos:?}");

        for i in 0..self.max_index {
            let cell = &mut self.cells[i as usize];
            cell.id = cell.m_id;
        }

        let index = to_1d(pos, self.size);
        self.cells[index].id = id;
        self.cells[index].m_id = id;
        self.m_indices.push(index);

        let mut neigbors = self.get_neigbors(pos);
        self.m_prop_indices.append(&mut neigbors);
        self.prop_indices = self.m_prop_indices.clone();

        self.mesh.reset();
        self.mesh.update(&self.cells, self.size, &self.m_indices)?;

        Ok(())
    }

    fn get_neigbors(&mut self, pos: UVec3) -> VecDeque<usize> {
        let mut indcies = VecDeque::new();
        for n in get_neigbor_offsets() {
            let pos = pos.as_ivec3() + n;
            if pos.cmplt(IVec3::ZERO).any() || pos.cmpeq(self.size.as_ivec3()).any() {
                continue;
            }

            indcies.push_back(to_1d(pos.as_uvec3(), self.size))
        }

        indcies
    }

    fn collapse(&mut self, node_controller: &NodeController) -> Option<usize> {
        let index = self.prop_indices.pop_front().unwrap();
        let pos = to_3d(index as u32, self.size);
        let cell = &self.cells[index];

        if cell.id.is_some() {
            return None;
        }

        let neigbors = get_neigbor_offsets();

        let mut wave = Vec::new();

        for (id, rules) in node_controller.rules.iter() {
            let mut fits = true;

            for neigbor in neigbors.iter() {
                let rule_pos = pos.as_ivec3() + *neigbor;
                let res = self.get_cell_i(rule_pos);
                let test_cell = if res.is_err() {
                    continue;
                } else {
                    res.unwrap()
                };

                fits &= test_cell.id.is_none()
                    || if rules.contains_key(neigbor) {
                        let possible_ids = rules.get(neigbor).unwrap();
                        possible_ids.contains(&test_cell.id)
                    } else {
                        false
                    };
            }

            if fits {
                wave.push(*id)
            }
        }

        if !wave.is_empty() {
            self.cells[index].id = *fastrand::choice(wave.iter()).unwrap();

            let mut neigbors = self.get_neigbors(pos);
            self.prop_indices.append(&mut neigbors);
        }

        return Some(index);
    }

    fn print_ship(&self) {
        log::info!("Ship: ");

        let mut text = "".to_owned();
        for x in 0..self.size.x {
            text.push_str("|");
            for y in 0..self.size.y {
                let pos = uvec3(x, y, 0);
                let cell = self.get_cell(pos).unwrap();

                let mut t = "".to_owned();

                if cell.id.is_some() {
                    t.push_str(&format!(" {:?} ", cell.id.index))
                }

                if self.prop_indices.contains(&to_1d(pos, self.size)) {
                    t.push_str("p");
                }

                text.push_str(&t);

                for _ in (t.len())..8 {
                    text.push_str(" ");
                }

                text.push_str("|");
            }
            log::info!("{:?}", text);
            text.clear();
        }
    }
}

impl Cell {
    fn from_m_id(id: NodeID) -> Cell {
        Cell { m_id: id, id: id }
    }

    fn from_id(id: NodeID) -> Cell {
        Cell {
            m_id: NodeID::none(),
            id,
        }
    }

    fn none() -> Cell {
        Cell {
            m_id: NodeID::none(),
            id: NodeID::none(),
        }
    }
}

impl block_mesh::Voxel for Cell {
    fn get_visibility(&self) -> block_mesh::VoxelVisibility {
        if self.id.is_none() {
            block_mesh::VoxelVisibility::Empty
        } else {
            block_mesh::VoxelVisibility::Translucent
        }
    }
}

impl block_mesh::MergeVoxel for Cell {
    type MergeValue = Self;

    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}
