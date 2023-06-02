use bevy::prelude::*;
use bevy::utils::tracing::field::debug;

use std::collections::HashMap;

use crate::game::Position;
use crate::network;
use crate::templates::{SkillTemplate, SkillTemplates};

pub const CLASS_GATHERING: &str = "Gathering";
pub const CLASS_CRAFTING: &str = "Crafting";

pub const MINING: &str = "Mining";
pub const WOODCUTTING: &str = "Woodcutting";
pub const STONECUTTING: &str = "Stonecutting";
pub const GATHERING: &str = "Gathering";
pub const FARMING: &str = "Farming";

pub const NOVICE_WARRIOR: &str = "Novice Warrior";
pub const NOVICE_RANGER: &str = "Novice Ranger";
pub const NOVICE_MAGE: &str = "Novice Mage";
pub const SKILLED_WARRIOR: &str = "Skilled Warrior";
pub const SKILLED_RANGER: &str = "Skilled Warrior";
pub const SKILLED_MAGE: &str = "Skilled Mage";
pub const GREAT_WARRIOR: &str = "Great Warrior";
pub const GREAT_RANGER: &str = "Great Ranger";
pub const GREAT_MAGE: &str = "Great Mage";
pub const LEGENDARY_WARRIOR: &str = "Legendary Warrior";
pub const LEGENDARY_RANGER: &str = "Legendary Ranger";
pub const LEGENDARY_MAGE: &str = "Legendary Mage";
pub const MAX_RANK: &str = "Max Rank";

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub level: i32,
    pub xp: i32,
}

#[derive(Debug, Clone)]
pub struct SkillUpdated {
    pub id: i32,
    pub xp_type: String,
    pub xp: i32,
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Skills(HashMap<i32, HashMap<String, Skill>>);

impl Skill {
    pub fn update(
        obj_id: i32,
        skill_name: String,
        value: i32,
        skills: &mut Skills,
        skill_templates: &SkillTemplates,
    ) {
        /*
            xp = 75
            value = 200
            total xp = 275
            level 0

            =>

            xp = 0
            remaining = 175
            level 1

            =>

            xp = 175
            level 1

        */
        // Get template
        let Some(skill_template) = skill_templates.get(&skill_name.clone()) else {
            panic!("Invalid skill name {:?}, does not exist in templates.", skill_name.clone());
        };

        if let Some(obj_skills) = skills.get_mut(&obj_id) {
            if let Some(obj_skill) = obj_skills.get_mut(&skill_name) {
                Self::update_xp_level(obj_skill, value, skill_template);
            } else {
                let mut new_skill = Skill {
                    name: skill_name.clone(),
                    level: 0,
                    xp: 0,
                };

                Self::update_xp_level(&mut new_skill, value, skill_template);

                obj_skills.insert(skill_name.clone(), new_skill);
            }
        } else {
            let mut new_skill = Skill {
                name: skill_name.clone(),
                level: 0,
                xp: 0,
            };

            Self::update_xp_level(&mut new_skill, value, skill_template);

            let mut obj_skills = HashMap::new();

            obj_skills.insert(skill_name.clone(), new_skill);

            skills.insert(obj_id, obj_skills);
        }
    }

    pub fn get_total_xp(obj_id: i32, skills: &Skills, skill_templates: &SkillTemplates) -> i32 {
        let mut total_xp = 0;

        if let Some(obj_skills) = skills.get(&obj_id) {
            for (skill_name, skill) in obj_skills {
                let Some(skill_template) = skill_templates.get(&skill_name.clone()) else {
                    panic!("Invalid skill name {:?}, does not exist in templates.", skill_name.clone());
                };

                let xp_index = skill.level as usize;
                let full_xp_level_list = &skill_template.xp;
                let xp_level_list = &full_xp_level_list[0..xp_index].to_vec();
                let sum_xp_level: i32 = xp_level_list.iter().sum();
                let skill_total_xp = sum_xp_level + skill.xp;

                total_xp += skill_total_xp;
            }
        }

        return total_xp;
    }

    fn update_xp_level(skill: &mut Skill, value: i32, skill_template: &SkillTemplate) {
        let xp_level_list = &skill_template.xp;
        let mut remaining = value;

        // Calculate skill level from xp value
        while remaining > 0 {
            if let Ok(xp_index) = usize::try_from(skill.level) {
                if xp_index < xp_level_list.len() {
                    if skill.xp + remaining < xp_level_list[xp_index] {
                        skill.xp += remaining;
                        remaining = 0;
                    } else if skill.xp + value == xp_level_list[skill.level as usize] {
                        skill.xp = 0;
                        skill.level += 1;
                        remaining = 0;
                    } else {
                        let total_xp = skill.xp + remaining;
                        remaining = total_xp - xp_level_list[skill.level as usize];

                        skill.xp = 0;
                        skill.level += 1;
                    }
                } else {
                    break;
                }
            }
        }
    }

    pub fn get_by_owner(obj_id: i32, skills: &Skills) -> HashMap<String, &Skill> {
        let mut skills_map = HashMap::new();

        if let Some(obj_skills) = skills.get(&obj_id) {
            for (skill_name, skill) in obj_skills.iter() {
                skills_map.insert(skill_name.clone(), skill);
            }
        }

        return skills_map;
    }

    pub fn get_levels_by_owner(obj_id: i32, skills: &Skills) -> HashMap<String, i32> {
        let mut skills_map = HashMap::new();

        if let Some(obj_skills) = skills.get(&obj_id) {
            for (skill_name, skill) in obj_skills.iter() {
                skills_map.insert(skill_name.clone(), skill.level);
            }
        }

        return skills_map;
    }

    pub fn get_by_owner_packet(
        obj_id: i32,
        skills: &Skills,
        skill_templates: &SkillTemplates,
    ) -> HashMap<String, network::Skill> {
        let mut skills_map = HashMap::new();

        if let Some(obj_skills) = skills.get(&obj_id) {
            for (skill_name, skill) in obj_skills.iter() {
                let next_xp =
                    Self::get_next(skill_name.to_string(), skill.level + 1, skill_templates);

                let skill_data = network::Skill {
                    level: skill.level,
                    xp: skill.xp,
                    next: next_xp,
                };

                skills_map.insert(skill_name.clone(), skill_data);
            }
        }

        return skills_map;
    }

    pub fn get_by_name(obj_id: i32, skill_name: String, skills: &Skills) -> Option<Skill> {
        if let Some(obj_skills) = skills.get(&obj_id) {
            if let Some(skill) = obj_skills.get(&skill_name) {
                return Some(skill.clone());
            }
        }

        return None;
    }

    pub fn get_templates_by_class(
        class: String,
        skill_templates: &SkillTemplates,
    ) -> Vec<SkillTemplate> {
        let mut skill_template_by_class = Vec::new();

        for (_skill_name, skill_template) in skill_templates.iter() {
            if skill_template.class == class {
                skill_template_by_class.push(skill_template.clone());
            }
        }

        return skill_template_by_class;
    }

    pub fn get_next(skill_name: String, level: i32, skill_templates: &SkillTemplates) -> i32 {
        let level_usize = level as usize;

        for (_skill_name, skill_template) in skill_templates.iter() {
            if skill_template.name == skill_name {
                return skill_template.xp[level_usize];
            }
        }

        // TODO reconsider panic state
        return i32::MAX;
    }

    pub fn hero_advance(hero_template: String) -> (String, i32) {
        let (next_template, required_xp) = match hero_template.as_str() {
            NOVICE_WARRIOR => (SKILLED_WARRIOR, 10000),
            NOVICE_RANGER => (SKILLED_RANGER, 10000),
            NOVICE_MAGE => (SKILLED_MAGE, 10000),
            SKILLED_WARRIOR => (GREAT_WARRIOR, 50000),
            SKILLED_RANGER => (GREAT_RANGER, 50000),
            SKILLED_MAGE => (GREAT_MAGE, 50000),
            GREAT_WARRIOR => (LEGENDARY_WARRIOR, 1000000),
            GREAT_RANGER => (LEGENDARY_RANGER, 1000000),
            GREAT_MAGE => (LEGENDARY_MAGE, 1000000),
            LEGENDARY_WARRIOR => (MAX_RANK, -1),
            LEGENDARY_RANGER => (MAX_RANK, -1),
            LEGENDARY_MAGE => (MAX_RANK, -1),
            _ => (MAX_RANK, -1),
        };

        return (next_template.to_string(), required_xp);
    }
}

pub struct SkillPlugin;

impl Plugin for SkillPlugin {
    fn build(&self, app: &mut App) {
        let mut skills = Skills(HashMap::new());

        app.insert_resource(skills);
    }
}
