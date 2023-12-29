use std::collections::{BTreeSet, BinaryHeap, HashMap, VecDeque};
use std::time::Duration;

use app::anyhow::{bail, Result};
use app::glam::{ivec3, mat2, mat3, uvec3, vec2, vec3, IVec3, Mat2, UVec3, Vec4Swizzles};
use app::log;
use app::vulkan::Context;

use crate::math::{get_neigbor_offsets, to_1d, to_1d_i, to_3d};
use crate::node::{NodeController, NodeID, RuleIndex};
use crate::rotation::Rot;
use crate::ship_mesh::ShipMesh;

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

pub struct Ship {
    pub size: UVec3,
    pub max_index: isize,

    pub cells: Vec<Cell>,
    pub m_indices: VecDeque<usize>,
    pub prop_indices: VecDeque<usize>,
    pub m_collp_indicies: VecDeque<usize>,
    pub collp_indicies: VecDeque<usize>,

    pub collapses_per_tick: usize,
    pub fully_collapsed: bool,

    pub mesh: ShipMesh,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Cell {
    pub m_id: NodeID,
    pub id: NodeID,
    pub wave: Vec<PID>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct PID {
    pub p_id: NodeID,
    pub rules: HashMap<IVec3, Vec<RuleIndex>>,
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
            m_indices: VecDeque::new(),
            prop_indices: VecDeque::new(),
            collp_indicies: VecDeque::new(),
            m_collp_indicies: VecDeque::new(),

            collapses_per_tick: 2,
            fully_collapsed: false,
            mesh,
        };

        ship.place_node(
            uvec3(5, 5, 0),
            NodeID::new(0, Rot::default()),
            node_controller,
        )?;

        Ok(ship)
    }

    pub fn tick(&mut self, node_controller: &NodeController, deltatime: Duration) -> Result<()> {
        if self.prop_indices.is_empty() && self.collp_indicies.is_empty() {
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
            if !self.prop_indices.is_empty() {
                self.propergate(node_controller);
                continue;
            }

            //self.print_ship();

            if !self.collp_indicies.is_empty() {
                let i = self.collapse();
                if i.is_some() {
                    changed_indices.push(i.unwrap())
                }
                continue;
            }

            self.fully_collapsed = false;
            break;
        }

        self.mesh.update(&self.cells, self.size, &changed_indices)?;

        Ok(())
    }

    pub fn get_cell(&self, pos: UVec3) -> Result<&Cell> {
        self.get_cell_i(pos.as_ivec3())
    }

    pub fn get_cell_i(&self, pos: IVec3) -> Result<&Cell> {
        let index = to_1d_i(pos, self.size.as_ivec3());

        if index < 0 || index >= self.max_index {
            bail!("Pos not in ship")
        }

        Ok(&self.cells[index as usize])
    }

    pub fn place_node(
        &mut self,
        pos: UVec3,
        id: NodeID,
        node_controller: &NodeController,
    ) -> Result<()> {
        log::info!("Place: {pos:?}");

        for i in 0..self.max_index {
            let cell = &mut self.cells[i as usize];
            cell.id = cell.m_id;
            if cell.m_id.is_none() {
                cell.wave = node_controller.full_wave.to_owned();
            } else {
                cell.wave.clear();
            }
        }

        let cell_index = to_1d(pos, self.size);
        self.cells[cell_index].id = id;
        self.cells[cell_index].m_id = id;
        self.m_indices.push_back(cell_index);
        self.m_indices.make_contiguous();

        self.prop_indices = self.m_indices.clone();

        let cell_pos = to_3d(cell_index as u32, self.size);
        let mut neigbors = self.get_neigbor_indices(cell_pos);
        self.m_collp_indicies.append(&mut neigbors);
        self.collp_indicies = self.m_collp_indicies.clone();

        self.mesh.reset();
        self.mesh
            .update(&self.cells, self.size, self.m_indices.as_slices().0)?;

        Ok(())
    }

    fn pos_in_bounds(&self, pos: IVec3) -> bool {
        pos.cmpge(IVec3::ZERO).all() && pos.cmplt(self.size.as_ivec3()).all()
    }

    fn get_neigbor_indices(&mut self, pos: UVec3) -> VecDeque<usize> {
        let mut indcies = VecDeque::new();
        for n in get_neigbor_offsets() {
            let pos = pos.as_ivec3() + n;
            if self.pos_in_bounds(pos) {
                indcies.push_back(to_1d(pos.as_uvec3(), self.size))
            }
        }

        indcies
    }

    fn propergate(&mut self, node_controller: &NodeController) {
        let cell_index = self.prop_indices.pop_front().unwrap();
        let cell_pos = to_3d(cell_index as u32, self.size);
        let neigbors_offsets = get_neigbor_offsets();

        for offset in neigbors_offsets {
            let neigbor_pos = cell_pos.as_ivec3() + offset;
            let inv_offset = offset * -1;
            if !self.pos_in_bounds(neigbor_pos) {
                continue;
            }

            let neigbor_index = to_1d(neigbor_pos.as_uvec3(), self.size);
            let neigbor_wave_len = self.cells[neigbor_index].wave.len();

            for i in (0..neigbor_wave_len).rev() {
                if !self.cells[neigbor_index].wave[i]
                    .rules
                    .contains_key(&inv_offset)
                {
                    continue;
                }

                let rules_len = self.cells[neigbor_index].wave[i]
                    .rules
                    .get(&inv_offset)
                    .unwrap()
                    .len();

                for j in (0..rules_len).rev() {
                    let rule_index = self.cells[neigbor_index].wave[i]
                        .rules
                        .get(&inv_offset)
                        .unwrap()[j];
                    let rule = &node_controller.rules[rule_index];

                    let mut remove = true;

                    if !rule.req.contains_key(&inv_offset) {
                        remove = false;
                    } else {
                        let cell_id = self.cells[cell_index].id;

                        let req_id = rule.req.get(&inv_offset).unwrap();

                        if cell_id.is_some() {
                            if cell_id == *req_id {
                                remove = false;
                            }
                        } else {
                            for pid in self.cells[cell_index].wave.iter() {
                                if pid.p_id == *req_id {
                                    remove = false;
                                    break;
                                }
                            }
                        }
                    }

                    if remove {
                        self.cells[neigbor_index].wave[i]
                            .rules
                            .get_mut(&inv_offset)
                            .unwrap()
                            .swap_remove(j);

                        if self.cells[neigbor_index].wave[i]
                            .rules
                            .get(&inv_offset)
                            .unwrap()
                            .is_empty()
                        {
                            self.cells[neigbor_index].wave.swap_remove(i);
                            break;
                        }
                    }
                }
            }

            if neigbor_wave_len != self.cells[neigbor_index].wave.len() {
                // Neigbor changed

                self.prop_indices.push_back(neigbor_index);
            }
        }
    }

    fn collapse(&mut self) -> Option<usize> {
        let cell_index = self.collp_indicies.pop_front().unwrap();
        let cell = &mut self.cells[cell_index];

        if cell.id.is_some() {
            return None;
        }

        if cell.wave.is_empty() {
            log::warn!("Cell could not collapse!");
            return None;
        }

        cell.id = fastrand::choice(cell.wave.iter()).unwrap().p_id;
        cell.wave.clear();

        let cell_pos = to_3d(cell_index as u32, self.size);
        let mut neigbors = self.get_neigbor_indices(cell_pos);
        self.collp_indicies.append(&mut neigbors);

        self.prop_indices.push_front(cell_index);

        return Some(cell_index);
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
                } else {
                    t.push_str("-");
                    for p_id in cell.wave.iter() {
                        t.push_str(&format!("{:?}", p_id.p_id.index));
                    }
                    t.push_str("-");
                }

                if self.prop_indices.contains(&to_1d(pos, self.size)) {
                    t.push_str("p");
                }

                if self.collp_indicies.contains(&to_1d(pos, self.size)) {
                    t.push_str("c");
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
