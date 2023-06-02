use bevy::prelude::*;

use std::collections::HashMap;

use rand::{random, Rng};

use crate::game::{BaseAttrs, Ids, Position};
use crate::item::{Item, Items};
use crate::map::TileType;
use crate::skill::{Skill, Skills};
use crate::templates::{SkillTemplate, SkillTemplates, Templates};

#[derive(Debug, Clone)]
pub struct Encounter;

#[derive(Debug, Clone)]
struct Loot {
    item_name: String,
    drop_rate: f32,
    min: i32,
    max: i32,
}

impl Encounter {
    pub fn check() {

    }

    pub fn generate_loot(
        npc_id: i32,
        mut ids: &mut ResMut<Ids>,
        mut items: &mut ResMut<Items>,
        templates: &Res<Templates>,
    ) {
        let mut rng = rand::thread_rng();

        let loot_list = Self::loot_list();

        for loot in loot_list.iter() {
            let random_num = rng.gen::<f32>();

            if loot.drop_rate > random_num {
                let item_quantity = rng.gen_range(loot.min..loot.max);

                Item::create(
                    ids.new_item_id(),
                    npc_id,
                    loot.item_name.clone(),
                    item_quantity, //TODO should this be only 1 ?
                    &templates.item_templates,
                    &mut items,
                );
            }
        }
    }

    pub fn npc_list(tile_type: TileType) -> Vec<&'static str> {
        match tile_type {
            TileType::DeciduousForest => return vec!["Spider", "Wose", "Skeleton"],
            TileType::Snow => return vec!["Wolf", "Yeti"],
            TileType::HillsSnow => return vec!["Wolf", "Yeti"],
            TileType::FrozenForest => return vec!["Wose", "Yeti", "Spider"],
            TileType::Desert => return vec!["Scorpion", "Giant Rat", "Skeleton"],
            TileType::HillsDesert => return vec!["Scorpion", "Giant Rat", "Skeleton"],
            //_ => return vec!["Giant Rat", "Wolf", "Skeleton"],
            _ => return vec!["Giant Rat"],
        }
    }

    fn loot_list() -> Vec<Loot> {
        let copper_dust = Loot {
            item_name: "Valleyrun Copper Dust".to_string(),
            drop_rate: 0.2,
            min: 1,
            max: 5,
        };

        let grape = Loot {
            item_name: "Amitanian Grape".to_string(),
            drop_rate: 0.5,
            min: 1,
            max: 3,
        };

        let training_axe = Loot {
            item_name: "Copper Training Axe".to_string(),
            drop_rate: 0.02,
            min: 1,
            max: 2,
        };

        let berries = Loot {
            item_name: "Honeybell Berries".to_string(),
            drop_rate: 0.99,
            min: 5,
            max: 10,
        };

        let mana = Loot {
            item_name: "Mana".to_string(),
            drop_rate: 0.75,
            min: 1,
            max: 3,
        };

        let coins = Loot {
            item_name: "Gold Coins".to_string(),
            drop_rate: 0.99,
            min: 1,
            max: 10,
        };

        return vec![copper_dust, grape, training_axe, berries, mana, coins];
    }
}
