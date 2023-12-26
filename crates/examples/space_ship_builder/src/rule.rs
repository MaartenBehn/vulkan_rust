use std::f32::consts::PI;

use app::{
    glam::{ivec3, mat3, vec3, IVec3, Mat3, UVec3},
    log,
};

use crate::ship::{NodeID, ID_BEAM, ID_BEAM_CON, ID_HULL, ID_MAX};

pub type RuleIndex = usize;

pub type RuleTemp = u16;
pub const TEMP_NONE: RuleTemp = 0;
pub const TMEP_ROT_X_1: RuleTemp = 1;
pub const TMEP_ROT_X_2: RuleTemp = 2;
pub const TMEP_ROT_X_3: RuleTemp = 4;
pub const TMEP_ROT_X: RuleTemp = TMEP_ROT_X_1 | TMEP_ROT_X_2 | TMEP_ROT_X_3;
pub const TMEP_ROT_Y_1: RuleTemp = 8;
pub const TMEP_ROT_Y_2: RuleTemp = 16;
pub const TMEP_ROT_Y_3: RuleTemp = 32;
pub const TMEP_ROT_Y: RuleTemp = TMEP_ROT_Y_1 | TMEP_ROT_Y_2 | TMEP_ROT_Y_3;
pub const TMEP_ROT_Z_1: RuleTemp = 64;
pub const TMEP_ROT_Z_2: RuleTemp = 128;
pub const TMEP_ROT_Z_3: RuleTemp = 256;
pub const TMEP_ROT_Z: RuleTemp = TMEP_ROT_Z_1 | TMEP_ROT_Z_2 | TMEP_ROT_Z_3;
pub const TMEP_ROT: RuleTemp = TMEP_ROT_X | TMEP_ROT_Y | TMEP_ROT_Z;
pub const TMEP_FLIP: RuleTemp = 512;
pub const TMEP_FULL: RuleTemp = TMEP_ROT | TMEP_FLIP;

#[derive(Debug, Clone)]
pub struct RuleSet {
    pub rules: Vec<Vec<Rule>>,
    pub rule_indecies: Vec<(NodeID, Vec<RuleIndex>)>,
}

#[derive(Debug, Clone)]
pub struct RuleTemplate {
    id: NodeID,
    req: Vec<(IVec3, NodeID)>,
    temp: RuleTemp,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub req: Vec<(IVec3, NodeID)>,
}

impl RuleSet {
    pub fn new() -> RuleSet {
        let rule_templates = vec![
            RuleTemplate::new(ID_BEAM_CON, vec![(ivec3(1, 0, 0), ID_BEAM)], TMEP_ROT),
            RuleTemplate::new(ID_BEAM_CON, vec![(ivec3(1, 1, 1), ID_BEAM)], TMEP_ROT),
        ];

        let mut rules = vec![Vec::new(); ID_MAX];
        for template in rule_templates.iter() {
            rules[template.id].append(&mut template.apply())
        }

        let mut rule_indecies = Vec::new();
        for (id, rules) in rules.iter().enumerate() {
            if rules.is_empty() {
                continue;
            }

            let mut indecies = Vec::new();
            for (i, _) in rules.iter().enumerate() {
                indecies.push(i)
            }

            rule_indecies.push((id, indecies))
        }

        RuleSet {
            rules,
            rule_indecies,
        }
    }
}

impl RuleTemplate {
    pub fn new(id: NodeID, req: Vec<(IVec3, NodeID)>, temp: RuleTemp) -> RuleTemplate {
        RuleTemplate { id, req, temp }
    }

    pub fn apply(&self) -> Vec<Rule> {
        let mut rules = vec![Rule {
            req: self.req.to_owned(),
        }];

        if (self.temp & TMEP_ROT_X_1) != 0 {
            rules.push(self.apply_rotation(0.5 * PI, UVec3::X))
        }

        if (self.temp & TMEP_ROT_X_2) != 0 {
            rules.push(self.apply_rotation(1.0 * PI, UVec3::X))
        }

        if (self.temp & TMEP_ROT_X_3) != 0 {
            rules.push(self.apply_rotation(1.5 * PI, UVec3::X))
        }

        if (self.temp & TMEP_ROT_Y_1) != 0 {
            rules.push(self.apply_rotation(0.5 * PI, UVec3::Y))
        }

        if (self.temp & TMEP_ROT_Y_2) != 0 {
            rules.push(self.apply_rotation(1.0 * PI, UVec3::Y))
        }

        if (self.temp & TMEP_ROT_Y_3) != 0 {
            rules.push(self.apply_rotation(1.5 * PI, UVec3::Y))
        }

        if (self.temp & TMEP_ROT_Z_1) != 0 {
            rules.push(self.apply_rotation(0.5 * PI, UVec3::Z))
        }

        if (self.temp & TMEP_ROT_Z_2) != 0 {
            rules.push(self.apply_rotation(1.0 * PI, UVec3::Z))
        }

        if (self.temp & TMEP_ROT_Z_3) != 0 {
            rules.push(self.apply_rotation(1.5 * PI, UVec3::Z))
        }

        if (self.temp & TMEP_FLIP) != 0 {
            rules.push(self.apply_flip())
        }

        let mut final_rules = Vec::new();
        for (i, rule) in rules.iter().enumerate() {
            let mut found = false;
            for j in (i + 1)..rules.len() {
                if rule.req == rules[j].req {
                    found = true;
                    break;
                }
            }

            if !found {
                final_rules.push(rule.clone())
            }
        }

        final_rules
    }

    fn apply_rotation(&self, angle: f32, axis: UVec3) -> Rule {
        let rot_mat = if axis == UVec3::X {
            mat3(
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, angle.cos(), -angle.sin()),
                vec3(0.0, angle.sin(), angle.cos()),
            )
        } else if axis == UVec3::Y {
            mat3(
                vec3(angle.cos(), 0.0, -angle.sin()),
                vec3(0.0, 1.0, 0.0),
                vec3(angle.sin(), 0.0, angle.cos()),
            )
        } else if axis == UVec3::Z {
            mat3(
                vec3(angle.cos(), -angle.sin(), 0.0),
                vec3(angle.sin(), angle.cos(), 0.0),
                vec3(0.0, 0.0, 1.0),
            )
        } else {
            log::error!("Invalid Axis");
            Mat3::IDENTITY
        };

        Rule {
            req: self
                .req
                .iter()
                .map(|(pos, id)| ((rot_mat * pos.as_vec3()).round().as_ivec3(), *id))
                .collect(),
        }
    }

    fn apply_flip(&self) -> Rule {
        Rule {
            req: self.req.iter().map(|(pos, id)| (*pos * -1, *id)).collect(),
        }
    }
}
