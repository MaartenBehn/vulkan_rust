use std::f32::consts::PI;

use app::{
    glam::{ivec3, mat3, vec3, IVec3},
    log,
};

use crate::ship::{NodeID, ID_BEAM, ID_BEAM_CON, ID_HULL, ID_MAX};

pub type RuleIndex = usize;

pub type RuleTemp = u8;
pub const TEMP_NONE: RuleTemp = 0;
pub const TMEP_ROT_1: RuleTemp = 1;
pub const TMEP_ROT_2: RuleTemp = 2;
pub const TMEP_ROT_3: RuleTemp = 4;
pub const TMEP_ROT: RuleTemp = TMEP_ROT_1 | TMEP_ROT_2 | TMEP_ROT_3;
pub const TMEP_FLIP: RuleTemp = 8;
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
            RuleTemplate::new(ID_HULL, vec![(ivec3(0, 1, 0), ID_BEAM)], TMEP_ROT),
            RuleTemplate::new(ID_HULL, vec![(ivec3(1, 1, 0), ID_BEAM)], TMEP_ROT),
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

        if (self.temp & TMEP_ROT_1) != 0 {
            rules.push(self.apply_rotation(0.5 * PI))
        }

        if (self.temp & TMEP_ROT_2) != 0 {
            rules.push(self.apply_rotation(1.0 * PI))
        }

        if (self.temp & TMEP_ROT_3) != 0 {
            rules.push(self.apply_rotation(1.5 * PI))
        }

        if (self.temp & TMEP_FLIP) != 0 {
            rules.push(self.apply_flip())
        }

        rules
    }

    fn apply_rotation(&self, angle: f32) -> Rule {
        let rot_mat = mat3(
            vec3(angle.cos(), -angle.sin(), 0.0),
            vec3(angle.sin(), angle.cos(), 0.0),
            vec3(0.0, 0.0, 1.0),
        );

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
