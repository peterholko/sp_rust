use bevy::prelude::*;

use std::collections::HashMap;

use crate::game::Position;
use crate::network;
use crate::templates::{SkillTemplate, SkillTemplates};



#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub level: i32,
    pub xp: i32,
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Skills(HashMap<i32, HashMap<String, Skill>>);

impl Skill {
    pub const MINING: &str = "Mining";
    pub const WOODCUTTING: &str = "Woodcutting";
    pub const STONECUTTING: &str = "Stonecutting";
    pub const GATHERING: &str = "Gathering";
    pub const FARMING: &str = "Farming";

    pub fn update(
        obj_id: i32,
        skill_name: String,
        value: i32,
        skill_templates: &SkillTemplates,
        skills: &mut Skills,
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

        if let Some(obj_skills) = skills.get_mut(&obj_id) {
            if let Some(skill) = obj_skills.get_mut(&skill_name) {
                println!("Skill: {:?}", skill);
                if let Some(skill_template) = skill_templates.get(&skill_name) {
                    let xp_level_list = &skill_template.xp;

                    let mut remaining = value;

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
            }
        } else {
            let mut obj_skills = HashMap::new();

            let skill = Skill {
                name: skill_name.clone(),
                level: 0,
                xp: 0,
            };

            println!("Skill: {:?}", skill.clone());
            obj_skills.insert(skill_name.clone(), skill);


            skills.insert(obj_id, obj_skills);
        }
    }

    pub fn get_by_name(obj_id: i32, skill_name: String, skills: &Skills) -> Option<Skill> {
        if let Some(obj_skills) = skills.get(&obj_id) {
            if let Some(skill) = obj_skills.get(&skill_name) {
                return Some(skill.clone());
            }
        }

        return None;
    }
}

pub struct SkillPlugin;

impl Plugin for SkillPlugin {
    fn build(&self, app: &mut App) {
        let mut skills = Skills(HashMap::new());

        app.insert_resource(skills);
    }
}
