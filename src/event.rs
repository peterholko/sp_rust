use bevy::prelude::*;
use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;

use uuid::Uuid;

use std::collections::HashMap;

use crate::game::Position;
use crate::effect::Effect;

#[derive(Clone, Reflect, Debug)]
pub enum VisibleEvent {
    NewObjEvent {
        new_player: bool,
    },
    RemoveObjEvent {
        pos: Position,
    },
    UpdateObjEvent {
        attr: String,
        value: String,
    },
    StateChangeEvent {
        new_state: String,
    },
    MoveEvent {
        dst_x: i32,
        dst_y: i32,
    },
    EmbarkEvent {
        transport_id: i32,
    },
    Disembark {
        pos: Position
    },
    CooldownEvent {
        duration: i32,
    },
    DamageEvent {
        target_id: i32,
        target_pos: Position,
        attack_type: String,
        damage: i32,
        combo: Option<String>,
        state: String,
    },
    EffectExpiredEvent {
        effect: Effect,
    },
    SoundObjEvent {
        sound: String,
        intensity: i32,
    },
    BuildEvent {
        builder_id: i32,
        structure_id: i32,
    },
    UpgradeEvent {
        builder_id: i32,
        structure_id: i32,
        selected_upgrade: String,
    },
    GatherEvent {
        res_type: String,
    },
    OperateEvent {
        structure_id: i32,
    },
    RefineEvent {
        structure_id: i32,
    },
    CraftEvent {
        structure_id: i32,
        recipe_name: String,
    },
    ExperimentEvent {
        structure_id: i32,
    },
    ExploreEvent,
    UseItemEvent {
        item_id: i32,
        item_owner_id: i32,
    },
    DrinkEvent {
        item_id: i32,
        obj_id: i32,
    },
    EatEvent {
        item_id: i32,
        obj_id: i32,
    },
    SleepEvent {
        obj_id: i32,
    },
    SpellRaiseDeadEvent {
        corpse_id: i32,
    },
    SpellDamageEvent {
        spell: Spell,
        target_id: i32,
    },
    NoEvent,
}

#[derive(Clone, Reflect, Debug)]
pub struct MapEvent {
    pub event_id: Uuid,
    pub obj_id: i32,
    pub run_tick: i32,
    pub event_type: VisibleEvent,
}

#[derive(Resource, Reflect, Default, Deref, DerefMut, Debug)]
#[reflect(Resource)]
pub struct MapEvents(pub HashMap<Uuid, MapEvent>);

impl MapEvents {
    pub fn new(&mut self, obj_id: i32, game_tick: i32, map_event_type: VisibleEvent) -> MapEvent {
        let map_event_id = Uuid::new_v4();

        let map_state_event = MapEvent {
            event_id: map_event_id,
            obj_id: obj_id,
            run_tick: game_tick,
            event_type: map_event_type,
        };

        self.insert(map_event_id, map_state_event.clone());

        return map_state_event;
    }
}

#[derive(Debug, Resource, Reflect, Deref, DerefMut)]
pub struct VisibleEvents(pub Vec<MapEvent>);

impl VisibleEvents {
    pub fn new(&mut self, obj_id: i32, game_tick: i32, event_type: VisibleEvent) {
        let event_id = Uuid::new_v4();

        let visible_event = MapEvent {
            event_id: event_id,
            obj_id: obj_id,
            run_tick: game_tick,
            event_type: event_type,
        };

        self.push(visible_event.clone());
    }
}

#[derive(Resource, Component, Reflect, Default, Deref, DerefMut, Debug)]
#[reflect(Resource, MapEntities)]
pub struct GameEvents(pub HashMap<i32, GameEvent>);

impl MapEntities for GameEvents {
    fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
        for (_index, game_event) in self.iter_mut() {
            match game_event.game_event_type {
                GameEventType::RemoveEntity { mut entity } => {
                    entity = entity_mapper.get_or_reserve(entity);
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Reflect, Debug)]
pub struct GameEvent {
    pub event_id: i32,
    pub run_tick: i32,
    pub game_event_type: GameEventType,
}

#[derive(Clone, Reflect, Debug)]

pub enum GameEventType {
    Login {
        player_id: i32,
    },
    SpawnNPC {
        npc_type: String,
        pos: Position,
        npc_id: Option<i32>,
    },
    NecroEvent {
        pos: Position,
    },
    RemoveEntity {
        entity: Entity,
    },
    CancelEvents {
        event_ids: Vec<uuid::Uuid>,
    },
}


#[derive(Clone, Reflect, Debug)]
pub enum Spell {
    ShadowBolt,
}

#[derive(Clone, Reflect, Debug)]
pub enum EmbarkAction {
    Embark,
    Disembark
}