use bevy::prelude::*;

use std::collections::HashMap;

use crate::game::StructureAttrs;
use crate::item::{Item, Items};
use crate::network;
use crate::resource::{ORE, STONE, WOOD};
use crate::templates::{ObjTemplate, ObjTemplates, ResReq};

pub const RESOURCE: &str = "resource";
pub const CRAFT: &str = "craft";
pub const SHELTER: &str = "shelter";

pub const MINE: &str = "Mine";
pub const LUMBERCAMP: &str = "Lumbercamp";
pub const QUARRY: &str = "Quarry";


#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Plans(Vec<Plan>);

#[derive(Debug, Clone)]
pub struct Plan {
    player_id: i32,
    structure: String,
    level: i32,
    tier: i32,
}

pub struct Structure;

impl Structure {
    pub fn add_plan(
        player_id: i32,
        structure: String,
        level: i32,
        tier: i32,
        plans: &mut ResMut<Plans>,
    ) {
        let plan = Plan {
            player_id: player_id,
            structure: structure,
            level: level,
            tier: tier,
        };

        plans.push(plan);
    }

    pub fn available_to_build(
        player_id: i32,
        plans: Vec<Plan>,
        obj_templates: &ObjTemplates,
    ) -> Vec<network::Structure> {
        let mut available_list: Vec<network::Structure> = Vec::new();

        for plan in plans.iter() {
            if player_id == plan.player_id {
                for obj_template in obj_templates.iter() {
                    if plan.structure == obj_template.name {
                        let structure = network::Structure {
                            name: obj_template.name.clone(),
                            image: str::replace(obj_template.name.as_str(), " ", "").to_lowercase(),
                            class: obj_template.class.clone(),
                            subclass: obj_template.subclass.clone(),
                            template: obj_template.template.clone(),
                            base_hp: obj_template.base_hp.unwrap_or_default(),
                            base_def: obj_template.base_def.unwrap_or_default(),
                            build_time: obj_template.build_time.unwrap_or_default(),
                            req: obj_template.req.clone().unwrap_or_default(),
                        };

                        available_list.push(structure);
                    }
                }
            }
        }

        return available_list;
    }

    pub fn get_template(template: String, obj_templates: &ObjTemplates) -> Option<ObjTemplate> {
        for obj_template in obj_templates.iter() {
            if obj_template.template == *template {
                return Some(obj_template.clone());
            }
        }

        return None;
    }

    pub fn get_template_by_name(name: String, obj_templates: &ObjTemplates) -> Option<ObjTemplate> {
        for obj_template in obj_templates.iter() {
            if obj_template.name == *name {
                return Some(obj_template.clone());
            }
        }

        return None;
    }

    pub fn has_req(structure_id: i32, req_items: &mut Vec<ResReq>, items: &ResMut<Items>) -> bool {
        let structure_items = Item::get_by_owner(structure_id, items);

        for req_item in req_items.iter_mut() {
            let mut req_quantity = req_item.quantity;

            for structure_item in structure_items.iter() {
                if req_item.req_type == structure_item.name
                    || req_item.req_type == structure_item.class
                    || req_item.req_type == structure_item.subclass
                {
                    if req_quantity - structure_item.quantity > 0 {
                        req_quantity -= structure_item.quantity;
                    } else {
                        req_quantity = 0;
                    }
                }
            }

            req_item.cquantity = Some(req_quantity);
        }

        for req_item in req_items.iter() {
            if let Some(current_req_quantity) = req_item.cquantity {
                if current_req_quantity != 0 {
                    return false;
                }
            } else {
                // If cquantity is None
                return false;
            }
        }

        return true;
    }

    pub fn consume_reqs(structure_id: i32, req_items: Vec<ResReq>, items: &mut ResMut<Items>) {
        let structure_items = Item::get_by_owner(structure_id, &items).clone();

        for req_item in req_items.iter() {
            for structure_item in structure_items.iter() {
                if req_item.req_type == structure_item.name
                    || req_item.req_type == structure_item.class
                    || req_item.req_type == structure_item.subclass
                {
                    Item::remove_quantity(structure_item.id, req_item.quantity, items);
                }
            }
        }
    }

    pub fn process_req_items(
        structure_items: Vec<Item>,
        mut req_items: Vec<ResReq>,
    ) -> Vec<ResReq> {
        // Check current required quantity from structure items
        for req_item in req_items.iter_mut() {
            let mut req_quantity = req_item.quantity;

            for structure_item in structure_items.iter() {
                if req_item.req_type == structure_item.name
                    || req_item.req_type == structure_item.class
                    || req_item.req_type == structure_item.subclass
                {
                    if req_quantity - structure_item.quantity > 0 {
                        req_quantity -= structure_item.quantity;
                    } else {
                        req_quantity = 0;
                    }
                }
            }

            req_item.cquantity = Some(req_quantity);
        }

        return req_items;
    }

    pub fn resource_type(structure_template: String) -> String {
        let mut resource = "unknown";

        match structure_template.as_str() {
            MINE => resource = ORE,
            LUMBERCAMP => resource = WOOD,
            QUARRY => resource = STONE,
            _ => resource = "unknown",
        }

        return resource.to_string();
    }
}

pub struct StructurePlugin;

impl Plugin for StructurePlugin {
    fn build(&self, app: &mut App) {
        let plans = Plans(Vec::new());

        app.insert_resource(plans);
    }
}
