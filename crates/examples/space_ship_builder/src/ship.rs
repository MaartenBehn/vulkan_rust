use std::collections::{BTreeSet, BinaryHeap, VecDeque};
use std::time::Duration;

use app::anyhow::{bail, Result};
use app::glam::{ivec3, mat2, mat3, uvec3, vec2, vec3, IVec3, Mat2, UVec3, Vec4Swizzles};
use app::log;

use crate::math::{to_1d, to_1d_i, to_3d};
use crate::rule::{RuleIndex, RuleSet};

pub type NodeID = usize;

pub const ID_BEAM: NodeID = 1;
pub const ID_BEAM_CON: NodeID = 2;
pub const ID_HULL: NodeID = 3;
pub const ID_MAX: NodeID = 4;

pub const SHIP_TICK_LENGTH: Duration = Duration::from_millis(0);

#[derive(Debug, Clone)]
pub struct Node {
    pub m_id: NodeID,
    pub id: NodeID,
    pub m_wave: Vec<(NodeID, Vec<RuleIndex>)>,
    pub wave: Vec<(NodeID, Vec<RuleIndex>)>,
}

#[derive(Debug, Clone, Eq, Ord)]
pub struct CollapseIndex {
    index: usize,
    wave_count: usize,
}

#[derive(Debug, Clone)]
pub struct Ship {
    pub size: UVec3,
    pub nodes: Vec<Node>,
    pub prop_indices: VecDeque<usize>,
    pub prop_done_indices: Vec<usize>,
    pub collapse_indices: VecDeque<usize>,
}

impl Ship {
    pub fn new(ruleset: &RuleSet) -> Result<Ship> {
        let size = uvec3(10, 10, 1);

        let mut ship = Ship {
            size,
            nodes: vec![Node::none(); (size.x * size.y * size.z) as usize],
            prop_indices: VecDeque::new(),
            prop_done_indices: Vec::new(),
            collapse_indices: VecDeque::new(),
        };

        ship.place_node(uvec3(5, 5, 0), ID_BEAM);
        ship.place_node(uvec3(7, 5, 0), ID_BEAM);

        Ok(ship)
    }

    pub fn tick(&mut self, ruleset: &RuleSet) -> Result<()> {
        if !self.prop_indices.is_empty() {
            self.propergate(ruleset);

            if self.prop_indices.is_empty() {
                self.prop_done_indices.clear();
            }
        } else if !self.collapse_indices.is_empty() {
            self.collapse();
        } else {
            return Ok(());
        }

        self.print_ship();

        Ok(())
    }

    fn get_node(&self, pos: UVec3) -> Result<&Node> {
        if pos.cmplt(UVec3::ZERO).any() || pos.cmpge(self.size).any() {
            bail!("Index out of bounds.")
        }

        Ok(&self.nodes[to_1d(pos, self.size)])
    }

    fn place_node(&mut self, pos: UVec3, id: NodeID) {
        let index = to_1d(pos, self.size);
        self.nodes[index].id = id;
        self.nodes[index].m_id = id;
        self.add_neigbors(pos, true)
    }

    fn add_neigbors(&mut self, pos: UVec3, collapse: bool) {
        let neigbors = [
            ivec3(1, 1, 0),
            ivec3(-1, -1, 0),
            ivec3(1, 0, 0),
            ivec3(-1, 0, 0),
            ivec3(0, -1, 0),
            ivec3(1, -1, 0),
            ivec3(0, 1, 0),
            ivec3(-1, 1, 0),
        ];

        let max_size = (self.size.x * self.size.y * self.size.z) as isize;

        let mut indcies = VecDeque::new();
        for n in neigbors {
            let i = to_1d_i((pos.as_ivec3() + n), self.size.as_ivec3());
            if i >= 0 && i < max_size {
                indcies.push_back(i as usize)
            }
        }

        if collapse {
            let mut collapse_indecies = indcies.clone();

            for i in self.collapse_indices.iter() {
                let index = collapse_indecies.iter().position(|t| *t == *i);
                if index.is_some() {
                    collapse_indecies.swap_remove_back(index.unwrap());
                }
            }

            self.collapse_indices.append(&mut collapse_indecies);
        }

        for i in self.prop_indices.iter() {
            let index = indcies.iter().position(|t| *t == *i);
            if index.is_some() {
                indcies.swap_remove_back(index.unwrap());
            }
        }

        for i in self.prop_done_indices.iter() {
            let index = indcies.iter().position(|t| *t == *i);
            if index.is_some() {
                indcies.swap_remove_back(index.unwrap());
            }
        }

        self.prop_indices.append(&mut indcies);
    }

    fn propergate(&mut self, ruleset: &RuleSet) {
        let index = self.prop_indices.pop_front().unwrap();
        let pos = to_3d(index as u32, self.size);
        let node = &self.nodes[index];

        self.prop_done_indices.push(index);

        if node.id != 0 {
            return;
        }

        let rules = if node.wave.is_empty() {
            &ruleset.rule_indecies
        } else {
            &node.wave
        };

        let mut wave = Vec::new();
        let mut m_wave = Vec::new();

        for (id, rules) in rules.iter() {
            let mut fitting_rules = Vec::new();
            let mut m_fitting_rules = Vec::new();

            for rule_index in rules.iter() {
                let rule = &ruleset.rules[*id][*rule_index];
                let mut fits = true;
                let mut m_fits = true;

                for (offset, rule_id) in rule.req.iter() {
                    let rule_pos = (pos.as_ivec3() + *offset).as_uvec3();

                    let res = self.get_node(rule_pos);
                    if res.is_err() {
                        fits = false;
                        m_fits = false;
                        break;
                    }
                    let rule_node = res.unwrap();

                    fits &= rule_node.id == *rule_id
                        || (rule_node.id == 0
                            && rule_node
                                .wave
                                .iter()
                                .any(|(wave_id, _)| *wave_id == *rule_id));

                    m_fits &= rule_node.m_id == *rule_id
                        || (rule_node.m_id == 0
                            && rule_node
                                .m_wave
                                .iter()
                                .any(|(wave_id, _)| *wave_id == *rule_id));
                }

                if fits {
                    fitting_rules.push(*rule_index);
                }

                if m_fits {
                    m_fitting_rules.push(*rule_index);
                }
            }

            if !fitting_rules.is_empty() {
                wave.push((*id, fitting_rules))
            }

            if !m_fitting_rules.is_empty() {
                m_wave.push((*id, m_fitting_rules))
            }
        }

        if node.wave != wave || node.m_wave != m_wave {
            self.nodes[index].wave = wave;
            self.nodes[index].m_wave = m_wave;

            self.add_neigbors(pos, false);
        }
    }

    fn collapse(&mut self) {
        let index = self.collapse_indices.pop_front().unwrap();
        let node = &self.nodes[index];

        if node.id != 0 || node.wave.is_empty() {
            return;
        }

        self.nodes[index].id = fastrand::choice(node.wave.iter()).unwrap().0;
        self.add_neigbors(to_3d(index as u32, self.size), true);
    }

    fn print_ship(&self) {
        log::info!("Ship: ");

        let mut text = "".to_owned();
        for x in 0..self.size.x {
            text.push_str("|");
            for y in 0..self.size.y {
                let pos = uvec3(x, y, 0);
                let node = self.get_node(pos).unwrap();

                let mut t = "".to_owned();

                if node.id != 0 {
                    t.push_str(&format!(" {:?} ", node.id))
                } else {
                    for (i, _) in node.wave.iter() {
                        t.push_str(&format!("{:?}, ", i))
                    }
                }

                if self.prop_indices.contains(&to_1d(pos, self.size)) {
                    t.push_str("p");
                }
                if self.prop_done_indices.contains(&to_1d(pos, self.size)) {
                    t.push_str("d");
                }
                if self.collapse_indices.contains(&to_1d(pos, self.size)) {
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

impl Node {
    fn from_m_id(id: NodeID) -> Node {
        Node {
            m_id: id,
            id: id,
            m_wave: Vec::new(),
            wave: Vec::new(),
        }
    }

    fn from_id(id: NodeID) -> Node {
        Node {
            m_id: 0,
            id,
            m_wave: Vec::new(),
            wave: Vec::new(),
        }
    }

    fn none() -> Node {
        Node {
            m_id: 0,
            id: 0,
            m_wave: Vec::new(),
            wave: Vec::new(),
        }
    }
}

impl PartialEq for CollapseIndex {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.wave_count == other.wave_count
    }
}

impl PartialOrd for CollapseIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.wave_count.partial_cmp(&self.wave_count)
    }
}
