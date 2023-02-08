use bevy::prelude::*;

use std::collections::HashMap;

use crate::game::StructureAttrs;
use crate::item::{Item, Items};
use crate::network;
use crate::templates::{ObjTemplate, ObjTemplates, ResReq};

pub struct Structure;

impl Structure {
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

    pub fn has_req(structure_id: i32, structure_attrs: StructureAttrs, items: &ResMut<Items>) -> bool {

        let mut req_items = structure_attrs.req;
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

    pub fn consume_reqs(structure_id: i32, structure_attrs: StructureAttrs, items: &mut ResMut<Items>) {
        let mut req_items = structure_attrs.req;
        let structure_items = Item::get_by_owner(structure_id, &items).clone();

        for req_item in req_items.iter() {
            for structure_item in structure_items.iter() {
                if req_item.req_type == structure_item.name
                    || req_item.req_type == structure_item.class
                    || req_item.req_type == structure_item.subclass
                {
                    Item::remove(structure_item.id, items);
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
    


}

/*pub struct StructurePlugin;

impl Plugin for StructurePlugin {
    fn build(&self, app: &mut App) {
    }
}*/
