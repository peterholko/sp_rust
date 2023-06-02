use bevy::prelude::*;

use std::collections::HashMap;

use crate::game::StructureAttrs;
use crate::item::{Item, Items};
use crate::network;
use crate::resource::{ORE, WOOD, STONE};
use crate::templates::{ObjTemplate, ObjTemplates, ResReq};


pub const RESOURCE: &str = "resource";
pub const CRAFT: &str = "craft";

pub const MINE: &str = "Mine";
pub const LUMBERCAMP: &str = "Lumbercamp";
pub const QUARRY: &str = "Quarry";

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Deeds(Vec<Deed>);

#[derive(Debug, Clone)]
pub struct Deed {
    player_id: i32,
    structure: String,
    level: i32,
    tier: i32
}

pub struct Structure;

impl Structure {
    pub fn add_deed(player_id: i32, structure: String, level: i32, tier: i32, deeds: &mut ResMut<Deeds>,) {

    }

    pub fn available_to_build(obj_templates: &ObjTemplates) -> Vec<network::Structure> {
        let mut available_list: Vec<network::Structure> = Vec::new();

        for obj_template in obj_templates.iter() {
            if let Some(level) = obj_template.level {
                if level == 0 {
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

        return available_list;
    }

    pub fn get(name: String, obj_templates: &ObjTemplates) -> Option<ObjTemplate> {
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

        return true

    }

    pub fn consume_reqs(structure_id: i32, req_items: Vec<ResReq>, items: &mut ResMut<Items>) {

        let structure_items = Item::get_by_owner(structure_id, &items).clone();

        for req_item in req_items.iter() {
            for structure_item in structure_items.iter() {
                if req_item.req_type == structure_item.name
                    || req_item.req_type == structure_item.class
                    || req_item.req_type == structure_item.subclass
                {                    
                    Item::remove_quantity(structure_item.id, req_item.quantity, items)
                }
            }            
            
        }
    }

    pub fn process_req_items(structure_items: Vec<Item>, mut req_items: Vec<ResReq>) -> Vec<ResReq> {
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
            MINE => {
                resource = ORE
            }
            LUMBERCAMP => {
                resource = WOOD
            }
            QUARRY => {
                resource = STONE
            }
            _ => {
                resource = "unknown"
            }
        }

        return resource.to_string();
    }


}

pub struct StructurePlugin;

impl Plugin for StructurePlugin {
    fn build(&self, app: &mut App) {
        let deeds = Deeds(Vec::new());

        app.insert_resource(deeds);        
    }
}
