use bevy::prelude::*;

use std::collections::HashMap;

use rand::Rng;

use crate::game::{BaseAttrs, Order, Position};
use crate::network;
use crate::skill::{self, Skill, Skills};
use crate::templates::{SkillTemplate, SkillTemplates};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Activity {
    None,
    Following,
    Drinking,
    Eating,
    Sleeping,
    Fleeing,
    Crafting,
    Experimenting,
}

#[derive(Debug, Clone)]
pub struct Villager;

impl Villager {
    pub fn generate() {}

    pub fn generate_name() -> String {
        let names = vec![
            "Geoffry Holte",
            "Roderich Denholm",
            "Warder Folcey",
            "Andes Bardaye",
        ];

        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..names.len());

        return names[index].to_string();
    }

    pub fn generate_attributes(level: i32) -> BaseAttrs {
        let mut rng = rand::thread_rng();
        let random_range = 10 + level;

        let attrs = BaseAttrs {
            creativity: rng.gen_range(1..random_range),
            dexterity: rng.gen_range(1..random_range),
            endurance: rng.gen_range(1..random_range),
            focus: rng.gen_range(1..random_range),
            intellect: rng.gen_range(1..random_range),
            spirit: rng.gen_range(1..random_range),
            strength: rng.gen_range(1..random_range),
            toughness: rng.gen_range(1..random_range),
        };

        return attrs;
    }

    pub fn generate_skills<'a>(
        villager_id: i32,
        skills: &mut Skills,
        skill_templates: &SkillTemplates,
    ) {
        let mut pool_of_skills = Vec::new();
        let mut gathering_skills =
            Skill::get_templates_by_class(skill::CLASS_GATHERING.to_string(), skill_templates);
        let mut crafting_skills =
            Skill::get_templates_by_class(skill::CLASS_CRAFTING.to_string(), skill_templates);

        pool_of_skills.append(&mut gathering_skills);
        pool_of_skills.append(&mut crafting_skills);

        let mut rng = rand::thread_rng();

        // Generate 3 random skills
        for _i in 0..3 {
            let index = rng.gen_range(0..pool_of_skills.len());
            let selected_skill_name = pool_of_skills.remove(index).name;
            let random_xp = rng.gen_range(1..2000);

            Skill::update(
                villager_id,
                selected_skill_name,
                random_xp,
                skills,
                skill_templates,
            );
        }
    }

    pub fn order_to_speech(order: &Order) -> String {
        match order {
            Order::Follow { .. } => "Yes sir, following!".to_string(),
            Order::Explore { .. } => "Yes sir, exploring this area!".to_string(),
            Order::Gather { .. } => "Yes sir, gathering resources!".to_string(),
            Order::Refine { .. } => "Yes sir, refining resources!".to_string(),
            Order::Craft { .. } => "Yes sir, crafting a quality item for you!".to_string(),
            Order::Experiment { .. } => {
                "Yes sir, my experiments will led to discoveries!".to_string()
            }
            _ => "I'm speechless for this type of order".to_string(),
        }
    }
}
