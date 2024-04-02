use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::slice::Iter;

use crate::effect::Effect;
use crate::network;
use crate::resource::{self};
use crate::templates::{ItemTemplate, RecipeTemplates, ResReq};


#[derive(Debug, Reflect, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttrKey {
    Damage,
    Defense,
    Feed,
    Healing,
    Thirst,
    Equipable,
    Consumable,
    DeepWoundChance,
    BleedChance,
    ConcussedChance,
    DisarmedChance,
    AllAttributes,
    Creativity,
    Dexterity,
    Endurance,
    Focus,
    Intellect,
    Spirit,
    Strength,
    Toughness,
    AxeDamage,
    SwordDamage,
    HammerDamage,
    DaggerDamage,
    SpearDamage,
    AxeSpeed,
    BowDamage,
    HeavyArmorDefense,
    HeavyArmorDurability,
    MeidumArmorDefense,
    MeidumArmorDurabilility,
    StructureHp,
    StructureDefense
}

impl AttrKey {
    pub fn proc_iter() -> Iter<'static, AttrKey> {
        static PROC_ATTR_KEYS: [AttrKey; 4] = [
            AttrKey::DeepWoundChance,
            AttrKey::BleedChance,
            AttrKey::ConcussedChance,
            AttrKey::DisarmedChance,
        ];
        PROC_ATTR_KEYS.iter()
    }

    pub fn proc_to_effect(self) -> Effect {
        match self {
            AttrKey::DeepWoundChance => Effect::DeepWound,
            AttrKey::BleedChance => Effect::Bleed,
            AttrKey::ConcussedChance => Effect::Concussed,
            AttrKey::DisarmedChance => Effect::Disarmed, 
            _ => panic!("Invalid Proc AttrKey, could not find Effect")
        }
    }

    pub fn str_to_key(val: String) -> AttrKey {
        match val.as_str() {
            "All Attributes" => AttrKey::AllAttributes,
            "Creativity" => AttrKey::Creativity,
            "Dexterity" => AttrKey::Dexterity,
            "Endurance" => AttrKey::Endurance,
            "Focus" => AttrKey::Focus,
            "Intellect" => AttrKey::Intellect,
            "Spirit" => AttrKey::Spirit,
            "Strength" => AttrKey::Strength,
            "Toughness" => AttrKey::Toughness,
            "Axe Damage" => AttrKey::AxeDamage,
            "Sword Damage" => AttrKey::SwordDamage,
            "Hammer Damage" => AttrKey::HammerDamage,
            "Dagger Damage" => AttrKey::DaggerDamage,
            "Spear Damage" => AttrKey::SpearDamage,      
            "Axe Speed" => AttrKey::AxeSpeed,
            "Bow Damage" => AttrKey::BowDamage,
            "Heavy Armor Defense" => AttrKey::HeavyArmorDefense,
            "Heavy Armor Durability" => AttrKey::HeavyArmorDurability,
            _ => AttrKey::AllAttributes
        }
    }
}

#[derive(Debug, Reflect, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttrVal {
    Num(f32),
    Bool(bool),
    Str(String),
}

pub const FILTER_ALL: &str = "all";

pub const DAMAGE: &str = "Damage";
pub const DEFENSE: &str = "Defense";

pub const WATER: &str = "Water";
pub const THIRST: &str = "Thirst";

pub const FOOD: &str = "Food";
pub const FEED: &str = "Feed";
pub const GOLD: &str = "Gold Coins";

pub const WEAPON: &str = "Weapon";
pub const ARMOR: &str = "Armor";

pub const GATHERING: &str = "Gathering";

pub const POTION: &str = "Potion";
pub const HEALTH: &str = "Health";
pub const DEED: &str = "Deed";

pub const HEALING: &str = "Healing";

pub const VISIBLE: &str = "Visble";

#[derive(Debug, Clone, PartialEq)]
pub enum ItemLocation {
    Own,
    OwnStructure,
    OtherOwnUnit,
    OtherStructure,
}

#[derive(Debug, Reflect, Clone, PartialEq)]
pub enum ExperimentItemType {
    Source,
    Reagent,
}

#[derive(Debug, Reflect, Clone, PartialEq)]
pub enum Slot {
    Invalid,
    Helm,
    Shoulder,
    Chest,
    Pants,
    Boots,
    MainHand,
}

impl Slot {
    pub fn str_to_slot(slot: String) -> Slot {
        match slot.as_str() {        
            "Helm" => Slot::Helm,
            "Shoulder" => Slot::Shoulder,
            "Chest" => Slot::Chest,
            "Pants" => Slot::Pants,
            "Boots" => Slot::Boots,
            "Main Hand" => Slot::MainHand,
            _ => {
                error!("Invalid slot: {:?}", slot);
                Slot::Invalid
            }
        }
    }

    pub fn to_str(slot: Option<Slot>) -> Option<String> {

        if let Some(slot) = slot {

            let slot_str = match slot {
                Slot::Helm => "Helm",
                Slot::Shoulder => "Shoulder",
                Slot::Chest => "Chest",
                Slot::Pants => "Pants",
                Slot::Boots => "Boots",
                Slot::MainHand => "Main Hand",
                _ => {
                    error!("Invalid slot: {:?}", slot);
                    "Invalid"
                }
            };

            return Some(slot_str.to_string());
        } else {
            return None;
        }
    }
}


#[derive(Debug, Reflect, Clone)]
pub struct Item {
    pub id: i32,
    pub owner: i32,
    pub name: String,
    pub quantity: i32,
    pub class: String,
    pub subclass: String,
    pub slot: Option<Slot>, 
    pub image: String,
    pub weight: f32,
    pub equipped: bool,
    pub experiment: Option<ExperimentItemType>,
    pub attrs: HashMap<AttrKey, AttrVal>,
}

#[derive(Resource, Reflect, Default, Debug)]
#[reflect(Resource)]
pub struct Items {
    items: Vec<Item>,
    next_id: i32,
    item_templates: Vec<ItemTemplate>,
}

impl Items {
    pub fn set_templates(&mut self, item_templates: Vec<ItemTemplate>) {
        self.item_templates = item_templates;
    }

    pub fn new(&mut self, owner: i32, name: String, quantity: i32) -> Item {
        let mut class = "Invalid".to_string();
        let mut subclass = "Invalid".to_string();
        let mut image = "Invalid".to_string();
        let mut weight = 0.0;
        let mut slot = None;

        for item_template in self.item_templates.iter() {
            if name == item_template.name {
                class = item_template.class.clone();
                subclass = item_template.subclass.clone();
                image = item_template.image.clone();
                weight = item_template.weight;
                
                if let Some(item_template_slot) = &item_template.slot {
                    slot = Some(Slot::str_to_slot(item_template_slot.to_string()));
                }
            }
        }

        let attrs = HashMap::new();

        let new_item = Item {
            id: self.get_next_id(),
            owner: owner,
            name: name,
            quantity: quantity,
            class: class,
            subclass: subclass,
            slot: slot,
            image: image,
            weight: weight,
            equipped: false,
            experiment: None,
            attrs: attrs,
        };

        self.items.push(new_item.clone());
        debug!("New Item by new(): {:?}", new_item);

        new_item
    }

    pub fn new_with_attrs(
        &mut self,
        owner: i32,
        name: String,
        quantity: i32,
        attrs: HashMap<AttrKey, AttrVal>,
    ) -> (Item, bool) {
        let mut class = "Invalid".to_string();
        let mut subclass = "Invalid".to_string();
        let mut image = "Invalid".to_string();
        let mut weight = 0.0;
        let mut slot = None;

        for item_template in self.item_templates.iter() {
            if name == item_template.name {
                class = item_template.class.clone();
                subclass = item_template.subclass.clone();
                image = item_template.image.clone();
                weight = item_template.weight;

                if let Some(item_template_slot) = &item_template.slot {
                    slot = Some(Slot::str_to_slot(item_template_slot.to_string()));
                }
            }
        }

        // Can new item be merged into existing
        if Item::can_merge_by_class(class.clone()) {
            if let Some(merged_index) = self
                .items
                .iter()
                .position(|item| item.owner == owner && item.name == name)
            {
                let merged_item = &mut self.items[merged_index];
                merged_item.quantity += quantity;

                return (merged_item.clone(), true);
            } else {
                // Create the new item
                let new_item = Item {
                    id: self.get_next_id(),
                    owner: owner,
                    name: name,
                    quantity: quantity,
                    class: class,
                    subclass: subclass,
                    slot: slot,
                    image: image,
                    weight: weight,
                    equipped: false,
                    experiment: None,
                    attrs: attrs,
                };

                self.items.push(new_item.clone());

                // Return new item to send to client
                return (new_item, false);
            }
        } else {
            // Create the new item
                // Create the new item
                let new_item = Item {
                    id: self.get_next_id(),
                    owner: owner,
                    name: name,
                    quantity: quantity,
                    class: class,
                    subclass: subclass,
                    slot: slot,
                    image: image,
                    weight: weight,
                    equipped: false,
                    experiment: None,
                    attrs: attrs,
                };

                self.items.push(new_item.clone());

            // Return new item to send to client
            return (new_item, false);
        }
    }

    pub fn create(&mut self, owner: i32, name: String, quantity: i32) -> (Item, bool) {
        let mut class = "Invalid".to_string();
        let mut subclass = "Invalid".to_string();
        let mut image = "Invalid".to_string();
        let mut weight = 0.0;

        for item_template in self.item_templates.iter() {
            if name == item_template.name {
                class = item_template.class.clone();
                subclass = item_template.subclass.clone();
                image = item_template.image.clone();
                weight = item_template.weight;
            }
        }

        // Can new item be merged into existing
        if Item::can_merge_by_class(class) {
            if let Some(merged_index) = self
                .items
                .iter()
                .position(|item| item.owner == owner && item.name == name)
            {
                let merged_item = &mut self.items[merged_index];
                merged_item.quantity += quantity;

                return (merged_item.clone(), true);
            } else {
                // Create the new item
                let new_item = self.new(owner, name, quantity);

                // Return new item to send to client
                return (new_item, false);
            }
        } else {
            // Create the new item
            let new_item = self.new(owner, name, quantity);

            // Return new item to send to client
            return (new_item, false);
        }
    }

    pub fn transfer(&mut self, item_id: i32, target_id: i32) {
        if let Some(transfer_index) = self.items.iter().position(|item| item.id == item_id) {
            // Immutable item to transfer
            let item_to_transfer = self.items[transfer_index].clone();

            if Item::can_merge_by_class(item_to_transfer.class.clone()) {
                if let Some(merged_index) = self
                    .items
                    .iter()
                    .position(|item| item.owner == target_id && item.name == item_to_transfer.name)
                {
                    let merged_item = &mut self.items[merged_index];
                    merged_item.quantity += item_to_transfer.quantity;

                    self.items.swap_remove(transfer_index);
                } else {
                    // Have to retrieve the item to transfer again as it was immutable above
                    let transfer_item = &mut self.items[transfer_index];
                    transfer_item.owner = target_id;
                }
            } else {
                let transfer_item = &mut self.items[transfer_index];
                transfer_item.owner = target_id;
            }
        }
    }

    pub fn split(&mut self, item_id: i32, quantity: i32) -> Option<Item> {
        if let Some(index) = self.items.iter().position(|item| item.id == item_id) {
            let new_item_id = self.get_next_id();
            let item = &mut self.items[index];

            if (item.quantity - quantity) > 0 {
                item.quantity -= quantity;

                /*let new_item = self.new_with_attrs(
                    item.owner,
                    item.name.clone(),
                    quantity,
                    item.attrs.clone(),
                );*/

                let mut class = "Invalid".to_string();
                let mut subclass = "Invalid".to_string();
                let mut image = "Invalid".to_string();
                let mut weight = 0.0;
                let mut slot = None;

                for item_template in self.item_templates.iter() {
                    if item.name == item_template.name {
                        class = item_template.class.clone();
                        subclass = item_template.subclass.clone();
                        image = item_template.image.clone();
                        weight = item_template.weight;

                        if let Some(item_template_slot) = &item_template.slot {
                            slot = Some(Slot::str_to_slot(item_template_slot.to_string()));
                        }                        
                    }
                }

                let new_item = Item {
                    id: new_item_id,
                    owner: item.owner,
                    name: item.name.clone(),
                    quantity: quantity,
                    class: class,
                    subclass: subclass,
                    slot: slot,
                    image: image,
                    weight: weight,
                    equipped: false,
                    experiment: None,
                    attrs: item.attrs.clone(),
                };

                self.items.push(new_item.clone());
                debug!("New Item: {:?}", new_item);

                return Some(new_item);
            } else {
                return None;
            }
        }

        return None;
    }

    pub fn transfer_quantity(&mut self, item_id: i32, target_id: i32, quantity: i32) {
        let result = self.split(item_id, quantity);

        // First call split, if successful transfer the new split item
        if let Some(new_item) = result {
            // Then transfer
            self.transfer(new_item.id, target_id);
        } else {
            // If split was not successful transfer the original item
            self.transfer(item_id, target_id);
        }
    }

    pub fn transfer_all_items(&mut self, source_id: i32, target_id: i32) {
        let source_items = self.get_by_owner(source_id);

        for source_item in source_items.iter() {
            self.transfer(source_item.id, target_id);
        }
    }

    pub fn craft(
        &mut self,
        owner: i32,
        recipe_name: String,
        quantity: i32,
        attrs: HashMap<AttrKey, AttrVal>,
        recipe_templates: &RecipeTemplates,
        custom_name: Option<String>,  //override
        custom_image: Option<String>, //override
    ) -> Item {
        // By default the recipe name is the item name
        let mut name: String = recipe_name.clone();

        let mut class = "Invalid".to_string();
        let mut subclass = "Invalid".to_string();
        let mut image = "Invalid".to_string();
        let mut weight = 0.0;
        let mut slot = None;

        for recipe_template in recipe_templates.iter() {
            if recipe_name == recipe_template.name {
                class = recipe_template.class.clone();
                subclass = recipe_template.subclass.clone();
                image = recipe_template.image.clone();
                weight = recipe_template.weight as f32 * (quantity as f32);

                if let Some(recipe_template_slot) = &recipe_template.slot {
                    slot = Some(Slot::str_to_slot(recipe_template_slot.to_string()));
                }
            }
        }

        if let Some(custom_name) = custom_name {
            name = custom_name;
        }

        if let Some(custom_image) = custom_image {
            image = custom_image;
        }


        let new_item = Item {
            id: self.get_next_id(),
            owner: owner,
            name: name,
            quantity: quantity,
            class: class,
            subclass: subclass,
            slot: slot,
            image: image,
            weight: weight,
            equipped: false,
            experiment: None,
            attrs: attrs,
        };

        self.items.push(new_item.clone());

        return new_item;
    }

    pub fn get_by_owner(&self, owner: i32) -> Vec<Item> {
        let mut owner_items: Vec<Item> = Vec::new();

        for item in self.items.iter() {
            if item.owner == owner {
                owner_items.push(item.clone());
            }
        }

        return owner_items;
    }

    pub fn get_by_class(&self, owner: i32, class: String) -> Option<Item> {
        if let Some(index) = self.find_by_class(owner, class) {
            let item = &self.items[index];
            return Some(item.clone());
        }

        return None;
    }

    pub fn get_by_owner_packet(&self, owner: i32) -> Vec<network::Item> {
        let mut owner_items: Vec<network::Item> = Vec::new();

        for item in self.items.iter() {
            if item.owner == owner {

                let item_packet = network::Item {
                    id: item.id,
                    owner: item.owner,
                    name: item.name.clone(),
                    quantity: item.quantity,
                    class: item.class.clone(),
                    subclass: item.subclass.clone(),
                    slot: Slot::to_str(item.slot.clone()),
                    image: item.image.clone(),
                    weight: item.weight,
                    equipped: item.equipped,
                    attrs: None,
                };

                owner_items.push(item_packet);
            }
        }

        return owner_items;
    }

    pub fn get_by_owner_packet_filter(
        &self,
        owner: i32,
        filter: Vec<String>,
    ) -> Vec<network::Item> {
        let mut owner_items: Vec<network::Item> = Vec::new();

        if filter.contains(&FILTER_ALL.to_string()) {
            return vec![];
        }

        for item in self.items.iter() {
            if item.owner == owner {
                if !filter.contains(&item.name) {
                    let item_packet = network::Item {
                        id: item.id,
                        owner: item.owner,
                        name: item.name.clone(),
                        quantity: item.quantity,
                        class: item.class.clone(),
                        subclass: item.subclass.clone(),
                        slot: Slot::to_str(item.slot.clone()),
                        image: item.image.clone(),
                        weight: item.weight,
                        equipped: item.equipped,
                        attrs: None,
                    };

                    owner_items.push(item_packet);
                }
            }
        }

        return owner_items;
    }

    pub fn get_packet(&self, item_id: i32) -> Option<network::Item> {
        for item in self.items.iter() {

            if item.id == item_id {
                return Some(network::Item {
                    id: item.id,
                    owner: item.owner,
                    name: item.name.clone(),
                    quantity: item.quantity,
                    class: item.class.clone(),
                    subclass: item.subclass.clone(),
                    slot: Slot::to_str(item.slot.clone()),
                    image: item.image.clone(),
                    weight: item.weight,
                    equipped: item.equipped,
                    attrs: Some(item.attrs.clone()),
                });
            }
        }

        return None;
    }

    pub fn get_by_name_packet(&self, item_name: String) -> Option<network::Item> {
        for item in self.items.iter() {
            if item.name == item_name {
                return Some(network::Item {
                    id: item.id,
                    owner: item.owner,
                    name: item.name.clone(),
                    quantity: item.quantity,
                    class: item.class.clone(),
                    subclass: item.subclass.clone(),
                    slot: Slot::to_str(item.slot.clone()),
                    image: item.image.clone(),
                    weight: item.weight,
                    equipped: item.equipped,
                    attrs: None, //TODO actually get the attrs
                });
            }
        }

        return None;
    }

    pub fn get_equipped(&self, owner: i32) -> Vec<Item> {
        let mut equipped = Vec::new();

        for item in self.items.iter() {
            if item.owner == owner && item.equipped {
                equipped.push(item.clone());
            }
        }

        return equipped;
    }

    pub fn get_equipped_weapons(&self, owner: i32) -> Vec<Item> {
        let mut equipped_weapons = Vec::new();

        for item in self.items.iter() {
            if item.owner == owner && item.class == WEAPON && item.equipped {
                equipped_weapons.push(item.clone());
            }
        }

        return equipped_weapons;
    }

    pub fn get_total_weight(&self, owner: i32) -> i32 {
        let mut total_weight = 0.0;

        for item in self.items.iter() {
            if item.owner == owner {
                total_weight += item.weight * item.quantity as f32;
            }
        }

        return total_weight as i32;
    }

    pub fn equip(&mut self, item_id: i32, status: bool) {
        for item in &mut self.items.iter_mut() {
            if item_id == item.id {
                item.equipped = status;
            }
        }
    }

    pub fn remove_quantity(&mut self, item_id: i32, quantity: i32) -> Option<Item> {
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .unwrap(); // Should panic if item is not found
        let item = &mut self.items[index];
        if item.quantity >= quantity {
            item.quantity -= quantity;

            if item.quantity == 0 {
                self.items.swap_remove(index);
                return None;
            }
        }

        return Some(item.clone());
    }

    pub fn remove_item(&mut self, item_id: i32) {
        if let Some(index) = self.items.iter().position(|item| item.id == item_id) {
            self.items.remove(index);
        } else {
            error!("Item does not exist");
        }
    }

    pub fn update_quantity_by_class(
        &mut self,
        owner: i32,
        class: String,
        mod_quantity: i32,
    ) -> Option<Item> {
        if let Some(index) = self.find_by_class(owner, class) {
            let item = &mut self.items[index];
            debug!(
                "item quantity: {:?} mod_quantity: {:?}",
                item.quantity, mod_quantity
            );
            if (item.quantity + mod_quantity) > 0 {
                item.quantity += mod_quantity;
                return Some(item.clone());
            } else if (item.quantity + mod_quantity) == 0 {
                debug!("Removing item {:?}", index);
                self.items.swap_remove(index);
                debug!("items: {:?}", self.items);
                return None;
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    pub fn set_experiment_source(&mut self, item_id: i32) -> Item {
        if let Some(index) = self.items.iter().position(|item| item.id == item_id) {
            let item = &mut self.items[index];

            item.experiment = Some(ExperimentItemType::Source);
            return item.clone();
        } else {
            panic!("Cannot find item: {:?}", item_id);
        }
    }

    pub fn remove_experiment_source(&mut self, item_id: i32) -> Item {
        if let Some(index) = self.items.iter().position(|item| item.id == item_id) {
            let item = &mut self.items[index];

            item.experiment = None;
            return item.clone();
        } else {
            panic!("Cannot find item: {:?}", item_id);
        }
    }

    pub fn set_experiment_reagent(&mut self, item_id: i32) {
        if let Some(index) = self.items.iter().position(|item| item.id == item_id) {
            let item = &mut self.items[index];

            item.experiment = Some(ExperimentItemType::Reagent);
        } else {
            error!("Cannot find item: {:?}", item_id);
        }
    }

    pub fn remove_experiment_reagent(&mut self, item_id: i32) {
        if let Some(index) = self.items.iter().position(|item| item.id == item_id) {
            let item = &mut self.items[index];

            item.experiment = None;
        } else {
            error!("Cannot find item: {:?}", item_id);
        }
    }

    pub fn get_experiment_details_packet(
        &self,
        structure_id: i32,
    ) -> (Vec<network::Item>, Vec<network::Item>, Vec<network::Item>) {
        let mut experiment_source: Vec<network::Item> = Vec::new();
        let mut experiment_reagents: Vec<network::Item> = Vec::new();
        let mut other_resources: Vec<network::Item> = Vec::new();

        for item in self.items.iter() {
            if item.owner == structure_id {
                if let Some(item_experiment_type) = &item.experiment {
                    if *item_experiment_type == ExperimentItemType::Reagent {
                        experiment_reagents.push(Item::to_packet(item.clone()));
                    } else if *item_experiment_type == ExperimentItemType::Source {
                        experiment_source.push(Item::to_packet(item.clone()));
                    }
                } else {
                    other_resources.push(Item::to_packet(item.clone()));
                }
            }
        }

        return (experiment_source, experiment_reagents, other_resources);
    }

    pub fn get_experiment_source_reagents(&self, structure_id: i32) -> (Option<Item>, Vec<Item>) {
        let mut experiment_source = None;
        let mut experiment_reagents = Vec::new();

        for item in self.items.iter() {
            if item.owner == structure_id {
                if let Some(item_experiment_type) = &item.experiment {
                    if *item_experiment_type == ExperimentItemType::Reagent {
                        experiment_reagents.push(item.clone());
                    } else if *item_experiment_type == ExperimentItemType::Source {
                        experiment_source = Some(item.clone());
                    }
                }
            }
        }

        return (experiment_source, experiment_reagents);
    }

    pub fn get_experiment_reagent(&self, structure_id: i32, subclass: String) -> Option<i32> {
        for item in self.items.iter() {
            if item.owner == structure_id
                && item.subclass == subclass
                && item.experiment == Some(ExperimentItemType::Reagent)
            {
                return Some(item.id);
            }
        }
        return None;
    }

    pub fn get_total_gold(&self, owner: i32) -> i32 {
        let mut total_gold = 0;

        for item in self.items.iter() {
            if item.owner == owner && item.class == GOLD.to_string() {
                total_gold += item.quantity;
            }
        }

        return total_gold;
    }

    pub fn transfer_gold(&mut self, owner: i32, target_id: i32, quantity: i32) {
        let mut remainder = quantity;
        let mut transfer_items = Vec::new();

        for item in &mut self.items.iter() {
            if item.owner == owner && item.class == GOLD.to_string() {
                if item.quantity >= remainder {
                    transfer_items.push((item.id, remainder));
                } else {
                    transfer_items.push((item.id, item.quantity));

                    remainder = remainder - item.quantity;
                }
            }
        }

        for (transfer_item_id, transfer_quantity) in transfer_items.iter() {
            self.transfer_quantity(*transfer_item_id, target_id, *transfer_quantity);
        }
    }

    // TODO reconsider returning the cloned item...
    pub fn find_by_id(&self, item_id: i32) -> Option<Item> {
        if let Some(index) = self.items.iter().position(|item| item.id == item_id) {
            return Some(self.items[index].clone());
        }

        return None;
    }

    pub fn find_index_by_id(&self, item_id: i32) -> Option<usize> {
        self.items.iter().position(|item| item.id == item_id)
    }

    fn find_by_class(&self, owner: i32, class: String) -> Option<usize> {
        let index = self
            .items
            .iter()
            .position(|item| item.owner == owner && item.class == class);
        return index;
    }

    fn get_next_id(&mut self) -> i32 {
       let next_id = self.next_id; 
       self.next_id += 1;
       return next_id;
    }
}

impl Item {
    pub fn to_packet(item: Item) -> network::Item {
        return network::Item {
            id: item.id,
            owner: item.owner,
            name: item.name.clone(),
            quantity: item.quantity,
            class: item.class.clone(),
            subclass: item.subclass.clone(),
            slot: Slot::to_str(item.slot),
            image: item.image.clone(),
            weight: item.weight,
            equipped: item.equipped,
            attrs: None,
        };
    }

    pub fn is_equipable(item: Item) -> bool {
        if item.class == WEAPON || item.class == ARMOR {
            return true;
        }
        return false;
    }

    pub fn use_item(_item_id: i32, _status: bool, _items: &mut ResMut<Items>) {}

    pub fn get_items_value_by_attr(attr: &AttrKey, items: Vec<Item>) -> f32 {
        let mut item_values = 0.0;

        for item in items.iter() {
            match item.attrs.get(&attr) {
                Some(item_value) => {
                    let mut val = 0.0;

                    match item_value {
                        AttrVal::Num(attr_val) => val = *attr_val,
                        _ => val = 0.0,
                    }
                    item_values += val
                }
                None => item_values += 0.0,
            }
        }

        item_values
    }

    pub fn is_req(item: Item, reqs: Vec<ResReq>) -> bool {
        for req in reqs.iter() {
            if req.req_type == item.name
                || req.req_type == item.class
                || req.req_type == item.subclass
            {
                return true;
            }
        }

        return false;
    }

    pub fn get_weight_from_template(
        item_name: String,
        item_quantity: i32,
        item_templates: &Vec<ItemTemplate>,
    ) -> i32 {
        let item_template = Item::get_template(item_name, item_templates);

        return (item_quantity as f32 * item_template.weight) as i32;
    }

    pub fn get_template(item_name: String, item_templates: &Vec<ItemTemplate>) -> &ItemTemplate {
        for item_template in item_templates.iter() {
            if item_name == item_template.name {
                return item_template;
            }
        }

        panic!("Invalid item template name {:?}", item_name);
    }

    pub fn is_resource(item: Item) -> bool {
        match item.class.as_str() {
            resource::ORE => true,
            resource::WOOD => true,
            resource::STONE => true,
            resource::INGOT => true,
            resource::TIMBER => true,
            resource::BLOCK => true,
            _ => false,
        }
    }

    fn can_merge_by_class(item_class: String) -> bool {
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
        let items = Items {
            items: Vec::new(),
            next_id: 0,
            item_templates: Vec::new(),
        };

        app.insert_resource(items);
    }
}
