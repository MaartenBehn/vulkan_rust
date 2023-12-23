use std::collections::BTreeSet;

use app::anyhow::{bail, Result};
use app::glam::{ivec3, mat2, mat3, uvec3, vec2, vec3, IVec3, Mat2, UVec3, Vec4Swizzles};
use app::log;

use crate::math::{to_1d, to_3d};
use crate::rule::{RuleIndex, RuleSet};

pub type NodeID = usize;

pub const ID_BEAM: NodeID = 1;
pub const ID_BEAM_CON: NodeID = 2;
pub const ID_HULL: NodeID = 3;
pub const ID_MAX: NodeID = 4;

#[derive(Debug, Clone)]
pub struct Node {
    pub m_id: NodeID,
    pub id: NodeID,
    pub m_wave: Vec<(NodeID, Vec<RuleIndex>)>,
    pub wave: Vec<(NodeID, Vec<RuleIndex>)>,
}

#[derive(Debug, Clone)]
pub struct Ship {
    pub size: UVec3,
    pub nodes: Vec<Node>,
    pub prop_indices: Vec<usize>,
    pub prop_done_indices: Vec<usize>,
    pub collap_indices: Vec<usize>,
}

impl Ship {
    pub fn new(ruleset: &RuleSet) -> Result<Ship> {
        let size = uvec3(10, 10, 1);

        let mut ship = Ship {
            size,
            nodes: vec![Node::none(); (size.x * size.y * size.z) as usize],
            prop_indices: Vec::new(),
            prop_done_indices: Vec::new(),
            collap_indices: Vec::new(),
        };

        ship.place_node(uvec3(5, 5, 0), ID_BEAM);
        ship.place_node(uvec3(7, 5, 0), ID_BEAM);

        ship.clean_prop_indices();
        ship.print_ship();

        loop {
            while ship.propergate(ruleset) {
                ship.print_ship()
            }

            while ship.collapse() {
                ship.print_ship()
            }

            if ship.prop_indices.is_empty() {
                break;
            }
        }

        Ok(ship)
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

                text.push_str(&t);

                for _ in (t.len())..5 {
                    text.push_str(" ");
                }

                if self.prop_indices.contains(&to_1d(pos, self.size)) {
                    text.push_str("x");
                } else if self.prop_done_indices.contains(&to_1d(pos, self.size)) {
                    text.push_str("o");
                } else {
                    text.push_str(" ");
                }

                text.push_str("|");
            }
            log::info!("{:?}", text);
            text.clear();
        }
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
        self.add_neigbors_to_prop(pos)
    }

    fn add_neigbors_to_prop(&mut self, pos: UVec3) {
        let neigbors = [
            ivec3(-1, -1, 0),
            ivec3(-1, 0, 0),
            ivec3(-1, 1, 0),
            ivec3(0, -1, 0),
            ivec3(0, 1, 0),
            ivec3(1, -1, 0),
            ivec3(1, 0, 0),
            ivec3(1, 1, 0),
        ];

        for neigbor in neigbors.iter() {
            let n_pos = (pos.as_ivec3() + *neigbor).as_uvec3();
            if n_pos.cmplt(UVec3::ZERO).any() || n_pos.cmpge(self.size).any() {
                continue;
            }

            self.prop_indices.push(to_1d(n_pos, self.size))
        }
    }

    fn clean_prop_indices(&mut self) {
        let set: BTreeSet<_> = self.prop_indices.drain(..).collect();
        for x in set {
            // data comes in in sorted order so you can further
            // process adjacenct elements like this
            if let Some(last) = self.prop_indices.last() {
                if *last == x {
                    continue;
                }
            }

            if !self.prop_done_indices.contains(&x) {
                self.prop_indices.push(x);
            }
        }
    }

    fn propergate(&mut self, ruleset: &RuleSet) -> bool {
        if self.prop_indices.is_empty() {
            return false;
        }

        let index = self.prop_indices.pop().unwrap();
        let pos = to_3d(index as u32, self.size);
        let node = &self.nodes[index];

        if node.id != 0 {
            return true;
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

        self.prop_done_indices.push(index);

        if node.wave != wave || node.m_wave != m_wave {
            self.nodes[index].wave = wave;
            self.nodes[index].m_wave = m_wave;

            self.add_neigbors_to_prop(pos);
            self.clean_prop_indices();
        }

        return true;
    }

    fn collapse(&mut self) -> bool {
        if self.prop_done_indices.is_empty() {
            return false;
        }

        let index = self.prop_done_indices.pop().unwrap();
        let node = &self.nodes[index];

        if node.id != 0 || node.wave.is_empty() {
            return true;
        }

        self.nodes[index].id = node.wave.first().unwrap().0;
        self.add_neigbors_to_prop(to_3d(index as u32, self.size));

        return false;
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
