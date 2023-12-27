use std::collections::{BTreeSet, BinaryHeap, VecDeque};
use std::time::Duration;

use app::anyhow::{bail, Result};
use app::glam::{ivec3, mat2, mat3, uvec3, vec2, vec3, IVec3, Mat2, UVec3, Vec4Swizzles};
use app::log;

use crate::math::{get_neigbor_offsets, to_1d, to_1d_i, to_3d};
use crate::node::{NodeController, NodeID};
use crate::rotation::Rot;

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Default, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct Cell {
    pub m_id: NodeID,
    pub id: NodeID,
}

#[derive(Debug, Clone)]
pub struct Ship {
    pub size: UVec3,
    pub max_index: isize,

    pub cells: Vec<Cell>,
    pub m_prop_indices: VecDeque<usize>,
    pub prop_indices: VecDeque<usize>,
    pub collapses_per_tick: usize,
    pub fully_collapsed: bool,
}

impl Ship {
    pub fn new(node_controller: &NodeController) -> Result<Ship> {
        let size = uvec3(100, 100, 100);

        let mut ship = Ship {
            size,
            max_index: (size.x * size.y * size.z) as isize,
            cells: vec![Cell::default(); (size.x * size.y * size.z) as usize],
            m_prop_indices: VecDeque::new(),
            prop_indices: VecDeque::new(),
            collapses_per_tick: 2,
            fully_collapsed: false,
        };

        ship.place_node(uvec3(5, 5, 5), NodeID::new(5, Rot::default()));

        Ok(ship)
    }

    pub fn tick(&mut self, node_controller: &NodeController, deltatime: Duration) -> Result<()> {
        if self.fully_collapsed {
            if deltatime < MIN_TICK_LENGTH && self.collapses_per_tick < usize::MAX / 2 {
                self.collapses_per_tick *= 2;
            } else if deltatime > MAX_TICK_LENGTH && self.collapses_per_tick > 4 {
                self.collapses_per_tick /= 2;
            }

            log::info!("{:?} {:?}", self.collapses_per_tick, deltatime);
        }

        self.fully_collapsed = true;
        for _ in 0..(self.collapses_per_tick) {
            if self.prop_indices.is_empty() {
                self.fully_collapsed = false;
                break;
            }

            self.collapse(node_controller);
        }

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

    pub fn place_node(&mut self, pos: UVec3, id: NodeID) {
        log::info!("Place: {pos:?}");

        for i in 0..self.max_index {
            let cell = &mut self.cells[i as usize];
            cell.id = cell.m_id;
        }

        let index = to_1d(pos, self.size);
        self.cells[index].id = id;
        self.cells[index].m_id = id;

        let mut neigbors = self.get_neigbors(pos);
        self.m_prop_indices.append(&mut neigbors);
        self.prop_indices = self.m_prop_indices.clone();
    }

    fn get_neigbors(&mut self, pos: UVec3) -> VecDeque<usize> {
        let mut indcies = VecDeque::new();
        for n in get_neigbor_offsets() {
            let i = to_1d_i(pos.as_ivec3() + n, self.size.as_ivec3());
            if i >= 0 && i < self.max_index {
                indcies.push_back(i as usize)
            }
        }

        indcies
    }

    fn collapse(&mut self, node_controller: &NodeController) {
        let index = self.prop_indices.pop_front().unwrap();
        let pos = to_3d(index as u32, self.size);
        let cell = &self.cells[index];

        if cell.id.index != 0
            || pos.cmpeq(UVec3::ONE).any()
            || pos.cmpge(self.size - uvec3(1, 1, 1)).any()
        {
            return;
        }

        let mut wave = Vec::new();

        for (id, rules) in node_controller.rules.iter() {
            let mut fits = true;
            for (offset, possible_ids) in rules.iter() {
                let rule_pos = (pos.as_ivec3() + *offset).as_uvec3();
                let res = self.get_cell(rule_pos);
                let test_cell = if res.is_err() {
                    continue;
                } else {
                    res.unwrap()
                };

                fits &= possible_ids.contains(&test_cell.id);
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

                if cell.id.index != 0 {
                    t.push_str(&format!(" {:?} ", cell.id))
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
            m_id: NodeID::default(),
            id,
        }
    }

    fn none() -> Cell {
        Cell {
            m_id: NodeID::default(),
            id: NodeID::default(),
        }
    }
}

impl block_mesh::Voxel for Cell {
    fn get_visibility(&self) -> block_mesh::VoxelVisibility {
        if self.id.index == 0 {
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
