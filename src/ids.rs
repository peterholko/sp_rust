use bevy::prelude::*;

use std::collections::HashMap;

// Indexes for IDs
#[derive(Resource, Clone, Debug)]
pub struct Ids {
    pub map_event: i32,
    pub player_event: i32,
    pub obj: i32,
    pub item: i32,
    pub player_hero_map: HashMap<i32, i32>,
    pub obj_entity_map: HashMap<i32, Entity>,
    pub obj_player_map: HashMap<i32, i32>,
}

impl Ids {
    pub fn new_map_event_id(&mut self) -> i32 {
        self.map_event = self.map_event + 1;
        self.map_event
    }

    pub fn new_obj_id(&mut self) -> i32 {
        self.obj = self.obj + 1;
        self.obj
    }

    pub fn get_hero(&self, player_id: i32) -> Option<i32> {
        if let Some(hero_id) = self.player_hero_map.get(&player_id) {
            return Some(*hero_id);
        }

        return None;
    }

    pub fn get_entity(&self, obj_id: i32) -> Option<Entity> {
        if let Some(entity) = self.obj_entity_map.get(&obj_id) {
            return Some(*entity);
        }

        return None;
    }

    pub fn get_player(&self, obj_id: i32) -> Option<i32> {
        if let Some(player) = self.obj_player_map.get(&obj_id) {
            return Some(*player);
        }

        return None;
    }

    pub fn get_player_by_entity(&self, entity: Entity) -> Option<i32> {
        for (obj_id, e) in &self.obj_entity_map {
            if *e == entity {
                return self.get_player(*obj_id);
            }
        }

        return None;
    }

    pub fn new_obj(&mut self, obj_id: i32, player_id: i32, entity: Entity) {
        self.obj_player_map.insert(obj_id, player_id);
        self.obj_entity_map.insert(obj_id, entity);
    }

    pub fn new_hero(&mut self, hero_id: i32, player_id: i32, entity: Entity) {
        self.player_hero_map.insert(player_id, hero_id);
        self.new_obj(hero_id, player_id, entity);
    }
}