use bevy::prelude::*;

use std::collections::HashMap;

use crate::network;
use crate::templates::ItemTemplates;

pub const WATER: &str = "Water";
pub const THIRST: &str = "Thirst";

pub const WEAPON: &str = "Weapon";
pub const ARMOR: &str = "Armor";

#[derive(Debug, Clone)]
pub struct Item {
    pub id: i32,
    pub owner: i32,
    pub name: String,
    pub quantity: i32,
    pub class: String,
    pub subclass: String,
    pub image: String,
    pub weight: f32,
    pub equipped: bool,
    pub attrs: HashMap<&'static str, f32>,
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Items(Vec<Item>);

impl Item {
    pub fn new(
        id: i32,
        owner: i32,
        name: String,
        quantity: i32,
        item_templates: &ItemTemplates,
    ) -> Item {
        let mut class = "Invalid".to_string();
        let mut subclass = "Invalid".to_string();
        let mut image = "Invalid".to_string();
        let mut weight = 0.0;

        for item_template in item_templates.iter() {
            if name == item_template.name {
                class = item_template.class.clone();
                subclass = item_template.subclass.clone();
                image = item_template.image.clone();
                weight = item_template.weight * (quantity as f32);
            }
        }

        let mut attrs = HashMap::new();

        attrs.insert(THIRST, 70.0);

        Item {
            id: id,
            owner: owner,
            name: name,
            quantity: quantity,
            class: class,
            subclass: subclass,
            image: image,
            weight: weight,
            equipped: false,
            attrs: attrs,
        }
    }

    pub fn get_by_owner_packet(owner: i32, items: &ResMut<Items>) -> Vec<network::Item> {
        let mut owner_items: Vec<network::Item> = Vec::new();

        for item in items.iter() {
            if item.owner == owner {
                let item_packet = network::Item {
                    id: item.id,
                    owner: item.owner,
                    name: item.name.clone(),
                    quantity: item.quantity,
                    class: item.class.clone(),
                    subclass: item.subclass.clone(),
                    image: item.image.clone(),
                    weight: item.weight,
                    equipped: item.equipped,
                };

                owner_items.push(item_packet);
            }
        }

        return owner_items;
    }

    pub fn get_packet(item_id: i32, items: &ResMut<Items>) -> Option<network::Item> {
        for item in items.iter() {
            if item.id == item_id {
                return Some(network::Item {
                    id: item.id,
                    owner: item.owner,
                    name: item.name.clone(),
                    quantity: item.quantity,
                    class: item.class.clone(),
                    subclass: item.subclass.clone(),
                    image: item.image.clone(),
                    weight: item.weight,
                    equipped: item.equipped,
                });
            }
        }

        return None;
    }

    pub fn get_by_name_packet(item_name: String, items: &ResMut<Items>) -> Option<network::Item> {
        for item in items.iter() {
            if item.name == item_name {
                return Some(network::Item {
                    id: item.id,
                    owner: item.owner,
                    name: item.name.clone(),
                    quantity: item.quantity,
                    class: item.class.clone(),
                    subclass: item.subclass.clone(),
                    image: item.image.clone(),
                    weight: item.weight,
                    equipped: item.equipped,
                });
            }
        }

        return None;
    }

    fn find_by_class(owner: i32, class: String, items: &ResMut<Items>) -> Option<usize> {
        println!("items: {:?}", items);

        let index = items
            .iter()
            .position(|item| item.owner == owner && item.class == class);
        println!("index: {:?}", index);
        return index;
    }

    pub fn update_quantity_by_class(
        owner: i32,
        class: String,
        mod_quantity: i32,
        items: &mut ResMut<Items>,
    ) -> Option<Item> {
        if let Some(index) = Item::find_by_class(owner, class, items) {
            let mut item = &mut items[index];

            println!(
                "Item Quantity: {:?} Mod Quantity: {:?}",
                item.quantity, mod_quantity
            );
            if item.quantity >= (-1 * mod_quantity) {
                item.quantity += mod_quantity;

                if item.quantity == 0 {
                    items.swap_remove(index);
                    return None;
                } else {
                    return Some(item.clone());
                }
            } else {
                return None;
            }
        } else {
            println!("Item not found");
            return None;
        }
    }

    // TODO reconsider returning the cloned item...
    pub fn find_by_id(item_id: i32, items: &ResMut<Items>) -> Option<Item> {
        if let Some(index) = items.iter().position(|item| item.id == item_id) {
            return Some(items[index].clone());
        }

        return None;
    }

    pub fn transfer(item_id: i32, target_id: i32, items: &mut ResMut<Items>) {
        if let Some(transfer_index) = items.iter().position(|item| item.id == item_id) {

            // Immutable item to transfer
            let item_to_transfer = items[transfer_index].clone();

            if Self::can_merge(item_to_transfer.class.clone()) {
                if let Some(merged_index) = items
                    .iter()
                    .position(|item| item.owner == target_id && item.name == item_to_transfer.name)
                {

                    let mut merged_item = &mut items[merged_index];
                    merged_item.quantity += item_to_transfer.quantity;

                    items.swap_remove(transfer_index);
                } else {
                    // Have to retrieve the item to transfer again as it was immutable above
                    let transfer_item = &mut items[transfer_index];
                    transfer_item.owner = target_id;
                }
            }
        }
    }

    pub fn split(item_id: i32, quantity: i32, new_id: i32, items: &mut ResMut<Items>, item_templates: &ItemTemplates) {
        if let Some(index) = items.iter().position(|item| item.id == item_id) {

            let mut item = &mut items[index];
            item.quantity -= quantity;

            let new_item = Self::new(new_id, item.owner, item.name.clone(), quantity, item_templates);

            items.push(new_item);
        } 
    }

    pub fn find_index_by_id(item_id: i32, items: &ResMut<Items>) -> Option<usize> {
        items.iter().position(|item| item.id == item_id)
    }

    fn can_merge(item_class: String) -> bool {
        match item_class.as_str() {
            WEAPON => false,
            ARMOR => false,
            _ => true,
        }
    }
}

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        let items = Items(Vec::new());

        app.insert_resource(items);
    }
}
