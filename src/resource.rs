use bevy::prelude::*;

use rand::Rng;
use std::collections::HashMap;

use crate::game::{Ids, Position};
use crate::item::{Item, Items};
use crate::network;
use crate::skill::{self, Skill, Skills};
use crate::templates::{ItemTemplates, ResReq, ResTemplate, ResTemplates};

pub const ORE: &str = "Ore";
pub const WOOD: &str = "Wood";
pub const STONE: &str = "Stone";
pub const WATER: &str = "Water";
pub const FOOD: &str = "Food";
pub const PLANT: &str = "Plant";

pub const INGOT: &str = "Ingot";
pub const TIMBER: &str = "Timber";
pub const BLOCK: &str = "Block";

pub const HIGH: &str = "high";
pub const AVERAGE: &str = "average";
pub const LOW: &str = "low";

#[derive(Debug, Clone)]
pub struct Resource {
    pub name: String,
    pub pos: Position,
    pub max: i32,
    pub quantity: i32,
    pub reveal: bool, //pub obj_id: Option<i32>,
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Resources(HashMap<Position, HashMap<String, Resource>>);

impl Resource {
    pub fn create(
        resource_type: String,
        quantity: i32,
        position: Position,
        resources: &mut Resources,
    ) {
        let resource = Resource {
            name: resource_type.clone(),
            pos: position,
            max: quantity,
            quantity: quantity,
            reveal: false,
        };

        if let Some(resources_on_tile) = resources.get_mut(&position) {
            resources_on_tile.insert(resource_type.clone(), resource);
        } else {
            let mut resources_on_tile = HashMap::new();

            resources_on_tile.insert(resource_type.clone(), resource);

            resources.insert(position, resources_on_tile);
        }
    }

    pub fn get_on_tile(position: Position, resources: &Resources) -> Vec<network::TileResource> {
        let mut tile_resources = Vec::new();

        if let Some(resources_on_tile) = resources.get(&position) {
            for (resource_type, resource) in &*resources_on_tile {
                if resource.reveal {
                    let tile_resource = network::TileResource {
                        name: resource_type.to_string(),
                        quantity: resource.quantity,
                    };

                    tile_resources.push(tile_resource);
                }
            }
        }

        return tile_resources;
    }

    pub fn num_unrevealed_on_tile(position: Position, resources: &Resources) -> i32 {
        let mut num_unrevealed = 0;

        if let Some(resources_on_tile) = resources.get(&position) {
            for (_resource_type, resource) in &*resources_on_tile {
                if resource.reveal != true {
                    num_unrevealed += 1;
                }
            }
        }

        return num_unrevealed;
    }

    pub fn get_by_type(
        position: Position,
        res_type: String,
        resources: &Resources,
    ) -> Vec<Resource> {
        if let Some(resources_on_tile) = resources.get(&position) {

            debug!("Restype: {:?} Resources on tile: {:?}", res_type, resources_on_tile);

            return resources_on_tile
                .clone()
                .into_values()
                .filter(|x| x.reveal == true)
                .collect();
        }

        // Return empty vector
        return Vec::new();
    }

    pub fn gather_by_type(
        obj_id: i32,
        dest_obj_id: i32,
        position: Position,
        res_type: String,
        skills: &Skills,
        mut items: &mut ResMut<Items>,
        item_templates: &ItemTemplates,
        resources: &Resources,
        res_templates: &ResTemplates,
        ids: &mut Ids,
    ) {
        //TODO move elsewhere...
        let mut rng = rand::thread_rng();

        let resources_on_tile = Resource::get_by_type(position, res_type.clone(), resources);

        println!("Resources on tile: {:?}", resources_on_tile);

        for resource in resources_on_tile.iter() {
            if let Some(res_template) = res_templates.get(&resource.name) {
                let skill_name = Resource::type_to_skill(res_type.clone());

                println!("Skill name: {:?}", skill_name);
                let mut skill_value = 0;

                if let Some(skill) = Skill::get_by_name(obj_id, skill_name, skills) {
                    println!("Skill: {:?}", skill);
                    skill_value = skill.level;
                }

                let gather_chance = Resource::gather_chance(skill_value, res_template.skill_req);

                let random_num = rng.gen::<f32>();

                if random_num < gather_chance {
                    Item::create(
                        ids.new_item_id(),
                        dest_obj_id,
                        resource.name.clone(),
                        1, //TODO should this be only 1 ?
                        item_templates,
                        &mut items
                    );
                } else {
                    trace!("No item gathered.");
                }
            } else {
                error!("Cannot find resource template for {:?}", resource.name);
            }
        }
    }

    pub fn explore(
        _obj_id: i32,
        position: Position,
        resources: &mut Resources,
        res_templates: &ResTemplates,
    ) {
        let explore_skill = 50; // TODO move to skills


        if let Some(resources_on_tile) = resources.get_mut(&position) {
            debug!("Resources on tile: {:?}", resources_on_tile);
            for (_resource_type, resource) in resources_on_tile {
                if let Some(res_template) = res_templates.get(&resource.name) {
                    let res_skill_req = res_template.skill_req;
                    let quantity_skill_req =
                        Resource::quantity_skill_req(resource.max, res_template.quantity.clone());

                    if explore_skill >= (res_skill_req + quantity_skill_req) {
                        resource.reveal = true;
                        debug!("Revealing resource: {:?}", resource);
                    }
                }
            }
        }
    }

    pub fn is_valid_type(res_type: String, pos: Position, resources: &Resources) -> bool {
        let resources_on_tile = Resource::get_by_type(pos, res_type.clone(), resources);

        if resources_on_tile.len() > 0 {
            return true;
        } else {
            return false;
        }
    }

    fn type_to_skill(restype: String) -> String {
        match restype.as_str() {
            ORE => skill::MINING.to_string(),
            WOOD => skill::WOODCUTTING.to_string(),
            STONE => skill::STONECUTTING.to_string(),
            WATER => skill::GATHERING.to_string(),
            FOOD => skill::FARMING.to_string(),
            PLANT => skill::GATHERING.to_string(),
            _ => "Invalid".to_string(),
        }
    }

    fn gather_chance(skill_value: i32, res_skill_req: i32) -> f32 {
        match (skill_value, res_skill_req) {
            (0, 0) => 0.7,
            (1, 0) => 0.2,
            (2, 0) => 0.3,
            (3, 0) => 0.4,
            (4, 0) => 0.5,
            (5, 0) => 0.6,
            (_, 0) => 1.0,

            (0, 25) => 0.00016,
            (1, 25) => 0.00032,
            (2, 25) => 0.00048,
            (3, 25) => 0.00064,
            (4, 25) => 0.00080,
            (5, 25) => 0.00096,

            (0, 50) => 0.00004,
            (1, 50) => 0.00008,
            (2, 50) => 0.00012,
            (3, 50) => 0.00016,
            (4, 50) => 0.00020,
            (5, 50) => 0.00024,

            (_, _) => 1.0,
        }
    }

    fn quantity_skill_req(max: i32, quantity_rates: Vec<i32>) -> i32 {
        let index = quantity_rates.iter().position(|&q| q == max).unwrap();

        match index {
            1 => 0,
            2 => 0,
            3 => 10,
            4 => 20,
            5 => 30,
            6 => 40,
            7 => 50,
            _ => 50,
        }
    }
}

pub struct ResourcePlugin;

impl Plugin for ResourcePlugin {
    fn build(&self, app: &mut App) {
        let mut resources = Resources(HashMap::new());

        Resource::create(
            "Valleyrun Copper Ore".to_string(),
            100,
            Position { x: 16, y: 36 },
            &mut resources,
        );

        Resource::create(
            "Quickforge Iron Ore".to_string(),
            50,
            Position { x: 16, y: 36 },
            &mut resources,
        );

        app.insert_resource(resources);
    }
}
