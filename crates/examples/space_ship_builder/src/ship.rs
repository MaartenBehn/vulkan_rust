use std::collections::{BTreeSet, BinaryHeap, VecDeque};
use std::time::Duration;

use app::anyhow::{bail, Result};
use app::glam::{ivec3, mat2, mat3, uvec3, vec2, vec3, IVec3, Mat2, UVec3, Vec4Swizzles};
use app::log;

use crate::math::{to_1d, to_1d_i, to_3d};
use crate::rule::RuleSet;

pub type NodeID = usize;

pub const ID_BEAM: NodeID = 1;
pub const ID_BEAM_CON: NodeID = 2;
pub const ID_HULL: NodeID = 3;
pub const ID_MAX: NodeID = 4;

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

#[derive(Debug, Clone)]
pub struct Node {
    pub m_id: NodeID,
    pub id: NodeID,
}

#[derive(Debug, Clone, Eq, Ord)]
pub struct CollapseIndex {
    index: usize,
    wave_count: usize,
}

#[derive(Debug, Clone)]
pub struct Ship {
    pub size: UVec3,
    pub max_index: isize,

    pub nodes: Vec<Node>,
    pub m_prop_indices: VecDeque<usize>,
    pub prop_indices: VecDeque<usize>,
    pub collapses_per_tick: usize,
    pub fully_collapsed: bool,
}

impl Ship {
    pub fn new(ruleset: &RuleSet) -> Result<Ship> {
        let size = uvec3(100, 100, 100);

        let mut ship = Ship {
            size,
            max_index: (size.x * size.y * size.z) as isize,
            nodes: vec![Node::none(); (size.x * size.y * size.z) as usize],
            m_prop_indices: VecDeque::new(),
            prop_indices: VecDeque::new(),
            collapses_per_tick: 2,
            fully_collapsed: false,
        };

        ship.place_node(uvec3(5, 5, 5), ID_BEAM);

        Ok(ship)
    }

    pub fn tick(&mut self, ruleset: &RuleSet, deltatime: Duration) -> Result<()> {
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

            self.collapse(ruleset);
        }

        Ok(())
    }

    pub fn get_node(&self, pos: UVec3) -> Result<&Node> {
        self.get_node_i(pos.as_ivec3())
    }

    pub fn get_node_i(&self, pos: IVec3) -> Result<&Node> {
        let index = to_1d_i(pos, self.size.as_ivec3());

        if index < 0 || index >= self.max_index {
            bail!("Pos not in ship")
        }

        Ok(&self.nodes[index as usize])
    }

    pub fn place_node(&mut self, pos: UVec3, id: NodeID) {
        log::info!("Place: {pos:?}");

        for i in 0..self.max_index {
            let node = &mut self.nodes[i as usize];
            node.id = node.m_id;
        }

        let index = to_1d(pos, self.size);
        self.nodes[index].id = id;
        self.nodes[index].m_id = id;

        let mut neigbors = self.get_neigbors(pos);
        self.m_prop_indices.append(&mut neigbors);
        self.prop_indices = self.m_prop_indices.clone();
    }

    fn get_neigbors(&mut self, pos: UVec3) -> VecDeque<usize> {
        let neigbors = [
            ivec3(1, 1, 0),
            ivec3(-1, -1, 0),
            ivec3(1, 0, 0),
            ivec3(-1, 0, 0),
            ivec3(0, -1, 0),
            ivec3(1, -1, 0),
            ivec3(0, 1, 0),
            ivec3(-1, 1, 0),
            ivec3(1, 1, 1),
            ivec3(-1, -1, 1),
            ivec3(1, 0, 1),
            ivec3(-1, 0, 1),
            ivec3(0, -1, 1),
            ivec3(1, -1, 1),
            ivec3(0, 1, 1),
            ivec3(-1, 1, 1),
            ivec3(0, 0, 1),
            ivec3(1, 1, -1),
            ivec3(-1, -1, -1),
            ivec3(1, 0, -1),
            ivec3(-1, 0, -1),
            ivec3(0, -1, -1),
            ivec3(1, -1, -1),
            ivec3(0, 1, -1),
            ivec3(-1, 1, -1),
            ivec3(0, 0, -1),
        ];

        let mut indcies = VecDeque::new();
        for n in neigbors {
            let i = to_1d_i(pos.as_ivec3() + n, self.size.as_ivec3());
            if i >= 0 && i < self.max_index {
                indcies.push_back(i as usize)
            }
        }

        indcies
    }

    fn collapse(&mut self, ruleset: &RuleSet) {
        let index = self.prop_indices.pop_front().unwrap();
        let pos = to_3d(index as u32, self.size);
        let node = &self.nodes[index];

        if node.id != 0 {
            return;
        }

        let mut wave = Vec::new();

        for (id, rules) in ruleset.rule_indecies.iter() {
            let mut fitting_rules = Vec::new();

            for rule_index in rules.iter() {
                let rule = &ruleset.rules[*id][*rule_index];
                let mut fits = true;

                for (offset, rule_id) in rule.req.iter() {
                    let rule_pos = (pos.as_ivec3() + *offset).as_uvec3();

                    let res = self.get_node(rule_pos);
                    if res.is_err() {
                        fits = false;
                        break;
                    }
                    let rule_node = res.unwrap();

                    fits &= rule_node.id == *rule_id;
                }

                if fits {
                    fitting_rules.push(*rule_index);
                }
            }

            if !fitting_rules.is_empty() {
                wave.push(*id)
            }
        }

        if !wave.is_empty() {
            self.nodes[index].id = *fastrand::choice(wave.iter()).unwrap();

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
                let node = self.get_node(pos).unwrap();

                let mut t = "".to_owned();

                if node.id != 0 {
                    t.push_str(&format!(" {:?} ", node.id))
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

impl Node {
    fn from_m_id(id: NodeID) -> Node {
        Node { m_id: id, id: id }
    }

    fn from_id(id: NodeID) -> Node {
        Node { m_id: 0, id }
    }

    fn none() -> Node {
        Node { m_id: 0, id: 0 }
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
