use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;

use rand::distributions::Distribution;
use rand::distributions::WeightedIndex;
use rand::Rng;

use crate::ids::Ids;
use crate::game::{Position};
use crate::item::{Item, Items, self, AttrKey};
use crate::map::Map;
use crate::network;

use crate::skill::{self, Skill, Skills};
use crate::templates::{
    ItemTemplate, ResTemplate, ResTemplates, Templates,
};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Property {
    pub name: String,
    pub value: i32
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub name: String,
    pub res_type: String,
    pub pos: Position,
    pub max: i32,
    pub yield_level: i32,
    pub yield_mod: f32,
    pub quantity_level: i32,
    pub quantity: i32,
    pub properties: Vec<Property>,
    pub reveal: bool, //pub obj_id: Option<i32>,
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Resources(HashMap<Position, HashMap<String, Resource>>);


impl Resource {
    pub fn spawn_all_resources(
        resources: &mut ResMut<Resources>,
        templates: &Templates,
        map: &Res<Map>,
    ) {
        let res_templates = &templates.res_templates;
        let res_property_templates = &templates.res_property_templates;

        let mut terrain_list: HashMap<String, Vec<ResTemplate>> = HashMap::new();
        let mut rng = rand::thread_rng();

        for (_resource_name, res_template) in res_templates.iter() {
            for terrain in res_template.terrain.iter() {
                match terrain_list.entry(terrain.to_string()) {
                    Vacant(entry) => {
                        let mut res_template_list = Vec::new();
                        res_template_list.push(res_template.clone());
                        entry.insert(res_template_list);
                    }
                    Occupied(entry) => {
                        entry.into_mut().push(res_template.clone());
                    }
                };
            }
        }

        for (index, tile_info) in map.base.iter().enumerate() {
            //debug!("{}", tile_info.tile_type.to_string().as_str());

            if let Some(res_template_list) =
                terrain_list.get(tile_info.tile_type.to_string().as_str())
            {
                debug!("res_template_list: {:?}", res_template_list);
                for res_template in res_template_list.iter() {
                    // Randomize quantity
                    let dist = WeightedIndex::new(&res_template.quantity_rate).unwrap();

                    let sample = dist.sample(&mut rng);
                    let quantity = res_template.quantity[sample];
                    let quantity_level = sample as i32;

                    if quantity > 0 {
                        let pos = Map::index_to_pos(index);

                        // Randomize yield
                        let yield_dist = WeightedIndex::new(&res_template.yield_rate).unwrap();

                        let yield_sample = yield_dist.sample(&mut rng);
                        let yield_level = (yield_sample + 1) as i32;
                        let yield_mod = res_template.yield_mod[yield_sample];

                        let mut property_available_list = Vec::new();
                        let mut property_selected_list = Vec::new();

                        if let Some(properties) = &mut res_template.properties.clone()
                        {
                            let mut num_properties = 1;

                            if let Some(num) = &res_template.num_properties {
                                num_properties = *num;
                            }

                            //debug!("num_properties: {:?}", num_properties);

                            for property in properties.iter() {
                                let property_templates =
                                    res_property_templates.get(property.to_string());

                                for property_template in property_templates.iter() {
                                    let level_range =
                                        &property_template.ranges[(res_template.level - 1) as usize];
                                    
                                    let min = level_range[0];
                                    let max = level_range[1];
                                        
                                    // Generate property value and round to 2 decimal places
                                    let property_value = rng.gen_range(min..=max);

                                    let property = Property {
                                        name: property_template.name.to_string(),
                                        value: property_value
                                    };

                                    debug!("{:?}", property);

                                    property_available_list.push(property);
                                }

                                debug!("{:?}", property_available_list);

                                // let index = rng.gen_range(0..characteristics.len());
                                // let characteristic_name = &characteristics[index];
                            }

                            //debug!("property_available_list: {:?}", property_available_list);
                            for _i in 0..num_properties {
                                let index = rng.gen_range(0..property_available_list.len());
                                let selected_property = &property_available_list[index];

                                //debug!("selected_property: {:?}", selected_property);
                                property_selected_list.push(selected_property.clone());

                                property_available_list.remove(index);
                            }
                        }

                        Resource::create(
                            res_template.name.to_string(),
                            res_template.res_type.to_string(),
                            yield_level,
                            yield_mod,
                            quantity_level,
                            quantity,
                            Position { x: pos.0, y: pos.1 },
                            property_selected_list,
                            resources,
                        );
                    }
                }
            }
        }
    }

    pub fn create(
        name: String,
        res_type: String,
        yield_level: i32,
        yield_mod: f32,
        quantity_level: i32,
        quantity: i32,
        position: Position,
        characteristics: Vec<Property>,
        resources: &mut Resources,
    ) {
        let resource = Resource {
            name: name.clone(),
            res_type: res_type.clone(),
            pos: position,
            max: quantity,
            yield_level: yield_level,
            yield_mod: yield_mod,
            quantity_level: quantity_level,
            quantity: quantity,
            properties: characteristics.clone(),
            reveal: false,
        };

        if characteristics.len() > 0 {
            debug!("{:?}", resource);
        }

        if let Some(resources_on_tile) = resources.get_mut(&position) {
            resources_on_tile.insert(name.clone(), resource);
        } else {
            let mut resources_on_tile = HashMap::new();

            resources_on_tile.insert(name.clone(), resource);

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
                        color: (resource.yield_level + resource.quantity_level) / 2,
                        yield_label: Resource::yield_level_to_label(resource.yield_level),
                        quantity_label: Resource::quantity_level_to_label(resource.quantity_level),
                        properties: resource.properties.clone(),
                    };

                    tile_resources.push(tile_resource);
                }
            }
        }

        return tile_resources;
    }

    pub fn get_nearby_resources(
        center: Position,
        resources: &Resources,
    ) -> Vec<network::TileResourceWithPos> {
        let mut tile_resources = Vec::new();

        let nearby_tiles = Map::range((center.x, center.y), 5);

        for (x, y) in nearby_tiles.iter() {
            let tile = Position { x: *x, y: *y };

            if let Some(resources_on_tile) = resources.get(&tile) {
                for (resource_type, resource) in &*resources_on_tile {
                    if resource.reveal {
                        let tile_resource = network::TileResourceWithPos {
                            name: resource_type.to_string(),
                            color: (resource.yield_level + resource.quantity_level) / 2,
                            yield_label: Resource::yield_level_to_label(resource.yield_level),
                            quantity_label: Resource::quantity_level_to_label(
                                resource.quantity_level,
                            ),
                            x: *x,
                            y: *y,
                        };

                        tile_resources.push(tile_resource);
                    }
                }
            }
        }

        return tile_resources;
    }

    pub fn resource_color(yield_level: i32, quantity_level: i32) -> String {
        let total_level = (yield_level + quantity_level) / 2;

        match total_level {
            1 => "None".to_string(),
            2 => "None".to_string(),
            3 => "Green".to_string(),
            4 => "Blue".to_string(),
            5 => "Purple".to_string(),
            6 => "Orange".to_string(),
            7 => "Gold".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    pub fn yield_level_to_label(level: i32) -> String {
        match level {
            1 => "Worthless".to_string(),
            2 => "Meager".to_string(),
            3 => "Fair".to_string(),
            4 => "Outstanding".to_string(),
            5 => "Supreme".to_string(),
            6 => "Legendary".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    pub fn quantity_level_to_label(level: i32) -> String {
        match level {
            1 => "Inadequate".to_string(),
            2 => "Sparse".to_string(),
            3 => "Moderate".to_string(),
            4 => "Significant".to_string(),
            5 => "Pleantiful".to_string(),
            6 => "Immense".to_string(),
            7 => "Fabled".to_string(),
            _ => "Unknown".to_string(),
        }
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
            debug!(
                "Restype: {:?} Resources on tile: {:?}",
                res_type, resources_on_tile
            );

            return resources_on_tile
                .clone()
                .into_values()
                .filter(|x| x.reveal == true && x.res_type == res_type)
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
        capacity: i32,
        items: &mut ResMut<Items>,
        item_templates: &Vec<ItemTemplate>,
        resources: &Resources,
        res_templates: &ResTemplates,
        _ids: &mut Ids,
    ) -> Vec<network::Item> {
        let mut rng = rand::thread_rng();

        let resources_on_tile = Resource::get_by_type(position, res_type.clone(), resources);

        let mut items_to_update: Vec<network::Item> = Vec::new();

        for resource in resources_on_tile.iter() {
            if let Some(res_template) = res_templates.get(&resource.name) {
                let skill_name = Resource::type_to_skill(res_type.clone());

                let mut skill_value = 0;

                if let Some(skill) = Skill::get_by_name(obj_id, skill_name, skills) {
                    skill_value = skill.level;
                }

                let gather_chance = Resource::gather_chance(skill_value, res_template.skill_req);

                let random_num = rng.gen::<f32>();

                if random_num < gather_chance {
                    let resource_quantity = 1;

                    let current_total_weight = items.get_total_weight(dest_obj_id);

                    let new_item_weight = Item::get_weight_from_template(
                        resource.name.clone(),
                        resource_quantity,
                        &item_templates,
                    );

                    if (current_total_weight + new_item_weight) < capacity {
                        let mut item_attrs = HashMap::new();

                        let mut quality_rate = Vec::new();

                        if let Some(template_quality_rate) = &res_template.quality_rate {
                            quality_rate = template_quality_rate.clone();
                        } else {
                            quality_rate = vec![60, 30, 10];
                        }

                        // Determine quality
                        let dist = WeightedIndex::new(quality_rate).unwrap();
                        let sample = dist.sample(&mut rng);
                        let quality_level = sample as i32;

                        debug!("Quality Level: {:?}", quality_level);

                        for property in resource.properties.iter() {
                            debug!("{:?} {:?}", property.name, property.value);
                            //let characteristic_value = rng.gen_range(characteristic.min..characteristic.max);               

                            let attr_key = AttrKey::str_to_key(property.name.clone());                            

                            item_attrs.insert(attr_key, item::AttrVal::Num(property.value as f32));
                        }

                        debug!("item_attrs: {:?}", item_attrs);

                        let (new_item, _merged) = items.new_with_attrs(
                            dest_obj_id,
                            resource.name.clone(),
                            1, //TODO should this be only 1 
                            item_attrs.clone()
                        );

                        info!("Gather item created: {:?}", new_item);

                        // Convert items to be updated to packets
                        let new_item_packet = Item::to_packet(new_item);

                        items_to_update.push(new_item_packet);
                    } else {
                        info!("No inventory room for new item.")
                    }
                } else {
                    info!("No item gathered.");
                }
            } else {
                error!("Cannot find resource template for {:?}", resource.name);
            }
        }

        return items_to_update;
    }

    pub fn explore(
        _obj_id: i32,
        position: Position,
        resources: &mut Resources,
        res_templates: &ResTemplates,
    ) -> Vec<Resource> {
        let explore_skill = 50; // TODO move to skills
        let mut revealed_resources = Vec::new();

        if let Some(resources_on_tile) = resources.get_mut(&position) {
            debug!("Resources on tile: {:?}", resources_on_tile);
            for (_resource_type, resource) in resources_on_tile {
                if let Some(res_template) = res_templates.get(&resource.name) {
                    let res_skill_req = res_template.skill_req;
                    let quantity_skill_req =
                        Resource::quantity_skill_req(resource.max, res_template.quantity.clone());

                    if explore_skill >= (res_skill_req + quantity_skill_req) {
                        resource.reveal = true;
                        revealed_resources.push(resource.clone());
                        debug!("Revealing resource: {:?}", resource);
                    }
                }
            }
        }

        return revealed_resources;
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
        let resources = Resources(HashMap::new());

        app.insert_resource(resources);
    }
}
