use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;
use big_brain::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;

use std::collections::HashMap;

use crate::components::npc::{
    Destination, Idle, MerchantScorer, MoveToPos, SetDestination, Transport,
};
use crate::components::villager::{
    Drink, DrinkDistanceScorer, DrowsyScorer, Eat, EnemyDistanceScorer, FindDrink, FindDrinkScorer, FindFood, FindFoodScorer, FindShelter, FindShelterScorer, Flee, FoodDistanceScorer, GoodMorale, HasDrinkScorer, HasFoodScorer, Heat, Hunger, HungryScorer, IdleScorer, Morale, MoveToFoodSource, MoveToSleepPos, MoveToWaterSource, NearShelterScorer, ProcessOrder, ShelterDistanceScorer, Sleep, Thirst, ThirstyScorer, Tired, TransferDrink, TransferDrinkScorer, TransferFood, TransferFoodScorer
};
use crate::event::{GameEvent, GameEventType, GameEvents, MapEvents, VisibleEvent};
use crate::ids::Ids;

use crate::combat::{Combat, CombatQuery};
use crate::effect::Effects;
use crate::experiment::{self, Experiment, ExperimentState, Experiments};
use crate::game::{
    is_pos_empty, BaseAttrs, Class, ClassStructure, Clients, GameTick, Id, MapObjQuery, Merchant, Misc, Name, NetworkReceiver, Order, PlayerId, Position, State, Stats, StructureAttrs, Subclass, SubclassHero, SubclassVillager, Template, Viewshed, VillagerAttrs
};
use crate::item::{self, Item, Items};
use crate::map::Map;
use crate::network::{self, send_to_client, ResponsePacket, StatsData, StructureList};
use crate::obj::{self, Obj};
use crate::recipe::Recipes;
use crate::resource::{Resource, Resources};
use crate::skill::{Skill, Skills};
use crate::structure::{self, Plans, Structure};
use crate::templates::{ObjTemplate, ResReq, Templates};
use crate::terrain_feature::{TerrainFeature, TerrainFeatures};
use crate::villager::{self, Villager};

#[derive(Resource, Deref, DerefMut)]
pub struct Player(pub HashMap<i32, PlayerEvent>);

#[derive(Resource, Deref, DerefMut)]
pub struct PlayerEvents(pub HashMap<i32, PlayerEvent>);

#[derive(Resource, Clone, Debug)]
pub enum PlayerEvent {
    NewPlayer {
        player_id: i32,
        account_name: String,
        class_name: String,
    },
    Login {
        player_id: i32,
    },
    Move {
        player_id: i32,
        x: i32,
        y: i32,
    },
    Attack {
        player_id: i32,
        attack_type: String,
        source_id: i32,
        target_id: i32,
    },
    Combo {
        player_id: i32,
        source_id: i32,
        target_id: i32,
        combo_type: String,
    },
    Gather {
        player_id: i32,
        source_id: i32,
        res_type: String,
    },
    Refine {
        player_id: i32,
    },
    Craft {
        player_id: i32,
        recipe_name: String,
    },
    GetStats {
        player_id: i32,
        id: i32,
    },
    InfoObj {
        player_id: i32,
        id: i32,
    },
    InfoSkills {
        player_id: i32,
        id: i32,
    },
    InfoAttrs {
        player_id: i32,
        id: i32,
    },
    InfoAdvance {
        player_id: i32,
        id: i32,
    },
    InfoUpgrade {
        player_id: i32,
        structure_id: i32,
    },
    InfoTile {
        player_id: i32,
        x: i32,
        y: i32,
    },
    InfoTileResources {
        player_id: i32,
        x: i32,
        y: i32,
    },
    InfoInventory {
        player_id: i32,
        id: i32,
    },
    InfoItem {
        player_id: i32,
        id: i32,
        merchant_id: i32,
        merchant_action: String,
    },
    InfoItemByName {
        player_id: i32,
        name: String,
    },
    InfoItemTransfer {
        player_id: i32,
        source_id: i32,
        target_id: i32,
    },
    InfoExit {
        player_id: i32,
        id: i32,
        panel_type: String,
    },
    InfoHire {
        player_id: i32,
        source_id: i32,
    },
    ItemTransfer {
        player_id: i32,
        target_id: i32,
        item_id: i32,
    },
    ItemSplit {
        player_id: i32,
        item_id: i32,
        quantity: i32,
    },
    OrderFollow {
        player_id: i32,
        source_id: i32,
    },
    OrderGather {
        player_id: i32,
        source_id: i32,
        res_type: String,
    },
    OrderRefine {
        player_id: i32,
        structure_id: i32,
    },
    OrderCraft {
        player_id: i32,
        structure_id: i32,
        recipe_name: String,
    },
    OrderExplore {
        player_id: i32,
        villager_id: i32,
    },
    OrderExperiment {
        player_id: i32,
        structure_id: i32,
    },
    OrderPlant {
        player_id: i32,
        structure_id: i32,
    },   
    OrderTend {
        player_id: i32,
        structure_id: i32,
    },     
    OrderHarvest {
        player_id: i32,
        structure_id: i32,
    },    
    StructureList {
        player_id: i32,
    },
    CreateFoundation {
        player_id: i32,
        source_id: i32,
        structure_name: String,
    },
    Build {
        player_id: i32,
        source_id: i32,
        structure_id: i32,
    },
    Upgrade {
        player_id: i32,
        source_id: i32,
        structure_id: i32,
        selected_upgrade: String,
    },
    Survey {
        player_id: i32,
        source_id: i32,
    },
    Explore {
        player_id: i32,
    },
    NearbyResources {
        player_id: i32,
    },
    AssignList {
        player_id: i32,
    },
    Assign {
        player_id: i32,
        source_id: i32,
        target_id: i32,
    },
    Equip {
        player_id: i32,
        item_id: i32,
        status: bool,
    },
    RecipeList {
        player_id: i32,
        structure_id: i32,
    },
    Use {
        player_id: i32,
        item_id: i32,
    },
    Remove {
        player_id: i32,
        structure_id: i32,
    },
    Advance {
        player_id: i32,
        id: i32,
    },
    InfoExperinment {
        player_id: i32,
        structure_id: i32,
    },
    SetExperimentItem {
        player_id: i32,
        item_id: i32,
        is_resource: bool, //assume is source if not resource
    },
    ResetExperiment {
        player_id: i32,
        structure_id: i32,
    },
    Hire {
        player_id: i32,
        merchant_id: i32,
        target_id: i32,
    },
    BuyItem {
        player_id: i32,
        item_id: i32,
        quantity: i32,
    },
    SellItem {
        player_id: i32,
        item_id: i32,
        target_id: i32,
        quantity: i32,
    },
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ActiveInfos(pub HashMap<(i32, i32, String), bool>);

#[derive(WorldQuery)]
struct CoreQuery {
    entity: Entity,
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    name: &'static Name,
    class: &'static Class,
    subclass: &'static Subclass,
    template: &'static Template,
    state: &'static State,
    misc: &'static Misc,
    effects: &'static Effects,
}

#[derive(WorldQuery)]
struct ItemTransferQuery {
    entity: Entity,
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    name: &'static Name,
    class: &'static Class,
    subclass: &'static Subclass,
    template: &'static Template,
    state: &'static State,
    misc: &'static Misc,
    structure_attrs: Option<&'static StructureAttrs>,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
struct StructureQuery {
    entity: Entity,
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    name: &'static Name,
    class: &'static Class,
    subclass: &'static Subclass,
    template: &'static Template,
    state: &'static State,
    attrs: &'static mut StructureAttrs,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
struct VillagerQuery {
    entity: Entity,
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    name: &'static Name,
    class: &'static Class,
    subclass: &'static Subclass,
    state: &'static State,
    misc: &'static Misc,
    attrs: &'static mut VillagerAttrs,
}
#[derive(Debug, Clone, Resource, PartialEq, Serialize, Deserialize)]
pub struct StartLocation {
    name: String,
    hero_pos: Vec<i32>,
    villager_pos: Vec<i32>,
    burrow_pos: Vec<i32>,
    monolith_pos: Vec<i32>,
    shipwreck_pos: Vec<i32>,
    corpse1_pos: Vec<i32>,
    corpse2_pos: Vec<i32>,
    necromancer_pos: Vec<i32>,
    mausoleum_pos: Vec<i32>,
    merchant_pos: Vec<i32>,
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct StartLocations(Vec<StartLocation>);

impl StartLocations {
    pub fn get_start_location(&mut self) -> StartLocation {
        // Randomly select a start location
        let mut rng = rand::thread_rng();

        let start_location_index = rng.gen_range(0..self.0.len());

        // Get the start location and remove it from the list
        let start_location = self.0.remove(start_location_index);

        start_location
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // Initialize events
        let player_events: PlayerEvents = PlayerEvents(HashMap::new());
        let active_infos: ActiveInfos = ActiveInfos(HashMap::new());

        let start_file = fs::File::open("start.yaml").expect("Could not open file.");
        let start_locations =
            StartLocations(serde_yaml::from_reader(start_file).expect("Could not read values."));

        app.add_systems(
            Update,
            (
                message_broker_system,
                new_player_system,
                login_system,
                move_system,
                attack_system,
                gather_refine_system,
                info_obj_system,
                info_skills_system,
                info_attrs_system,
                info_advance_system,
            ),
        )
        .add_systems(
            Update,
            (
                info_upgrade_system,
                info_tile_system,
                info_item_system,
                info_hire_system,
                info_experiment_system,
                item_transfer_system,
                item_split_system,
                order_follow_system,
                order_gather_system,
                order_refine_system,
                order_craft_system,
                order_farm_system
            ),
        )
        .add_systems(
            Update,
            (
                order_experiment_system,
                structure_list_system,
                create_foundation_system,
                build_system,
                upgrade_system,
                explore_system,
                assign_list_system,
                assign_system,
                equip_system,
                recipe_list_system,
                order_explore_system,
                use_item_system,
                remove_system,
                set_experiment_item_system,
                hire_system,
                buy_sell_system,
            ),
        )
        .insert_resource(player_events)
        .insert_resource(active_infos)
        .insert_resource(start_locations);
    }
}

fn message_broker_system(
    client_to_game_receiver: Res<NetworkReceiver>,
    mut player_events: ResMut<PlayerEvents>,
    mut ids: ResMut<Ids>,
) {
    if let Ok(evt) = client_to_game_receiver.try_recv() {
        println!("{:?}", evt);

        player_events.insert(ids.player_event, evt.clone());

        ids.player_event += 1;
    }
}

fn new_player_system(
    mut events: ResMut<PlayerEvents>,
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut start_locations: ResMut<StartLocations>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    mut items: ResMut<Items>,
    mut skills: ResMut<Skills>,
    mut recipes: ResMut<Recipes>,
    mut plans: ResMut<Plans>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::NewPlayer {
                player_id,
                account_name,
                class_name,
            } => {
                events_to_remove.push(*event_id);

                new_player(
                    *player_id,
                    account_name.to_string(),
                    class_name.to_string(),
                    &mut commands,
                    &mut start_locations,
                    &mut ids,
                    &mut map_events,
                    &mut game_events,
                    &mut items,
                    &mut skills,
                    &mut recipes,
                    &mut plans,
                    &templates,
                    &game_tick,
                );
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(index);
    }
}

fn login_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: ResMut<GameTick>,
    mut game_events: ResMut<GameEvents>,
    mut ids: ResMut<Ids>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Login { player_id } => {
                events_to_remove.push(*event_id);

                let event_type = GameEventType::Login {
                    player_id: *player_id,
                };
                let event_id = ids.new_map_event_id();

                let event = GameEvent {
                    event_id: event_id,
                    run_tick: game_tick.0 + 4, // Add one game tick
                    game_event_type: event_type,
                };

                game_events.insert(event.event_id, event);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(index);
    }
}

fn move_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    map: Res<Map>,
    hero_query: Query<CoreQuery, With<SubclassHero>>,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Move { player_id, x, y } => {
                debug!("Move Event: {:?}", event);
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    break;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find hero entity for hero {:?}", hero_id);
                    break;
                };

                let Ok(hero) = hero_query.get(hero_entity) else {
                    error!("Cannot find hero for {:?}", hero_entity);
                    break;
                };

                if Obj::is_dead(hero.state) {
                    let error = ResponsePacket::Error {
                        errmsg: "The dead cannot move.".to_owned(),
                    };
                    send_to_client(*player_id, error, &clients);
                    continue;
                }

                if !Map::is_passable(*x, *y, &map) {
                    let error = ResponsePacket::Error {
                        errmsg: "Tile is not passable.".to_owned(),
                    };
                    send_to_client(*player_id, error, &clients);
                    continue;
                }

                if !is_pos_empty(*player_id, *x, *y, &query) {
                    let error = ResponsePacket::Error {
                        errmsg: "Tile is occupied.".to_owned(),
                    };
                    send_to_client(*player_id, error, &clients);
                    continue;
                }

                // Remove events that are cancellable
                let mut events_to_remove = Vec::new();

                // TODO move this into a function
                for (map_event_id, map_event) in map_events.iter() {
                    if map_event.obj_id == hero_id {
                        match map_event.event_type {
                            VisibleEvent::MoveEvent { .. }
                            | VisibleEvent::BuildEvent { .. }
                            | VisibleEvent::GatherEvent { .. }
                            | VisibleEvent::OperateEvent { .. }
                            | VisibleEvent::CraftEvent { .. }
                            | VisibleEvent::ExploreEvent
                            | VisibleEvent::UseItemEvent { .. } => {
                                events_to_remove.push(*map_event_id);
                            }
                            _ => {}
                        }
                    }
                }

                let event_type = GameEventType::CancelEvents {
                    event_ids: events_to_remove,
                };
                let event_id = ids.new_map_event_id();

                let event = GameEvent {
                    event_id: event_id,
                    run_tick: game_tick.0 + 1, // Add one game tick
                    game_event_type: event_type,
                };

                game_events.insert(event.event_id, event);

                // Add State Change Event to Moving
                let state_change_event = VisibleEvent::StateChangeEvent {
                    new_state: obj::STATE_MOVING.to_string(),
                };

                map_events.new(hero.id.0, game_tick.0, state_change_event);

                // Add Move Event
                let move_event = VisibleEvent::MoveEvent {
                    src: hero.pos.clone(),
                    dst: Position { x: *x, y: *y },
                };

                map_events.new(
                    hero.id.0,
                    game_tick.0 + 12, // in the future
                    move_event,
                );
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn attack_system(
    mut commands: Commands,
    mut events: ResMut<PlayerEvents>,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    mut items: ResMut<Items>,
    mut skills: ResMut<Skills>,
    templates: Res<Templates>,
    map: Res<Map>,
    mut query: Query<CombatQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Attack {
                player_id,
                attack_type,
                source_id,
                target_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(attacker_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find attacker entity from id: {:?}", source_id);
                    continue;
                };

                let Some(target_entity) = ids.get_entity(*target_id) else {
                    error!("Cannot find target entity from id: {:?}", target_id);
                    continue;
                };

                let entities = [attacker_entity, target_entity];

                let Ok([mut attacker, mut target]) = query.get_many_mut(entities) else {
                    error!(
                        "Cannot find attacker or target from entities {:?}",
                        entities
                    );
                    continue;
                };

                if Obj::is_dead(&attacker.state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot attack.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if attacker is owned by player
                if attacker.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Attacker not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Is target adjacent
                if Map::dist(*attacker.pos, *target.pos) > 1 {
                    let packet = ResponsePacket::Error {
                        errmsg: "Target is not adjacent.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if target is dead
                if *target.state == State::Dead {
                    let packet = ResponsePacket::Error {
                        errmsg: "Target is dead.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Calculate and process damage
                let (damage, combo, skill_updated) = Combat::process_attack(
                    Combat::attack_type_to_enum(attack_type.to_string()),
                    &mut attacker,
                    &mut target,
                    &mut commands,
                    &mut items,
                    &templates,
                    &map,
                    &mut ids,
                    &game_tick,
                    &mut map_events,
                );

                // Add visible damage event to broadcast to everyone nearby
                Combat::add_damage_event(
                    game_tick.0,
                    attack_type.to_string(),
                    damage,
                    combo,
                    &attacker,
                    &target,
                    &mut map_events,
                );

                // Response to client with attack response packet
                let packet = ResponsePacket::Attack {
                    sourceid: *source_id,
                    attacktype: attack_type.clone(),
                    cooldown: 5,
                    stamina_cost: 5,
                };

                send_to_client(*player_id, packet, &clients);

                debug!("Skill gain: {:?}", skill_updated);

                if let Some(skill_updated) = skill_updated {
                    Skill::update(
                        attacker.id.0,
                        skill_updated.xp_type.clone(),
                        skill_updated.xp,
                        &mut skills,
                        &templates.skill_templates,
                    );

                    let skill_updated_packet = ResponsePacket::Xp {
                        id: attacker.id.0,
                        xp_type: skill_updated.xp_type,
                        xp: skill_updated.xp,
                    };

                    send_to_client(*player_id, skill_updated_packet, &clients);
                };
            }
            PlayerEvent::Combo {
                player_id,
                source_id,
                target_id,
                combo_type: _,
            } => {
                events_to_remove.push(*event_id);

                let Some(attacker_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find attacker entity from id: {:?}", source_id);
                    continue;
                };

                let Some(target_entity) = ids.get_entity(*target_id) else {
                    error!("Cannot find target entity from id: {:?}", target_id);
                    continue;
                };

                let entities = [attacker_entity, target_entity];

                let Ok([mut attacker, mut target]) = query.get_many_mut(entities) else {
                    error!(
                        "Cannot find attacker or target from entities {:?}",
                        entities
                    );
                    continue;
                };

                if Obj::is_dead(&attacker.state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot attack.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if attacker is owned by player
                if attacker.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Attacker not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Is target adjacent
                if Map::dist(*attacker.pos, *target.pos) > 1 {
                    let packet = ResponsePacket::Error {
                        errmsg: "Target is not adjacent.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if target is dead
                if *target.state == State::Dead {
                    let packet = ResponsePacket::Error {
                        errmsg: "Target is dead.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Calculate and process damage
                let (damage, combo, skill_updated) = Combat::process_combo(
                    &mut attacker,
                    &mut target,
                    &mut commands,
                    &mut items,
                    &templates,
                    &map,
                    &mut ids,
                    &game_tick,
                    &mut map_events,
                );

                debug!("Found combo: {:?}", combo);

                // Add visible damage event to broadcast to everyone nearby
                Combat::add_damage_event(
                    game_tick.0,
                    "combo".to_string(),
                    damage,
                    combo,
                    &attacker,
                    &target,
                    &mut map_events,
                );

                // Response to client with attack response packet
                let packet = ResponsePacket::Attack {
                    sourceid: *source_id,
                    attacktype: "combo".to_string(),
                    cooldown: 5,
                    stamina_cost: 5,
                };

                send_to_client(*player_id, packet, &clients);

                debug!("Skill gain: {:?}", skill_updated);

                if let Some(skill_updated) = skill_updated {
                    Skill::update(
                        attacker.id.0,
                        skill_updated.xp_type.clone(),
                        skill_updated.xp,
                        &mut skills,
                        &templates.skill_templates,
                    );

                    let skill_updated_packet = ResponsePacket::Xp {
                        id: attacker.id.0,
                        xp_type: skill_updated.xp_type,
                        xp: skill_updated.xp,
                    };

                    send_to_client(*player_id, skill_updated_packet, &clients);
                };

                /*let Some(attacker_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find attacker entity from id: {:?}", source_id);
                    continue;
                };

                let Ok(attacker) = query.get_mut(attacker_entity) else {
                    error!("Cannot find attacker entity {:?}", attacker_entity);
                    continue;
                };

                // Check if attacker is owned by player
                if attacker.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Attacker not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                if let Some(mut combo_tracker) = attacker.combo_tracker {
                    combo_tracker.attacks.clear();
                    combo_tracker.target_id = -1;
                }*/
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn gather_refine_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: ResMut<GameTick>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    resources: Res<Resources>,
    _skills: ResMut<Skills>,
    mut items: ResMut<Items>,
    recipes: Res<Recipes>,
    hero_query: Query<CoreQuery, With<SubclassHero>>,
    structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Gather {
                player_id,
                source_id: _,
                res_type,
            } => {
                debug!("PlayerEvent::Gather");
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    continue;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find hero entity for hero {:?}", hero_id);
                    continue;
                };

                let Ok(hero) = hero_query.get(hero_entity) else {
                    error!("Cannot find hero for {:?}", hero_entity);
                    continue;
                };

                if Obj::is_dead(&hero.state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot gather.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let gather_event = VisibleEvent::GatherEvent {
                    res_type: res_type.clone(),
                };

                /*Skill::update(
                    obj_id.0,
                    "Mining".to_string(),
                    100,
                    &templates.skill_templates,
                    &mut skills,
                );*/

                map_events.new(
                    hero.id.0,
                    game_tick.0 + 8, // in the future
                    gather_event,
                );

                let packet = ResponsePacket::Gather { gather_time: 8 };
                send_to_client(*player_id, packet, &clients);
            }
            PlayerEvent::NearbyResources { player_id } => {
                debug!("PlayerEvent::NearbyResources");
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    continue;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find hero entity for hero {:?}", hero_id);
                    continue;
                };

                let Ok(hero) = hero_query.get(hero_entity) else {
                    error!("Cannot find hero for {:?}", hero_entity);
                    continue;
                };

                let nearby_resources = Resource::get_nearby_resources(*hero.pos, &resources);

                let nearby_resources_packet = ResponsePacket::NearbyResources {
                    data: nearby_resources,
                };

                send_to_client(*player_id, nearby_resources_packet, &clients);
            }
            PlayerEvent::Refine { player_id } => {
                debug!("PlayerEvent::Refine");
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    continue;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find hero entity for hero {:?}", hero_id);
                    continue;
                };

                let Ok(hero) = hero_query.get(hero_entity) else {
                    error!("Cannot find hero for {:?}", hero_entity);
                    continue;
                };

                if Obj::is_dead(&hero.state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot refine.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let mut refining_structure = None;

                // Get structure from hero position
                for structure in structure_query.iter() {
                    if structure.pos.x == hero.pos.x && structure.pos.y == hero.pos.y {
                        refining_structure = Some(structure);
                    }
                }

                let Some(refining_structure) = refining_structure else {
                    error!("No structure available to refine. {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "No structure available to refine.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                };

                if refining_structure.player_id.0 != *player_id {
                    error!("Structure not owned by player {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure not owned by player".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let refine_event = VisibleEvent::RefineEvent {
                    structure_id: refining_structure.id.0,
                };

                /*Skill::update(
                    obj_id.0,
                    "Mining".to_string(),
                    100,
                    &templates.skill_templates,
                    &mut skills,
                );*/

                map_events.new(
                    hero.id.0,
                    game_tick.0 + 8, // in the future
                    refine_event,
                );
            }
            PlayerEvent::Craft {
                player_id,
                recipe_name,
            } => {
                debug!("PlayerEvent::Craft");
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    continue;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find hero entity for hero {:?}", hero_id);
                    continue;
                };

                let Ok(hero) = hero_query.get(hero_entity) else {
                    error!("Cannot find hero for {:?}", hero_entity);
                    continue;
                };

                if Obj::is_dead(&hero.state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot craft.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let Some(mut recipe) = recipes.get_by_name(recipe_name.clone()) else {
                    error!("Invalid recipe name {:?}", *recipe_name);
                    let packet = ResponsePacket::Error {
                        errmsg: "Invalid recipe".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                };

                let mut crafting_structure = None;

                // Get structure from hero position
                for structure in structure_query.iter() {
                    if structure.pos.x == hero.pos.x && structure.pos.y == hero.pos.y {
                        crafting_structure = Some(structure);
                    }
                }

                let Some(crafting_structure) = crafting_structure else {
                    error!("No structure available to craft. {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "No structure available to craft.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                };

                if crafting_structure.player_id.0 != *player_id {
                    error!("Structure not owned by player {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure not owned by player".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                if !Structure::has_req(crafting_structure.id.0, &mut recipe.req, &mut items) {
                    error!("Insufficient resources to craft {:?}", *recipe_name);
                    let packet = ResponsePacket::Error {
                        errmsg: "Insufficient resources to craft".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                //Crafting state change
                let state_change_event = VisibleEvent::StateChangeEvent {
                    new_state: obj::STATE_CRAFTING.to_string(),
                };

                map_events.new(
                    hero.id.0,
                    game_tick.0 + 1, // in the future
                    state_change_event,
                );

                let craft_event = VisibleEvent::CraftEvent {
                    structure_id: crafting_structure.id.0,
                    recipe_name: recipe_name.clone(),
                };

                map_events.new(
                    hero.id.0,
                    game_tick.0 + 100, // in the future
                    craft_event,
                );
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_obj_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: ResMut<GameTick>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    items: ResMut<Items>,
    skills: Res<Skills>,
    templates: Res<Templates>,
    mut active_infos: ResMut<ActiveInfos>,
    query: Query<CoreQuery>,
    attrs_query: Query<&BaseAttrs>,
    stats_query: Query<&Stats>,
    structure_query: Query<&StructureAttrs>,
    villager_query: Query<&VillagerAttrs>,
) {


    
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::GetStats { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    break;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find obj for {:?}", entity);
                    break;
                };

                if obj.player_id.0 != *player_id {
                    // Silent error
                    error!("GetStats request for object not owned by player.");
                    continue;
                };

                if let Ok(stats) = stats_query.get(obj.entity) {
                    let packet = ResponsePacket::Stats {
                        data: StatsData {
                            id: *id,
                            hp: stats.hp,
                            base_hp: stats.base_hp,
                            stamina: stats.stamina.unwrap_or(100),
                            base_stamina: stats.base_stamina.unwrap_or(100),
                            effects: Vec::new(),
                        },
                    };

                    send_to_client(*player_id, packet, &clients);
                }
            }
            PlayerEvent::InfoObj { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    break;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find obj for {:?}", entity);
                    break;
                };

                let mut response_packet = ResponsePacket::None;

                if obj.player_id.0 == *player_id {
                    if obj.class.0 == obj::CLASS_UNIT {
                        let items_packet = Some(items.get_by_owner_packet(*id));
                        let skills_packet = Some(Skill::get_levels_by_owner(*id, &skills));

                        let mut attributes: HashMap<String, i32> = HashMap::new();
                        let effects = Some(Vec::new());

                        // Required stats for all objects
                        let mut hp = None;
                        let mut base_hp = None;
                        let mut base_def = None;

                        let mut damage_range = None;
                        let mut base_damage = None;
                        let mut base_speed = None;
                        let mut base_vision = None;

                        let stamina = None;
                        let base_stamina = None;

                        let mut structure = None;
                        let mut activity = None;
                        let mut shelter = None;

                        let morale = None;
                        let order = None;

                        let total_weight = Some(items.get_total_weight(obj.id.0));
                        let capacity = Some(Obj::get_capacity(
                            &obj.template.0.to_string(),
                            &templates.obj_templates,
                        ));

                        if let Ok(attrs) = attrs_query.get(obj.entity) {
                            attributes.insert(obj::CREATIVITY.to_string(), attrs.creativity);
                            attributes.insert(obj::DEXTERITY.to_string(), attrs.dexterity);
                            attributes.insert(obj::ENDURANCE.to_string(), attrs.endurance);
                            attributes.insert(obj::FOCUS.to_string(), attrs.focus);
                            attributes.insert(obj::INTELLECT.to_string(), attrs.intellect);
                            attributes.insert(obj::SPIRIT.to_string(), attrs.spirit);
                            attributes.insert(obj::STRENGTH.to_string(), attrs.strength);
                            attributes.insert(obj::TOUGHNESS.to_string(), attrs.toughness);
                        }

                        if let Ok(stats) = stats_query.get(obj.entity) {
                            hp = Some(stats.hp);
                            base_hp = Some(stats.base_hp);
                            base_def = Some(stats.base_def);

                            damage_range = stats.damage_range;
                            base_damage = stats.base_damage;
                            base_speed = stats.base_speed;
                            base_vision = stats.base_vision;
                        }

                        if obj.subclass.0 == obj::SUBCLASS_HERO {
                            response_packet = ResponsePacket::InfoHero {
                                id: obj.id.0,
                                name: obj.name.0.to_string(),
                                template: obj.template.0.to_string(),
                                class: obj.class.0.to_string(),
                                subclass: obj.subclass.0.to_string(),
                                state: Obj::state_to_str(obj.state.to_owned()),
                                image: obj.misc.image.clone(),
                                hsl: obj.misc.hsl.clone(),
                                items: items_packet,
                                skills: skills_packet,
                                attributes: Some(attributes),
                                effects: effects,
                                hp: hp,
                                stamina: stamina,
                                base_hp: base_hp,
                                base_stamina: base_stamina,
                                base_def: base_def,
                                base_vision: base_vision,
                                base_speed: base_speed,
                                dmg_range: damage_range,
                                base_dmg: base_damage,
                            };
                        } else if obj.subclass.0 == obj::SUBCLASS_VILLAGER {

                            if let Ok(villager_attrs) = villager_query.get(obj.entity) {
                                activity = Some(villager_attrs.activity.to_string());
                                shelter = Some(villager_attrs.shelter.clone());
                                structure = Some(villager_attrs.structure);
                            }

                            response_packet = ResponsePacket::InfoVillager {
                                id: obj.id.0,
                                name: obj.name.0.to_string(),
                                template: obj.template.0.to_string(),
                                class: obj.class.0.to_string(),
                                subclass: obj.subclass.0.to_string(),
                                state: Obj::state_to_str(obj.state.to_owned()),
                                image: obj.misc.image.clone(),
                                hsl: obj.misc.hsl.clone(),
                                items: items_packet,
                                skills: skills_packet,
                                attributes: Some(attributes),
                                effects: effects,
                                hp: hp,
                                stamina: stamina,
                                base_hp: base_hp,
                                base_stamina: base_stamina,
                                base_def: base_def,
                                base_vision: base_vision,
                                base_speed: base_speed,
                                dmg_range: damage_range,
                                base_dmg: base_damage,
                                structure: None,
                                activity,
                                shelter: shelter,
                                morale: morale,
                                order: order,
                                capacity: capacity,
                                total_weight: total_weight,
                            };

                            let active_info_key = (*player_id, obj.id.0, "obj".to_string());
                            active_infos.insert(active_info_key, true);
                        }
                    } else if obj.class.0 == obj::CLASS_STRUCTURE {
                        let items_packet = Some(items.get_by_owner_packet(*id));
                        let effects = Some(Vec::new());

                        let total_weight = Some(items.get_total_weight(obj.id.0));
                        let capacity = Some(Obj::get_capacity(
                            &obj.template.0.to_string(),
                            &templates.obj_templates,
                        ));
                        let structure_template = Structure::get_template(
                            obj.template.0.to_string(),
                            &templates.obj_templates,
                        )
                        .expect("Cannot find structure template");

                        // Required stats for all objects
                        let mut hp = None;
                        let mut base_hp = None;
                        let mut base_def = None;

                        let mut progress = None;

                        if let Ok(stats) = stats_query.get(obj.entity) {
                            hp = Some(stats.hp);
                            base_hp = Some(stats.base_hp);
                            base_def = Some(stats.base_def);
                        }

                        if let Ok(structure_attrs) = structure_query.get(obj.entity) {
                            if *obj.state == State::Progressing {
                                let diff_time = structure_attrs.end_time - game_tick.0;
                                let ratio = diff_time as f32
                                    / structure_template.build_time.unwrap() as f32;
                                let percentage = ((1.0 - ratio) * 100.0).round() as i32;

                                progress = Some(percentage);
                            } else if *obj.state == State::Stalled {
                                progress = Some(structure_attrs.progress);
                            }
                        }

                        response_packet = ResponsePacket::InfoStructure {
                            id: obj.id.0,
                            name: obj.name.0.to_string(),
                            template: obj.template.0.to_string(),
                            class: obj.class.0.to_string(),
                            subclass: obj.subclass.0.to_string(),
                            state: Obj::state_to_str(obj.state.to_owned()),
                            image: obj.misc.image.clone(),
                            hsl: obj.misc.hsl.clone(),
                            items: items_packet,
                            effects: effects,
                            hp: hp,
                            base_hp: base_hp,
                            base_def: base_def,
                            capacity: capacity,
                            total_weight: total_weight,
                            build_time: structure_template.build_time,
                            progress: progress,
                            upgrade_req: structure_template.upgrade_req,
                        };
                    }
                } else {
                    let mut items_packet = None;

                    // Add items if object is dead
                    if *obj.state == State::Dead {
                        items_packet = Some(items.get_by_owner_packet(*id));
                    }

                    let mut effects = Vec::new();

                    // Get effects
                    for (key, _val) in obj.effects.0.iter() {
                        effects.push(key.clone().to_str());
                    }

                    // Non player owned object
                    response_packet = ResponsePacket::InfoNPC {
                        id: obj.id.0,
                        name: obj.name.0.to_string(),
                        template: obj.template.0.to_string(),
                        class: obj.class.0.to_string(),
                        subclass: obj.subclass.0.to_string(),
                        state: Obj::state_to_str(obj.state.to_owned()),
                        image: obj.misc.image.clone(),
                        hsl: obj.misc.hsl.clone(),
                        items: items_packet,
                        effects: effects,
                    };
                }

                send_to_client(*player_id, response_packet, &clients);
            }

            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_skills_system(
    mut events: ResMut<PlayerEvents>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    skills: Res<Skills>,
    templates: Res<Templates>,
    query: Query<CoreQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoSkills { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    continue;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find villager for {:?}", entity);
                    continue;
                };

                if obj.player_id.0 == *player_id {
                    let obj_skills =
                        Skill::get_by_owner_packet(obj.id.0, &skills, &templates.skill_templates);

                    let info_skills_packet = ResponsePacket::InfoSkills {
                        id: *id,
                        skills: obj_skills,
                    };

                    send_to_client(*player_id, info_skills_packet, &clients);
                } else {
                    error!("Object {:?} is not owned by player {:?}", id, player_id);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_attrs_system(
    mut events: ResMut<PlayerEvents>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    query: Query<CoreQuery>,
    attr_query: Query<&BaseAttrs>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoAttrs { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    continue;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find villager for {:?}", entity);
                    continue;
                };

                if obj.player_id.0 == *player_id {
                    if let Ok(attrs) = attr_query.get(entity) {
                        let mut attrs_packet = HashMap::new();

                        attrs_packet.insert(obj::CREATIVITY.to_string(), attrs.creativity);
                        attrs_packet.insert(obj::DEXTERITY.to_string(), attrs.dexterity);
                        attrs_packet.insert(obj::ENDURANCE.to_string(), attrs.endurance);
                        attrs_packet.insert(obj::FOCUS.to_string(), attrs.focus);
                        attrs_packet.insert(obj::INTELLECT.to_string(), attrs.intellect);
                        attrs_packet.insert(obj::SPIRIT.to_string(), attrs.spirit);
                        attrs_packet.insert(obj::TOUGHNESS.to_string(), attrs.toughness);

                        let info_attrs_packet = ResponsePacket::InfoAttrs {
                            id: *id,
                            attrs: attrs_packet,
                        };

                        send_to_client(*player_id, info_attrs_packet, &clients);
                    } else {
                        error!("Cannot find attributes for {:?}", id);
                    }
                } else {
                    error!("Object {:?} is not owned by player {:?}", id, player_id);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_advance_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    query: Query<CoreQuery>,
    skills: Res<Skills>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoAdvance { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    continue;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find obj for {:?}", entity);
                    continue;
                };

                if obj.player_id.0 == *player_id {
                    let (next_template, required_xp) = Skill::hero_advance(obj.template.0.clone());

                    let info_advance_packet = ResponsePacket::InfoAdvance {
                        id: obj.id.0,
                        rank: obj.template.0.clone(),
                        next_rank: next_template,
                        total_xp: Skill::get_total_xp(*id, &skills, &templates.skill_templates),
                        req_xp: required_xp,
                    };

                    send_to_client(*player_id, info_advance_packet, &clients);
                } else {
                    error!("Object {:?} is not owned by player {:?}", id, player_id);
                }
            }
            PlayerEvent::Advance { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    continue;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find obj for {:?}", entity);
                    continue;
                };

                if obj.player_id.0 == *player_id {
                    let (next_template, _required_xp) = Skill::hero_advance(obj.template.0.clone());

                    //Add obj update event
                    let obj_update_event = VisibleEvent::UpdateObjEvent {
                        attr: obj::TEMPLATE.to_string(),
                        value: next_template.clone(),
                    };

                    map_events.new(obj.id.0, game_tick.0, obj_update_event);

                    let (new_next_template, new_required_xp) =
                        Skill::hero_advance(next_template.clone());

                    let advance_packet = ResponsePacket::InfoAdvance {
                        id: obj.id.0,
                        rank: next_template.clone(),
                        next_rank: new_next_template,
                        total_xp: 0, // Advancing resets to zero
                        req_xp: new_required_xp,
                    };

                    send_to_client(*player_id, advance_packet, &clients);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_upgrade_system(
    mut events: ResMut<PlayerEvents>,
    _game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    structure_query: Query<StructureQuery, With<ClassStructure>>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoUpgrade {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(structure_entity) = ids.get_entity(*structure_id) else {
                    error!("Cannot find structure entity for {:?}", structure_id);
                    break;
                };

                let Ok(structure) = structure_query.get(structure_entity) else {
                    error!("Query failed to find entity {:?}", structure_entity);
                    break;
                };

                if structure.player_id.0 != *player_id {
                    error!("Structure not owned by player {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure not owned by player".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let current_structure_template =
                    ObjTemplate::get_template_by_name(structure.name.0.clone(), &templates);
                debug!(
                    "current_structure_template: {:?}",
                    current_structure_template
                );

                let Some(upgrade_to_list) = current_structure_template.upgrade_to else {
                    error!(
                        "Missing upgrade_to field on structure template: {:?}",
                        structure.name.0.clone()
                    );
                    continue;
                };

                let Some(upgrade_req) = current_structure_template.upgrade_req else {
                    error!(
                        "Missing upgrade_req field on structure template: {:?}",
                        structure.name.0.clone()
                    );
                    continue;
                };

                let mut upgrade_template_list = Vec::new();
                debug!("upgrade_to_list {:?}", upgrade_to_list);
                for upgrade_to_structure in upgrade_to_list.iter() {
                    let upgrade_structure_template = ObjTemplate::get_template_by_name(
                        upgrade_to_structure.to_string(),
                        &templates,
                    );
                    debug!(
                        "upgrade_structure_template {:?}",
                        upgrade_structure_template
                    );
                    let upgrade_template = network::UpgradeTemplate {
                        name: upgrade_structure_template.name,
                        template: upgrade_structure_template.template,
                    };

                    upgrade_template_list.push(upgrade_template);
                }

                if upgrade_template_list.len() == 0 {
                    error!(
                        "Cannot build upgrade template list for {:?}",
                        structure.name.0.clone()
                    );
                    continue;
                }

                let upgrade_packet = ResponsePacket::InfoUpgrade {
                    id: structure.id.0,
                    upgrade_list: upgrade_template_list,
                    req: upgrade_req,
                };

                send_to_client(*player_id, upgrade_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_tile_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    map: Res<Map>,
    resources: Res<Resources>,
    terrain_features: Res<TerrainFeatures>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoTile { player_id, x, y } => {
                debug!("PlayerEvent::InfoTile x: {:?} y: {:?}", *x, *y);
                events_to_remove.push(*event_id);

                let tile_type = Map::tile_type(*x, *y, &map);

                let info_tile_packet: ResponsePacket = ResponsePacket::InfoTile {
                    x: *x,
                    y: *y,
                    name: Map::tile_name(tile_type),
                    mc: Map::movement_cost(tile_type),
                    def: Map::def_bonus(tile_type),
                    unrevealed: Resource::num_unrevealed_on_tile(
                        Position { x: *x, y: *y },
                        &resources,
                    ),
                    sanctuary: "true".to_owned(),
                    passable: Map::is_passable(*x, *y, &map),
                    wildness: "high".to_owned(),
                    resources: Resource::get_on_tile(Position { x: *x, y: *y }, &resources),
                    terrain_features: TerrainFeature::get_by_tile(
                        Position { x: *x, y: *y },
                        &terrain_features,
                    ),
                };

                send_to_client(*player_id, info_tile_packet, &clients);
            }
            PlayerEvent::InfoTileResources { player_id, x, y } => {
                debug!("PlayerEvent::InfoTileResources x: {:?} y: {:?}", *x, *y);
                events_to_remove.push(*event_id);

                let tile_type = Map::tile_type(*x, *y, &map);

                let info_tile_resources_packet = ResponsePacket::InfoTileResources {
                    x: *x,
                    y: *y,
                    name: Map::tile_name(tile_type),
                    resources: Resource::get_on_tile(Position { x: *x, y: *y }, &resources),
                };

                send_to_client(*player_id, info_tile_resources_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_item_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    ids: ResMut<Ids>,
    items: ResMut<Items>,
    query: Query<CoreQuery>,
    templates: Res<Templates>,
    mut active_infos: ResMut<ActiveInfos>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoInventory { player_id, id } => {
                debug!("PlayerEvent::InfoInventory id: {:?}", id);
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    break;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find obj for {:?}", entity);
                    break;
                };

                let capacity = Obj::get_capacity(&obj.template.0, &templates.obj_templates);
                let total_weight = items.get_total_weight(*id);

                let inventory_items = items.get_by_owner_packet(*id);

                let info_inventory_packet: ResponsePacket = ResponsePacket::InfoInventory {
                    id: *id,
                    cap: capacity as i32,
                    tw: total_weight as i32,
                    items: inventory_items,
                };

                let active_info_key = (*player_id, *id, "inventory".to_string());
                active_infos.insert(active_info_key, true);

                send_to_client(*player_id, info_inventory_packet, &clients);
            }
            PlayerEvent::InfoItem {
                player_id,
                id,
                merchant_id: _,
                merchant_action,
            } => {
                events_to_remove.push(*event_id);

                if merchant_action == "merchantsell" {
                    let item = items.get_packet(*id);

                    if let Some(item) = item {
                        let info_item_packet: ResponsePacket = ResponsePacket::InfoItem {
                            id: item.id,
                            owner: item.owner,
                            name: item.name,
                            quantity: item.quantity,
                            class: item.class,
                            subclass: item.subclass,
                            image: item.image,
                            weight: item.weight,
                            equipped: item.equipped,
                            price: Some(10),
                            attrs: None,
                        };

                        send_to_client(*player_id, info_item_packet, &clients);
                    }
                } else if merchant_action == "merchantbuy" {
                    let item = items.get_packet(*id);

                    if let Some(item) = item {
                        let info_item_packet: ResponsePacket = ResponsePacket::InfoItem {
                            id: item.id,
                            owner: item.owner,
                            name: item.name,
                            quantity: item.quantity,
                            class: item.class,
                            subclass: item.subclass,
                            image: item.image,
                            weight: item.weight,
                            equipped: item.equipped,
                            price: Some(10),
                            attrs: None,
                        };

                        send_to_client(*player_id, info_item_packet, &clients);
                    }
                } else {
                    let item = items.get_packet(*id);

                    //let mut attrs = HashMap::new();
                    //attrs.insert(item::AttrKey::Damage, item::AttrVal::Num(17.0));

                    if let Some(item) = item {
                        let info_item_packet: ResponsePacket = ResponsePacket::InfoItem {
                            id: item.id,
                            owner: item.owner,
                            name: item.name,
                            quantity: item.quantity,
                            class: item.class,
                            subclass: item.subclass,
                            image: item.image,
                            weight: item.weight,
                            equipped: item.equipped,
                            price: None,
                            attrs: item.attrs,
                        };

                        send_to_client(*player_id, info_item_packet, &clients);
                    }
                }
            }
            PlayerEvent::InfoItemByName { player_id, name } => {
                debug!("PlayerEvent::InfoItemByName name: {:?}", name.clone());
                events_to_remove.push(*event_id);

                let item = items.get_by_name_packet(name.clone());

                if let Some(item) = item {
                    let info_item_packet: ResponsePacket = ResponsePacket::InfoItem {
                        id: item.id,
                        owner: item.owner,
                        name: item.name,
                        quantity: item.quantity,
                        class: item.class,
                        subclass: item.subclass,
                        image: item.image,
                        weight: item.weight,
                        equipped: item.equipped,
                        price: None,
                        attrs: None,
                    };

                    send_to_client(*player_id, info_item_packet, &clients);
                }
            }
            PlayerEvent::InfoExit {
                player_id,
                id,
                panel_type,
            } => {
                debug!(
                    "PlayerEvent::InfoExit {:?} {:?} {:?}",
                    player_id, id, panel_type
                );
                events_to_remove.push(*event_id);

                let active_info_key = (*player_id, *id, panel_type.clone());
                active_infos.remove(&active_info_key);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn item_transfer_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
    query: Query<ItemTransferQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::ItemTransfer {
                player_id,
                target_id,
                item_id,
            } => {
                events_to_remove.push(*event_id);

                if let Some(item) = items.find_by_id(*item_id) {
                    debug!("Item found: {:?}", item);

                    debug!("Entity ID map: {:?}", ids.obj_entity_map);

                    let Some(owner_entity) = ids.get_entity(item.owner) else {
                        error!("Cannot find owner entity from id: {:?}", item.owner);
                        continue;
                    };

                    let Some(target_entity) = ids.get_entity(*target_id) else {
                        error!("Cannot find target entity from id: {:?}", target_id);
                        continue;
                    };

                    let entities = [owner_entity, target_entity];

                    let Ok([owner, target]) = query.get_many(entities) else {
                        error!("Cannot find owner or target from entities {:?}", entities);
                        continue;
                    };

                    // Item has to be nearby
                    debug!(
                        "owner.pos: {:?} target.pos {:?} is_adjacent: {:?}",
                        owner.pos,
                        target.pos,
                        Map::is_adjacent(*owner.pos, *target.pos)
                    );
                    if !(owner.pos == target.pos || Map::is_adjacent(*owner.pos, *target.pos)) {
                        let packet = ResponsePacket::Error {
                            errmsg: "Item is not nearby.".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        continue;
                    }

                    // Transfer target is not dead
                    if *target.state == State::Dead {
                        let packet = ResponsePacket::Error {
                            errmsg: "Cannot transfer items to the dead or destroyed".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        continue;
                    }

                    // Cannot take items from tax collector, only transfer to
                    if Obj::has_group(obj::GROUP_TAX_COLLECTOR, owner.misc.groups.clone()) {
                        let packet = ResponsePacket::Error {
                            errmsg: "Cannot transfer items from tax collector".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        continue;
                    }

                    // Structure is not completed
                    if target.class.0 == "structure"
                        && (*target.state == State::Progressing || *target.state == State::Stalled)
                    {
                        let packet = ResponsePacket::Error {
                            errmsg: "Structure is not completed.".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        continue;
                    }

                    // Transfer target does not have enough capacity
                    let target_total_weight = items.get_total_weight(target.id.0);
                    let transfer_item_weight = (item.quantity as f32 * item.weight) as i32;
                    let target_capacity =
                        Obj::get_capacity(&target.template.0, &templates.obj_templates);

                    // Structure founded and under construction use case
                    if target.class.0 == "structure" && *target.state == State::Founded {
                        info!("Transfering to target structure with state founded.");
                        let structure_template =
                            ObjTemplate::get_template_by_name(target.name.0.clone(), &templates);
                        let structure_req = structure_template
                            .req
                            .expect("Template should have req field.");

                        //let attrs = target.structure_attrs;

                        // Check if item is required for structure construction

                        if !Item::is_req(item.clone(), structure_req) {
                            info!("Item not required for construction: {:?}", item);
                            let packet = ResponsePacket::Error {
                                errmsg: "Item not required for construction.".to_string(),
                            };
                            send_to_client(*player_id, packet, &clients);
                            continue;
                        }

                        // Process item transfer and calculate the require item quantities
                        let req_items = process_item_transfer_structure(
                            item.clone(),
                            target, // target is the structure
                            &mut items,
                            &templates,
                        );

                        if req_items.len() == 0 {
                            let packet = ResponsePacket::Error {
                                errmsg: "All structure item requirements met.".to_string(),
                            };
                            send_to_client(*player_id, packet, &clients);
                            continue;
                        }

                        let source_capacity =
                            Obj::get_capacity(&owner.template.0, &templates.obj_templates);
                        let source_total_weight = items.get_total_weight(owner.id.0);

                        let source_items = items.get_by_owner_packet(item.owner);
                        let target_items = items.get_by_owner_packet(*target_id);

                        let source_inventory = network::Inventory {
                            id: item.owner,
                            cap: source_capacity,
                            tw: source_total_weight,
                            items: source_items.clone(),
                        };

                        let target_inventory = network::Inventory {
                            id: *target_id,
                            cap: target_capacity,
                            tw: (target_total_weight + transfer_item_weight),
                            items: target_items.clone(),
                        };

                        let item_transfer_packet: ResponsePacket = ResponsePacket::ItemTransfer {
                            result: "success".to_string(),
                            sourceid: item.owner,
                            sourceitems: source_inventory,
                            targetid: *target_id,
                            targetitems: target_inventory,
                            reqitems: req_items,
                        };

                        send_to_client(*player_id, item_transfer_packet, &clients);
                    } else if owner.class.0 == "structure" && *owner.state == State::Founded {
                        info!("Transfering from owner structure with state founded.");

                        let structure_template =
                            ObjTemplate::get_template_by_name(owner.name.0.clone(), &templates);
                        let structure_req = structure_template
                            .req
                            .expect("Template should have req field.");

                        // This code appears to be a mistake
                        /* let attrs = target.structure_attrs;

                        // Check if item is required for structure construction
                        if let Some(attrs) = attrs {
                            if !Item::is_req(item.clone(), attrs.req.clone()) {
                                info!("Item not required for construction: {:?}", item);
                                let packet = ResponsePacket::Error {
                                    errmsg: "Item not required for construction.".to_string(),
                                };
                                send_to_client(*player_id, packet, &clients);
                                break;
                            }
                        } */

                        if let Some(_structure_attrs) = owner.structure_attrs {
                            items.transfer(item.id, target.id.0);

                            let structure_items = items.get_by_owner(owner.id.0);

                            let req_items =
                                Structure::process_req_items(structure_items, structure_req);

                            let source_capacity =
                                Obj::get_capacity(&owner.template.0, &templates.obj_templates);
                            let source_total_weight = items.get_total_weight(owner.id.0);

                            let source_items = items.get_by_owner_packet(item.owner);
                            let target_items = items.get_by_owner_packet(*target_id);

                            let source_inventory = network::Inventory {
                                id: item.owner,
                                cap: source_capacity,
                                tw: source_total_weight,
                                items: source_items.clone(),
                            };

                            let target_inventory = network::Inventory {
                                id: *target_id,
                                cap: target_capacity,
                                tw: target_total_weight + transfer_item_weight,
                                items: target_items.clone(),
                            };

                            let item_transfer_packet: ResponsePacket =
                                ResponsePacket::ItemTransfer {
                                    result: "success".to_string(),
                                    sourceid: item.owner,
                                    sourceitems: source_inventory,
                                    targetid: *target_id,
                                    targetitems: target_inventory,
                                    reqitems: req_items,
                                };

                            send_to_client(*player_id, item_transfer_packet, &clients);
                        } else {
                            error!("Obj is missing expected structure attributes");
                        }
                    } else {
                        if target_total_weight + transfer_item_weight > target_capacity {
                            let packet = ResponsePacket::Error {
                                errmsg: "Transfer target does not have enough capacity".to_string(),
                            };
                            send_to_client(*player_id, packet, &clients);
                            continue;
                        }

                        info!("Other item transfer");
                        items.transfer(item.id, target.id.0);

                        let source_capacity =
                            Obj::get_capacity(&owner.template.0, &templates.obj_templates);
                        let source_total_weight = items.get_total_weight(owner.id.0);

                        let source_items = items.get_by_owner_packet(item.owner);
                        let target_items = items.get_by_owner_packet(*target_id);

                        let source_inventory = network::Inventory {
                            id: item.owner,
                            cap: source_capacity,
                            tw: source_total_weight,
                            items: source_items.clone(),
                        };

                        let target_inventory = network::Inventory {
                            id: *target_id,
                            cap: target_capacity,
                            tw: target_total_weight + transfer_item_weight,
                            items: target_items.clone(),
                        };

                        let item_transfer_packet: ResponsePacket = ResponsePacket::ItemTransfer {
                            result: "success".to_string(),
                            sourceid: item.owner,
                            sourceitems: source_inventory,
                            targetid: *target_id,
                            targetitems: target_inventory,
                            reqitems: Vec::new(),
                        };

                        send_to_client(*player_id, item_transfer_packet, &clients);
                    }
                } else {
                    error!("Failed to find item");
                }
            }
            PlayerEvent::InfoItemTransfer {
                player_id,
                source_id,
                target_id,
            } => {
                events_to_remove.push(*event_id);

                debug!(
                    "PlayerEvent::InfoItemTransfer sourceid: {:?} targetid: {:?}",
                    *source_id, *target_id
                );

                if source_id == target_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Cannot transfer items to self".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let Some(source_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find source entity from id: {:?}", source_id);
                    continue;
                };

                let Some(target_entity) = ids.get_entity(*target_id) else {
                    error!("Cannot find target entity from id: {:?}", target_id);
                    continue;
                };

                let entities = [source_entity, target_entity];

                let Ok([source, target]) = query.get_many(entities) else {
                    error!("Cannot find source or target from entities {:?}", entities);
                    continue;
                };

                if !Map::is_adjacent(*source.pos, *target.pos) {
                    error!("Target is not nearby {:?}", target.id.0);
                    let packet = ResponsePacket::Error {
                        errmsg: "Target is not nearby".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                if target.player_id.0 != *player_id
                    && *target.state != State::Dead
                    && *target.subclass.0 != obj::SUBCLASS_MERCHANT.to_string()
                    && !Obj::has_group(obj::GROUP_TAX_COLLECTOR, (*target.misc.groups).to_vec())
                {
                    error!("Cannot transfer items with this target {:?}", target.id.0);
                    let packet = ResponsePacket::Error {
                        errmsg: "Cannot transfer items with this unit".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let source_capacity =
                    Obj::get_capacity(&source.template.0, &templates.obj_templates);
                let source_total_weight = items.get_total_weight(source.id.0);

                let mut target_capacity = -1; // -1 representing unknown
                let mut target_total_weight = -1; // -1 representing unknown

                if target.player_id.0 == *player_id {
                    target_capacity =
                        Obj::get_capacity(&target.template.0, &templates.obj_templates);
                    target_total_weight = items.get_total_weight(target.id.0);
                }

                let mut target_filter = Vec::new();

                if *target.subclass.0 == obj::SUBCLASS_MERCHANT.to_string() {
                    target_filter.push(item::GOLD.to_string());
                }

                if Obj::has_group(obj::GROUP_TAX_COLLECTOR, (*target.misc.groups).to_vec()) {
                    target_filter.push(item::FILTER_ALL.to_string());
                }

                let source_items = items.get_by_owner_packet(*source_id);
                let target_items = items.get_by_owner_packet_filter(*target_id, target_filter);

                let source_inventory = network::Inventory {
                    id: *source_id,
                    cap: source_capacity,
                    tw: source_total_weight,
                    items: source_items,
                };

                let target_inventory = network::Inventory {
                    id: *target_id,
                    cap: target_capacity,
                    tw: target_total_weight,
                    items: target_items.clone(),
                };

                let req_items = get_current_req_quantities(target, &items, &templates);

                let info_item_transfer_packet: ResponsePacket = ResponsePacket::InfoItemTransfer {
                    sourceid: *source_id,
                    sourceitems: source_inventory,
                    targetid: *target_id,
                    targetitems: target_inventory,
                    reqitems: req_items,
                };

                send_to_client(*player_id, info_item_transfer_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn item_split_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    _ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    _templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::ItemSplit {
                player_id,
                item_id,
                quantity,
            } => {
                events_to_remove.push(*event_id);

                if let Some(item) = items.find_by_id(*item_id) {
                    // TODO add checks if item_id is owned by player and if quantity is more than item quantity
                    items.split(*item_id, *quantity);

                    let item_split_packet: ResponsePacket = ResponsePacket::ItemSplit {
                        result: "success".to_string(),
                        owner: item.owner,
                    };

                    send_to_client(*player_id, item_split_packet, &clients);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_experiment_system(
    mut events: ResMut<PlayerEvents>,
    _game_tick: ResMut<GameTick>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    items: ResMut<Items>,
    experiments: Res<Experiments>,
    query: Query<CoreQuery>,
    _templates: Res<Templates>,
    mut active_infos: ResMut<ActiveInfos>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoExperinment {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(structure_entity) = ids.get_entity(*structure_id) else {
                    error!("Cannot find structure for {:?}", structure_id);
                    continue;
                };

                let Ok(structure) = query.get(structure_entity) else {
                    error!("Cannot find structure for {:?}", structure_entity);
                    continue;
                };

                if structure.player_id.0 != *player_id {
                    error!("Structure not owned by player {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure not owned by player".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let info_experiment;
                let (experiment_source, experiment_reagents, other_resources) =
                    items.get_experiment_details_packet(*structure_id);

                if let Some(experiment) = experiments.get(structure_id) {
                    info_experiment = ResponsePacket::InfoExperiment {
                        id: *structure_id,
                        expitem: experiment_source,
                        expresources: experiment_reagents,
                        validresources: other_resources,
                        expstate: Experiment::state_to_string(experiment.state.clone()),
                        recipe: Experiment::recipe_to_packet(experiment.clone()),
                    };
                } else {
                    info_experiment = ResponsePacket::InfoExperiment {
                        id: *structure_id,
                        expitem: experiment_source,
                        expresources: experiment_reagents,
                        validresources: other_resources,
                        expstate: experiment::EXP_STATE_NONE.to_string(),
                        recipe: None,
                    };
                }

                let active_info_key = (*player_id, *structure_id, "experiment".to_string());
                active_infos.insert(active_info_key, true);

                send_to_client(*player_id, info_experiment, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_hire_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    ids: ResMut<Ids>,
    skills: Res<Skills>,
    merchant_query: Query<&Transport, With<Merchant>>,
    query: Query<CoreQuery>,
    attrs_query: Query<&BaseAttrs>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoHire {
                player_id,
                source_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(merchant_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find entity for {:?}", source_id);
                    break;
                };

                let Ok(merchant) = merchant_query.get(merchant_entity) else {
                    error!("Cannot find obj for {:?}", merchant_entity);
                    break;
                };

                let mut hire_data: Vec<network::HireData> = Vec::new();

                for obj_id in merchant.hauling.iter() {
                    let Some(entity) = ids.get_entity(*obj_id) else {
                        error!("Cannot find entity for {:?}", obj_id);
                        break;
                    };

                    let Ok(obj) = query.get(entity) else {
                        error!("Cannot find obj for {:?}", entity);
                        break;
                    };

                    let Ok(attrs) = attrs_query.get(entity) else {
                        error!("Cannot find attrs for {:?}", entity);
                        break;
                    };

                    let skills = Skill::get_levels_by_owner(*obj_id, &skills);

                    let villager_data = network::HireData {
                        id: obj.id.0,
                        name: obj.name.0.clone(),
                        image: obj.misc.image.clone(),
                        wage: 25,
                        creativity: attrs.creativity,
                        dexterity: attrs.dexterity,
                        endurance: attrs.endurance,
                        focus: attrs.focus,
                        intellect: attrs.intellect,
                        spirit: attrs.spirit,
                        strength: attrs.strength,
                        toughness: attrs.toughness,
                        skills: skills,
                    };

                    hire_data.push(villager_data);
                }

                let info_hire = ResponsePacket::InfoHire { data: hire_data };

                send_to_client(*player_id, info_hire, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_follow_system(
    mut commands: Commands,
    clients: Res<Clients>,
    _game_tick: ResMut<GameTick>,
    ids: ResMut<Ids>,
    mut events: ResMut<PlayerEvents>,
    _map_events: ResMut<MapEvents>,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderFollow {
                player_id,
                source_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    break;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find hero entity for hero {:?}", hero_id);
                    break;
                };

                // Get hero state
                let mut hero_state = State::None;

                for q in &query {
                    if q.id.0 == hero_id {
                        hero_state = q.state.clone();
                    }
                }

                if Obj::is_dead(&hero_state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot give.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Add OrderFollow component to source and set hero_entity as target
                for q in &query {
                    if q.id.0 == *source_id {
                        commands.entity(q.entity).insert(Order::Follow {
                            target: hero_entity,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_gather_system(
    mut commands: Commands,
    clients: Res<Clients>,
    ids: ResMut<Ids>,
    game_tick: ResMut<GameTick>,
    mut events: ResMut<PlayerEvents>,
    mut map_events: ResMut<MapEvents>,
    resources: Res<Resources>,
    query: Query<CoreQuery, With<SubclassVillager>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderGather {
                player_id,
                source_id,
                res_type,
            } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find entity for {:?}", source_id);
                    break;
                };

                let Ok(villager) = query.get(entity) else {
                    error!("Cannot find villager for {:?}", entity);
                    break;
                };

                // TODO check if hero is dead

                if villager.player_id.0 != *player_id {
                    error!("Villager not owned by player {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Cannot order another player's villager".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                if Resource::is_valid_type(res_type.to_string(), *villager.pos, &resources) {
                    error!("Invalid resource type {:?}", res_type);
                    let packet = ResponsePacket::Error {
                        errmsg: "Invalid resource type {:?}".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                commands.entity(entity).insert(Order::Gather {
                    res_type: res_type.to_string(),
                });

                Obj::add_sound_obj_event(
                    game_tick.0,
                    Villager::order_to_speech(&Order::Gather {
                        res_type: res_type.to_string(),
                    }),
                    villager.id,
                    &mut map_events,
                );
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn structure_list_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    plans: Res<Plans>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::StructureList { player_id } => {
                events_to_remove.push(*event_id);
                let structure_list = Structure::available_to_build(
                    *player_id,
                    plans.clone(),
                    &templates.obj_templates,
                );

                let structure_list = StructureList {
                    result: structure_list,
                };

                let res_packet = ResponsePacket::StructureList(structure_list);

                send_to_client(*player_id, res_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn create_foundation_system(
    mut events: ResMut<PlayerEvents>,
    mut commands: Commands,
    game_tick: ResMut<GameTick>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    templates: Res<Templates>,
    hero_query: Query<CoreQuery, With<SubclassHero>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::CreateFoundation {
                player_id,
                source_id,
                structure_name,
            } => {
                debug!("CreateFoundation");
                events_to_remove.push(*event_id);

                // Validation checks and get hero entity
                let Some(hero_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find hero entity for {:?}", source_id);
                    continue;
                };

                let Ok(hero) = hero_query.get(hero_entity) else {
                    error!("Query failed to find entity {:?}", hero_entity);
                    continue;
                };

                if Obj::is_dead(&hero.state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot build structures.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if hero is owned by player
                if hero.player_id.0 != *player_id {
                    error!("Hero is not owned by player {:?}", *player_id);
                    continue;
                }

                // Get structure template
                let Some(structure_template) = Structure::get_template_by_name(
                    structure_name.clone(),
                    &templates.obj_templates,
                ) else {
                    let packet = ResponsePacket::Error {
                        errmsg: "Invalid structure name".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                };

                let structure_id = ids.new_obj_id();

                let structure = Obj {
                    id: Id(structure_id),
                    player_id: PlayerId(*player_id),
                    position: Position {
                        x: hero.pos.x,
                        y: hero.pos.y,
                    },
                    name: Name(structure_name.clone()),
                    template: Template(structure_template.template.clone()),
                    class: Class(structure_template.class),
                    subclass: Subclass(structure_template.subclass),
                    state: State::Founded,
                    viewshed: Viewshed { range: 0 },
                    misc: Misc {
                        image: str::replace(structure_template.template.as_str(), " ", "")
                            .to_lowercase(),
                        hsl: Vec::new(),
                        groups: Vec::new(),
                    },
                    stats: Stats {
                        hp: 1,
                        base_hp: structure_template.base_hp.unwrap(), // Convert option to non-option
                        stamina: None,
                        base_stamina: None,
                        base_def: 0,
                        base_damage: None,
                        damage_range: None,
                        base_speed: None,
                        base_vision: None,
                    },
                    effects: Effects(HashMap::new()),
                };

                let structure_attrs = StructureAttrs {
                    start_time: 0,
                    end_time: 0,
                    //build_time: structure_template.build_time.unwrap(), // Structure must have build time
                    builder: *source_id,
                    progress: 0,
                    //req: structure_template.req.unwrap(),
                };

                let structure_entity_id = commands
                    .spawn((structure, structure_attrs, ClassStructure))
                    .id();

                ids.new_obj(structure_id, *player_id, structure_entity_id);

                // Insert new obj event
                map_events.new(
                    structure_id,
                    game_tick.0 + 1,
                    VisibleEvent::NewObjEvent { new_player: false },
                );

                let packet = ResponsePacket::CreateFoundation {
                    result: "success".to_string(),
                };

                send_to_client(*player_id, packet, &clients)
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn build_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    game_tick: ResMut<GameTick>,
    mut map_events: ResMut<MapEvents>,
    ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
    builder_query: Query<CoreQuery, Or<(With<SubclassHero>, With<SubclassVillager>)>>,
    mut structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Build {
                player_id,
                source_id,
                structure_id,
            } => {
                debug!("Build");
                events_to_remove.push(*event_id);

                // Validation checks and get builder and structure entities
                let Some(builder_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find builder entity for {:?}", source_id);
                    break;
                };

                let Ok(builder) = builder_query.get(builder_entity) else {
                    error!("Query failed to find entity {:?}", builder_entity);
                    break;
                };

                let Some(structure_entity) = ids.get_entity(*structure_id) else {
                    error!("Cannot find structure entity for {:?}", structure_id);
                    break;
                };

                let Ok(mut structure) = structure_query.get_mut(structure_entity) else {
                    error!("Query failed to find entity {:?}", structure_entity);
                    break;
                };

                // Check if builder is owned by player
                if builder.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Builder not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Check if structure is owned by player
                if structure.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Check if builder is on the same pos as structure
                if *builder.pos != *structure.pos {
                    let packet = ResponsePacket::Error {
                        errmsg: "Builder must be on the same position as structure.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                let structure_template =
                    ObjTemplate::get_template_by_name(structure.name.0.clone(), &templates);

                let structure_req = structure_template
                    .req
                    .expect("Template should have req field");
                let structure_build_time = structure_template
                    .build_time
                    .expect("Template should have build_time field");

                // If structure is stalled, restart building
                if *structure.state != State::Stalled {
                    // Check if structure is missing required items
                    if !Structure::has_req(structure.id.0, &structure_req, &mut items) {
                        let packet = ResponsePacket::Error {
                            errmsg: "Structure is missing required items.".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        break;
                    }

                    // Consume req items
                    Structure::consume_reqs(structure.id.0, structure_req, &mut items);
                }

                // Set structure building attributes
                let progress_ratio = (100 - structure.attrs.progress) as f32 / 100.0;
                let build_time = (structure_build_time as f32 * progress_ratio) as i32;

                structure.attrs.start_time = game_tick.0;
                structure.attrs.end_time = game_tick.0 + build_time;
                structure.attrs.builder = *source_id;

                debug!("progress: {:?}", structure.attrs.progress);
                debug!("start_time: {:?}", structure.attrs.start_time);
                debug!("end_time: {:?}", structure.attrs.end_time);

                // Builder State Change Event to Building
                let state_change_event = VisibleEvent::StateChangeEvent {
                    new_state: obj::STATE_BUILDING.to_string(),
                };

                map_events.new(
                    builder.id.0,
                    game_tick.0 + 1, // in the future
                    state_change_event,
                );

                // Structure State Change Event to Progressing
                let structure_state_change = VisibleEvent::StateChangeEvent {
                    new_state: obj::STATE_PROGRESSING.to_string(),
                };

                map_events.new(
                    structure.id.0,
                    game_tick.0 + 1, // in the future
                    structure_state_change,
                );

                // Add build event for completion
                let build_event = VisibleEvent::BuildEvent {
                    builder_id: builder.id.0,
                    structure_id: structure.id.0,
                };

                map_events.new(
                    builder.id.0,
                    structure.attrs.end_time, // in the future
                    build_event,
                );

                let packet = ResponsePacket::Build {
                    build_time: structure_build_time,
                };

                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn upgrade_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    game_tick: ResMut<GameTick>,
    mut map_events: ResMut<MapEvents>,
    ids: ResMut<Ids>,
    _items: ResMut<Items>,
    templates: Res<Templates>,
    builder_query: Query<CoreQuery, Or<(With<SubclassHero>, With<SubclassVillager>)>>,
    mut structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Upgrade {
                player_id,
                source_id,
                structure_id,
                selected_upgrade,
            } => {
                events_to_remove.push(*event_id);

                let Some(structure_entity) = ids.get_entity(*structure_id) else {
                    error!("Cannot find structure entity for {:?}", structure_id);
                    continue;
                };

                let Ok(mut structure) = structure_query.get_mut(structure_entity) else {
                    error!("Query failed to find entity {:?}", structure_entity);
                    continue;
                };

                if *player_id != structure.player_id.0 {
                    error!("Structure not owned by player {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure not owned by player".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Validation checks and get builder and structure entities
                let Some(builder_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find builder entity for {:?}", source_id);
                    break;
                };

                let Ok(builder) = builder_query.get(builder_entity) else {
                    error!("Query failed to find entity {:?}", builder_entity);
                    break;
                };

                // Starting the upgrade and structure is not stalled upgrading
                if *structure.state == State::None {
                    let structure_template =
                        ObjTemplate::get_template_by_name(structure.name.0.clone(), &templates);

                    let structure_build_time = structure_template
                        .build_time
                        .expect("Template should have build_time field");

                    structure.attrs.start_time = game_tick.0;
                    structure.attrs.end_time = game_tick.0 + structure_build_time;
                    structure.attrs.builder = *source_id;
                }

                // Structure State Change Event to Progressing
                let builder_state_change = VisibleEvent::StateChangeEvent {
                    new_state: obj::STATE_UPGRADING.to_string(),
                };

                map_events.new(
                    builder.id.0,
                    game_tick.0 + 1, // in the future
                    builder_state_change,
                );

                // Structure State Change Event to Progressing
                let structure_state_change = VisibleEvent::StateChangeEvent {
                    new_state: obj::STATE_UPGRADING.to_string(),
                };

                map_events.new(
                    structure.id.0,
                    game_tick.0 + 1, // in the future
                    structure_state_change,
                );

                // Add upgrade event for completion
                let upgrade_event = VisibleEvent::UpgradeEvent {
                    builder_id: builder.id.0,
                    structure_id: structure.id.0,
                    selected_upgrade: selected_upgrade.clone(),
                };

                map_events.new(
                    structure.id.0,
                    game_tick.0 + 100, // in the future
                    upgrade_event,
                );

                let packet = ResponsePacket::Upgrade { upgrade_time: 100 };

                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn explore_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    hero_query: Query<CoreQuery, With<SubclassHero>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Explore { player_id } => {
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    continue;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find hero entity for hero {:?}", hero_id);
                    continue;
                };

                let Ok(hero) = hero_query.get(hero_entity) else {
                    error!("Cannot find hero for {:?}", hero_entity);
                    continue;
                };

                if Obj::is_dead(&hero.state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot explore.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // If hero is not already exploring
                // TODO expand the action and state checking across all actions
                if *hero.state == State::Exploring {
                    error!("Hero is already exploring {:?}", hero_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Already exploring".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Builder State Change Event to Building
                let state_change_event = VisibleEvent::StateChangeEvent {
                    new_state: obj::STATE_EXPLORING.to_string(),
                };

                map_events.new(
                    hero.id.0,
                    game_tick.0 + 1, // in the future
                    state_change_event,
                );

                // Insert explore event
                let explore_event = VisibleEvent::ExploreEvent;

                map_events.new(
                    hero.id.0,
                    game_tick.0 + 20, // in the future
                    explore_event,
                );

                let packet = ResponsePacket::Explore { explore_time: 20 };
                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn assign_list_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    ids: ResMut<Ids>,
    villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::AssignList { player_id } => {
                events_to_remove.push(*event_id);

                let mut assignments = Vec::new();

                for villager in villager_query.iter() {
                    if *player_id == villager.player_id.0 {
                        let mut structure_name = "None".to_string();

                        if villager.attrs.structure != -1 {
                            let Some(structure_entity) = ids.get_entity(villager.attrs.structure)
                            else {
                                error!(
                                    "Cannot find structure entity for {:?}",
                                    villager.attrs.structure
                                );
                                continue;
                            };

                            let Ok(structure) = structure_query.get(structure_entity) else {
                                error!("Query failed to find entity {:?}", structure_entity);
                                continue;
                            };

                            structure_name = structure.name.0.to_string();
                        }

                        let assignment = network::Assignment {
                            id: villager.id.0,
                            name: villager.name.0.to_string(),
                            image: villager.misc.image.to_string(),
                            order: "none".to_string(), // Query order
                            structure: structure_name,
                        };

                        assignments.push(assignment);
                    }
                }

                if assignments.len() == 0 {
                    let packet = ResponsePacket::Error {
                        errmsg: "No villagers available to assign".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let packet = ResponsePacket::AssignList {
                    result: assignments,
                };

                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn assign_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut events: ResMut<PlayerEvents>,
    ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    clients: Res<Clients>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Assign {
                player_id,
                source_id,
                target_id,
            } => {
                events_to_remove.push(*event_id);

                // Validation checks get source entity
                let Some(villager_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find villager entity for {:?}", source_id);
                    continue;
                };

                let Ok(mut villager) = villager_query.get_mut(villager_entity) else {
                    error!("Query failed to find entity {:?}", villager_entity);
                    continue;
                };

                let Some(structure_entity) = ids.get_entity(*target_id) else {
                    error!("Cannot find structure entity for {:?}", target_id);
                    continue;
                };

                let Ok(structure) = structure_query.get(structure_entity) else {
                    error!("Query failed to find entity {:?}", structure_entity);
                    continue;
                };

                // Check if builder is owned by player
                if villager.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Villager not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if structure is owned by player
                if structure.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                debug!(
                    "Assign villager to structure class {:?} with id {:?}",
                    structure.subclass.0, structure.id.0
                );

                // Set villager structure
                villager.attrs.structure = structure.id.0;
                villager.attrs.structure_template = structure.template.0.clone();

                if structure.subclass.0 == structure::RESOURCE {

                    Obj::add_sound_obj_event(
                        game_tick.0,
                        Villager::order_to_speech(&Order::Operate),
                        villager.id,
                        &mut map_events,
                    );

                    commands.entity(villager_entity).insert(Order::Operate);
                } else if structure.subclass.0 == structure::CRAFT {

                    Obj::add_sound_obj_event(
                        game_tick.0,
                        Villager::order_to_speech(&Order::Refine),
                        villager.id,
                        &mut map_events,
                    );

                    commands.entity(villager_entity).insert(Order::Refine);
                } else if structure.subclass.0 == structure::FARM {
                    // Do nothing because Plant, Tend and Harvest will start the action
                } else {
                    error!("Can only assign to crafting or harvesting buildings");
                    continue;
                }

                let packet = ResponsePacket::Assign {
                    result: "success".to_string(),
                };

                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn equip_system(
    mut events: ResMut<PlayerEvents>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut items: ResMut<Items>,
    query: Query<CoreQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Equip {
                player_id,
                item_id,
                status,
            } => {
                events_to_remove.push(*event_id);

                let Some(item) = items.find_by_id(*item_id) else {
                    debug!("Failed to find item: {:?}", item_id);
                    continue;
                };

                // Validation checks get source entity
                let Some(owner_entity) = ids.get_entity(item.owner) else {
                    error!("Cannot find villager entity for {:?}", item.owner);
                    continue;
                };

                let Ok(owner) = query.get(owner_entity) else {
                    error!("Query failed to find entity {:?}", owner_entity);
                    continue;
                };

                if Obj::is_dead(&owner.state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot equip items.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if entity is owned by player
                if owner.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if equipable
                if !Item::is_equipable(item.clone()) {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item is not equipable.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if object is busy
                if *owner.state != State::None {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item owner is busy".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                debug!("Equip packet: {:?}", status);
                // Equip if status is true
                if *status {
                    items.equip(*item_id, *status);
                } else {
                    items.equip(*item_id, *status);
                }

                let success_packet = ResponsePacket::Equip {
                    result: "success".to_string(),
                };

                send_to_client(*player_id, success_packet, &clients);

                let item_packet = items.get_packet(item.id).unwrap();

                let item_update_packet: ResponsePacket = ResponsePacket::InfoItemsUpdate {
                    id: item.owner,
                    items_updated: vec![item_packet],
                    items_removed: Vec::new(),
                };

                send_to_client(*player_id, item_update_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn recipe_list_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    ids: ResMut<Ids>,
    recipes: Res<Recipes>,
    structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::RecipeList {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(structure_entity) = ids.get_entity(*structure_id) else {
                    error!("Cannot find structure entity for {:?}", structure_id);
                    continue;
                };

                let Ok(structure) = structure_query.get(structure_entity) else {
                    error!("Query failed to find entity {:?}", structure_entity);
                    continue;
                };

                let structure_recipes =
                    recipes.get_by_structure_packet(*player_id, structure.template.0.clone());

                let packet = ResponsePacket::RecipeList {
                    result: structure_recipes,
                };

                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_refine_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    _ids: ResMut<Ids>,
    mut events: ResMut<PlayerEvents>,
    mut map_events: ResMut<MapEvents>,
    clients: Res<Clients>,
    villager_query: Query<VillagerQuery, With<SubclassVillager>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderRefine {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                let mut villager = None;

                //Find villager assigned to structure
                for villager_item in villager_query.iter() {
                    if villager_item.attrs.structure == *structure_id
                        && villager_item.player_id.0 == *player_id
                    {
                        villager = Some(villager_item);
                    }
                }

                if villager.is_none() {
                    error!(
                        "Cannot find a villager assigned to structure {:?}",
                        *structure_id
                    );
                    let packet = ResponsePacket::Error {
                        errmsg: "No villager assigned to structure to refine.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                if let Some(villager) = villager {
                    info!("Adding Order Refine to {:?}", villager.id);

                    //Add speech
                    Obj::add_sound_obj_event(
                        game_tick.0,
                        Villager::order_to_speech(&Order::Refine),
                        villager.id,
                        &mut map_events,
                    );

                    commands.entity(villager.entity).insert(Order::Refine);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_craft_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    _ids: ResMut<Ids>,
    mut events: ResMut<PlayerEvents>,
    mut map_events: ResMut<MapEvents>,
    clients: Res<Clients>,
    mut items: ResMut<Items>,
    recipes: Res<Recipes>,
    villager_query: Query<VillagerQuery, With<SubclassVillager>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderCraft {
                player_id,
                structure_id,
                recipe_name,
            } => {
                events_to_remove.push(*event_id);

                let mut villager = None;

                //Find villager assigned to structure
                for villager_item in villager_query.iter() {
                    if villager_item.attrs.structure == *structure_id
                        && villager_item.player_id.0 == *player_id
                    {
                        villager = Some(villager_item);
                    }
                }

                if villager.is_none() {
                    error!(
                        "Cannot find a villager assigned to structure {:?}",
                        *structure_id
                    );
                    let packet = ResponsePacket::Error {
                        errmsg: "No villager assigned to structure to refine.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                let recipe = recipes.get_by_name(recipe_name.clone());

                if recipe.is_none() {
                    error!("Invalid recipe name {:?}", *recipe_name);
                    let packet = ResponsePacket::Error {
                        errmsg: "Invalid recipe".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                if let Some(mut recipe) = recipe {
                    //TODO consider if checking reqs is required here
                    if Structure::has_req(*structure_id, &mut recipe.req, &mut items) {
                        if let Some(villager) = villager {
                            info!("Adding Order Craft to {:?}", villager.id);
                            commands.entity(villager.entity).insert(Order::Craft {
                                recipe_name: recipe_name.clone(),
                            });

                            //Add speech
                            Obj::add_sound_obj_event(
                                game_tick.0,
                                Villager::order_to_speech(&Order::Craft {
                                    recipe_name: recipe_name.to_string(),
                                }),
                                villager.id,
                                &mut map_events,
                            );
                        }
                    } else {
                        error!("Insufficient resources to craft {:?}", *recipe_name);
                        let packet = ResponsePacket::Error {
                            errmsg: "Insufficient resources to craft".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        break;
                    }
                } else {
                    error!("Cannot find recipe: {:?}", *recipe_name);
                    let packet = ResponsePacket::Error {
                        errmsg: "Cannot find recipe".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_explore_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    mut commands: Commands,
    mut map_events: ResMut<MapEvents>,
    clients: Res<Clients>,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderExplore {
                player_id,
                villager_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*villager_id) else {
                    error!("Cannot find entity for {:?}", villager_id);
                    break;
                };

                let Ok(villager) = query.get(entity) else {
                    error!("Cannot find villager for {:?}", entity);
                    break;
                };

                if villager.player_id.0 != *player_id {
                    error!("Villager not owned by player {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Cannot order another player's villager".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Add OrderFollow component to source and set hero_entity as target
                for q in &query {
                    if q.id.0 == *villager_id {
                        //Add speech
                        Obj::add_sound_obj_event(
                            game_tick.0,
                            Villager::order_to_speech(&Order::Explore),
                            villager.id,
                            &mut map_events,
                        );

                        commands.entity(q.entity).insert(Order::Explore);
                    }
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_experiment_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    _ids: ResMut<Ids>,
    mut events: ResMut<PlayerEvents>,
    mut map_events: ResMut<MapEvents>,
    items: ResMut<Items>,
    mut experiments: ResMut<Experiments>,
    _templates: Res<Templates>,
    active_infos: Res<ActiveInfos>,
    clients: Res<Clients>,
    villager_query: Query<VillagerQuery, With<SubclassVillager>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderExperiment {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                let mut villager = None;

                //Find villager assigned to structure
                for villager_item in villager_query.iter() {
                    if villager_item.attrs.structure == *structure_id
                        && villager_item.player_id.0 == *player_id
                    {
                        villager = Some(villager_item);
                    }
                }

                if villager.is_none() {
                    error!(
                        "Cannot find a villager assigned to structure {:?}",
                        *structure_id
                    );
                    let packet = ResponsePacket::Error {
                        errmsg: "No villager assigned to structure to refine.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                if let Some(villager) = villager {
                    info!("Adding Order Experiment to {:?}", villager.id);

                    // Update experiment state to progressing
                    let updated_experiment = Experiment::update_state(
                        villager.attrs.structure,
                        experiment::ExperimentState::Waiting,
                        &mut experiments,
                    );

                    if let Some(updated_experiment) = updated_experiment {
                        active_info_experiment(
                            villager.player_id.0,
                            villager.attrs.structure,
                            updated_experiment,
                            &items,
                            &active_infos,
                            &clients,
                        );
                    }

                    commands.entity(villager.entity).insert(Order::Experiment);

                    Obj::add_sound_obj_event(
                        game_tick.0,
                        Villager::order_to_speech(&Order::Experiment),
                        villager.id,
                        &mut map_events,
                    );
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_farm_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    _ids: ResMut<Ids>,
    mut events: ResMut<PlayerEvents>,
    mut map_events: ResMut<MapEvents>,
    items: ResMut<Items>,
    mut experiments: ResMut<Experiments>,
    _templates: Res<Templates>,
    active_infos: Res<ActiveInfos>,
    clients: Res<Clients>,
    villager_query: Query<VillagerQuery, With<SubclassVillager>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderPlant {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                // Find all villagers assigned to farm and order them to plant
                for villager_item in villager_query.iter() {
                    if villager_item.attrs.structure == *structure_id
                        && villager_item.player_id.0 == *player_id
                    {
                        commands.entity(villager_item.entity).insert(Order::Plant);

                        Obj::add_sound_obj_event(
                            game_tick.0,
                            Villager::order_to_speech(&Order::Plant),
                            villager_item.id,
                            &mut map_events,
                        );
                    }
                }                
            }
            PlayerEvent::OrderHarvest {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                // Find all villagers assigned to farm and order them to plant
                for villager_item in villager_query.iter() {
                    if villager_item.attrs.structure == *structure_id
                        && villager_item.player_id.0 == *player_id
                    {
                        commands.entity(villager_item.entity).insert(Order::Harvest);

                        Obj::add_sound_obj_event(
                            game_tick.0,
                            Villager::order_to_speech(&Order::Harvest),
                            villager_item.id,
                            &mut map_events,
                        );
                    }
                }                
            }            
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}


fn use_item_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    _commands: Commands,
    clients: Res<Clients>,
    items: ResMut<Items>,
    mut map_events: ResMut<MapEvents>,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Use { player_id, item_id } => {
                events_to_remove.push(*event_id);

                let Some(item) = items.find_by_id(*item_id) else {
                    debug!("Failed to find item: {:?}", item_id);
                    continue;
                };

                // Validation checks get source entity
                let Some(owner_entity) = ids.get_entity(item.owner) else {
                    error!("Cannot find villager entity for {:?}", item.owner);
                    continue;
                };

                let Ok(owner) = query.get(owner_entity) else {
                    error!("Query failed to find entity {:?}", owner_entity);
                    continue;
                };

                if Obj::is_dead(&owner.state) {
                    let packet = ResponsePacket::Error {
                        errmsg: "The dead cannot use items.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if entity is owned by player
                if owner.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }
                // Insert explore event
                let use_item_event = VisibleEvent::UseItemEvent {
                    item_id: *item_id,
                    item_owner_id: owner.id.0,
                };

                map_events.new(owner.id.0, game_tick.0 + 1, use_item_event);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn remove_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Remove {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*structure_id) else {
                    error!("Cannot find entity for {:?}", structure_id);
                    continue;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find obj for {:?}", entity);
                    continue;
                };

                // Check if entity is owned by player
                if obj.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Obj not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                debug!("Removing obj: {:?}", obj.id.0);

                map_events.new(
                    obj.id.0,
                    game_tick.0 + 1,
                    VisibleEvent::RemoveObjEvent {
                        pos: obj.pos.to_owned(),
                    },
                );
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn set_experiment_item_system(
    mut events: ResMut<PlayerEvents>,
    _game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    _map_events: ResMut<MapEvents>,
    mut items: ResMut<Items>,
    mut experiments: ResMut<Experiments>,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::SetExperimentItem {
                player_id,
                item_id,
                is_resource,
            } => {
                events_to_remove.push(*event_id);

                let Some(item) = items.find_by_id(*item_id) else {
                    debug!("Failed to find item: {:?}", item_id);
                    continue;
                };

                if !is_resource {
                    if Item::is_resource(item.clone()) {
                        let packet = ResponsePacket::Error {
                            errmsg: "Cannot set resource item as experiment source.".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        continue;
                    }
                } else {
                    if !Item::is_resource(item.clone()) {
                        let packet = ResponsePacket::Error {
                            errmsg: "Can only set resource items as an experiment reagent."
                                .to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        continue;
                    }
                }

                // Validation checks get source entity
                let Some(owner_entity) = ids.get_entity(item.owner) else {
                    error!("Cannot find villager entity for {:?}", item.owner);
                    continue;
                };

                let Ok(owner) = query.get(owner_entity) else {
                    error!("Query failed to find entity {:?}", owner_entity);
                    continue;
                };

                // Check if entity is owned by player
                if owner.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item owner not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                if !is_resource {
                    if let Some(experiment) = experiments.get_mut(&item.owner) {
                        debug!("Experiment: {:?}", experiment);
                        if let Some(source_item) = &experiment.source_item {
                            if source_item.id == *item_id {
                                // Player is transfering the item source out of experiment
                                items.remove_experiment_source(*item_id);
                                Experiment::reset(experiment);

                                send_info_experiment(
                                    *player_id,
                                    item.owner,
                                    experiment.clone(),
                                    &items,
                                    &clients,
                                );
                            } else {
                                let packet = ResponsePacket::Error {
                                    errmsg: "Experiment source item already set.".to_string(),
                                };
                                send_to_client(*player_id, packet, &clients);
                                continue;
                            }
                        } else {
                            let source_item = items.set_experiment_source(*item_id);
                            experiment.source_item = Some(source_item);

                            send_info_experiment(
                                *player_id,
                                item.owner,
                                experiment.clone(),
                                &items,
                                &clients,
                            );
                        }
                    } else {
                        // Experiment does not exist, set experiment item source and create experiment
                        let source_item = items.set_experiment_source(*item_id);

                        let experiment = Experiment::create(
                            item.owner,
                            None,
                            ExperimentState::None,
                            source_item,
                            Vec::new(),
                            &mut experiments,
                        );

                        send_info_experiment(
                            *player_id,
                            item.owner,
                            experiment.clone(),
                            &items,
                            &clients,
                        );
                    }
                } else {
                    if let Some(experiment) = experiments.get(&item.owner) {
                        if item.experiment.is_none() {
                            items.set_experiment_reagent(*item_id);
                        } else {
                            items.remove_experiment_reagent(*item_id);
                        }

                        send_info_experiment(
                            *player_id,
                            item.owner,
                            experiment.clone(),
                            &items,
                            &clients,
                        );
                    }
                }
            }
            PlayerEvent::ResetExperiment {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                if let Some(experiment) = experiments.get_mut(structure_id) {
                    Experiment::reset(experiment);

                    send_info_experiment(
                        *player_id,
                        *structure_id,
                        experiment.clone(),
                        &items,
                        &clients,
                    );
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn hire_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut events: ResMut<PlayerEvents>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    items: Res<Items>,
    mut map_events: ResMut<MapEvents>,
    mut pos_query: Query<&mut Position>,
    merchant_query: Query<&Transport, With<Merchant>>,
    mut player_query: Query<&mut PlayerId>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Hire {
                player_id,
                merchant_id,
                target_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    break;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find entity for {:?}", hero_id);
                    continue;
                };

                let Ok(hero_pos) = pos_query.get(hero_entity).copied() else {
                    error!("Cannot find obj for {:?}", hero_entity);
                    continue;
                };

                let Some(merchant_entity) = ids.get_entity(*merchant_id) else {
                    error!("Cannot find entity for {:?}", merchant_id);
                    continue;
                };

                let Ok(merchant_pos) = pos_query.get(merchant_entity).copied() else {
                    error!("Cannot find obj for {:?}", merchant_entity);
                    continue;
                };

                let Ok(merchant) = merchant_query.get(merchant_entity) else {
                    error!("Cannot find merchant component for {:?}", merchant_entity);
                    continue;
                };

                // Check if merchant is hauling target
                let mut hauling_target = false;

                for hauling_id in merchant.hauling.iter() {
                    debug!("hauling id: {:?}", hauling_id);
                    if hauling_id == target_id {
                        hauling_target = true;
                        break;
                    }
                }

                if !hauling_target {
                    let packet = ResponsePacket::Error {
                        errmsg: "Hire target is not being hauled".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                debug!("hero gold: {:?}", items.get_total_gold(hero_id));

                if items.get_total_gold(hero_id) < 25 {
                    let packet = ResponsePacket::Error {
                        errmsg: "Insufficient gold".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                if !Map::is_adjacent(hero_pos, merchant_pos) {
                    let packet = ResponsePacket::Error {
                        errmsg: "Merchant is not nearby".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let Some(target_entity) = ids.get_entity(*target_id) else {
                    error!("Cannot find entity for {:?}", target_id);
                    continue;
                };

                let mut target_player_id = player_query.get_mut(target_entity).unwrap();

                *target_player_id = PlayerId(*player_id);

                let Ok(mut target_pos) = pos_query.get_mut(target_entity) else {
                    error!("Cannot find pos for {:?}", target_entity);
                    continue;
                };

                *target_pos = hero_pos;

                // Add Move Event
                let new_obj_event = VisibleEvent::NewObjEvent { new_player: false };

                map_events.new(
                    *target_id,
                    game_tick.0 + 1, // in the future
                    new_obj_event,
                );

                commands.entity(target_entity).insert((
                    Thirst::new(0.0, 0.10), //0.1 before
                    Hunger::new(0.0, 0.10),
                    Tired::new(0.0, 0.10),
                    Morale::new(50.0),
                    Thinker::build()
                        .label("My Thinker")
                        .picker(Highest)
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("EnemyDistanceScorer")
                                .push(EnemyDistanceScorer),
                            Flee,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("FindDrinkScorer")
                                .push(ThirstyScorer)
                                .push(FindDrinkScorer),
                            FindDrink,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("DrinkDistanceScorer")
                                .push(ThirstyScorer)
                                .push(DrinkDistanceScorer),
                            MoveToWaterSource,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("TransferDrinkScorer")
                                .push(ThirstyScorer)
                                .push(TransferDrinkScorer),
                            TransferDrink,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("HasDrinkScorer")
                                .push(ThirstyScorer)
                                .push(HasDrinkScorer),
                            Drink { until: 70.0 },
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("FindFoodScorer")
                                .push(HungryScorer)
                                .push(FindFoodScorer),
                            FindFood,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("FoodDistanceScorer")
                                .push(HungryScorer)
                                .push(FoodDistanceScorer),
                            MoveToFoodSource,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("TransferFoodScorer")
                                .push(HungryScorer)
                                .push(TransferFoodScorer),
                            TransferFood,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("HasFoodScorer")
                                .push(HungryScorer)
                                .push(HasFoodScorer),
                            Eat,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("FindShelterScorer")
                                .push(DrowsyScorer)
                                .push(FindShelterScorer),
                            FindShelter,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("ShelterDistanceScorer")
                                .push(DrowsyScorer)
                                .push(ShelterDistanceScorer),
                            MoveToSleepPos,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("NearShelterScorer")
                                .push(DrowsyScorer)
                                .push(NearShelterScorer),
                            Sleep,
                        )
                        .when(
                            ProductOfScorers::build(0.5)
                                .label("GoodMoraleScorer")
                                .push(GoodMorale),
                            ProcessOrder,
                        ),
                ));
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn buy_sell_system(
    _commands: Commands,
    _game_tick: Res<GameTick>,
    mut events: ResMut<PlayerEvents>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut items: ResMut<Items>,
    _map_events: ResMut<MapEvents>,
    pos_query: Query<&mut Position>,
    _merchant_query: Query<&Merchant>,
    _player_query: Query<&mut PlayerId>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::BuyItem {
                player_id,
                item_id,
                quantity,
            } => {
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    continue;
                };

                let Some(item) = items.find_by_id(*item_id) else {
                    debug!("Failed to find item: {:?}", item_id);
                    continue;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find entity for {:?}", hero_id);
                    continue;
                };

                let Ok(hero_pos) = pos_query.get(hero_entity).copied() else {
                    error!("Cannot find obj for {:?}", hero_entity);
                    continue;
                };

                let Some(merchant_entity) = ids.get_entity(item.owner) else {
                    error!("Cannot find entity for {:?}", item.owner);
                    continue;
                };

                let Ok(merchant_pos) = pos_query.get(merchant_entity).copied() else {
                    error!("Cannot find obj for {:?}", merchant_entity);
                    continue;
                };

                if items.get_total_gold(hero_id) < 10 {
                    let packet = ResponsePacket::Error {
                        errmsg: "Insufficient gold".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                if !Map::is_adjacent(hero_pos, merchant_pos) {
                    let packet = ResponsePacket::Error {
                        errmsg: "Merchant is not nearby".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let merchant_id = item.owner;

                items.transfer_gold(hero_id, merchant_id, 25);
                items.transfer_quantity(item.id, hero_id, *quantity);

                let mut item_filter = Vec::new();
                item_filter.push(item::GOLD.to_string());

                let source_items = items.get_by_owner_packet(hero_id);
                let target_items = items.get_by_owner_packet_filter(merchant_id, item_filter);

                let source_inventory = network::Inventory {
                    id: hero_id,
                    cap: 0,
                    tw: 0,
                    items: source_items.clone(),
                };

                let target_inventory = network::Inventory {
                    id: merchant_id,
                    cap: 0,
                    tw: 0,
                    items: target_items.clone(),
                };

                let item_transfer_packet: ResponsePacket = ResponsePacket::BuyItem {
                    sourceid: hero_id,
                    sourceitems: source_inventory,
                    targetid: merchant_id,
                    targetitems: target_inventory,
                };

                send_to_client(*player_id, item_transfer_packet, &clients);
            }
            PlayerEvent::SellItem {
                player_id,
                item_id,
                target_id,
                quantity,
            } => {
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    continue;
                };

                let Some(item) = items.find_by_id(*item_id) else {
                    debug!("Failed to find item: {:?}", item_id);
                    continue;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find entity for {:?}", hero_id);
                    continue;
                };

                let Ok(hero_pos) = pos_query.get(hero_entity).copied() else {
                    error!("Cannot find obj for {:?}", hero_entity);
                    continue;
                };

                let Some(merchant_entity) = ids.get_entity(*target_id) else {
                    error!("Cannot find entity for {:?}", item.owner);
                    continue;
                };

                let Ok(merchant_pos) = pos_query.get(merchant_entity).copied() else {
                    error!("Cannot find obj for {:?}", merchant_entity);
                    continue;
                };

                if item.owner != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item is not owned by you.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                debug!("Hero Pos: {:?} Merchant Pos: {:?}", hero_pos, merchant_pos);

                if !Map::is_adjacent(hero_pos, merchant_pos) {
                    let packet = ResponsePacket::Error {
                        errmsg: "Merchant is not nearby".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // TOOD check if owner has room for the gold coins

                // TODO check if target has the space to hold the item

                items.transfer_gold(*target_id, item.owner, 25);
                items.transfer_quantity(*item_id, *target_id, *quantity);

                let mut item_filter = Vec::new();
                item_filter.push(item::GOLD.to_string());

                let source_items = items.get_by_owner_packet(item.owner);
                let target_items = items.get_by_owner_packet_filter(*target_id, item_filter);

                let source_inventory = network::Inventory {
                    id: item.owner,
                    cap: 0,
                    tw: 0,
                    items: source_items.clone(),
                };

                let target_inventory = network::Inventory {
                    id: *target_id,
                    cap: 0,
                    tw: 0,
                    items: target_items.clone(),
                };

                let item_transfer_packet: ResponsePacket = ResponsePacket::SellItem {
                    sourceid: item.owner,
                    sourceitems: source_inventory,
                    targetid: *target_id,
                    targetitems: target_inventory,
                };

                send_to_client(*player_id, item_transfer_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

pub fn active_info_experiment(
    player_id: i32,
    structure_id: i32,
    experiment: Experiment,
    items: &ResMut<Items>,
    active_infos: &Res<ActiveInfos>,
    clients: &Res<Clients>,
) {
    let active_info_key = (player_id, structure_id, "experiment".to_string());

    if let Some(_active_info) = active_infos.get(&active_info_key) {
        send_info_experiment(player_id, structure_id, experiment, items, clients);
    }
}

pub fn send_info_experiment(
    player_id: i32,
    structure_id: i32,
    experiment: Experiment,
    items: &ResMut<Items>,
    clients: &Res<Clients>,
) {
    let (experiment_source, experiment_reagents, other_resources) =
        items.get_experiment_details_packet(structure_id);

    let info_experiment: ResponsePacket = ResponsePacket::InfoExperiment {
        id: structure_id,
        expitem: experiment_source,
        expresources: experiment_reagents,
        validresources: other_resources,
        expstate: Experiment::state_to_string(experiment.state.clone()),
        recipe: Experiment::recipe_to_packet(experiment.clone()),
    };

    send_to_client(player_id, info_experiment, &clients);
}

fn new_player(
    player_id: i32,
    account_name: String,
    class_name: String,
    commands: &mut Commands,
    start_locations: &mut ResMut<StartLocations>,
    ids: &mut ResMut<Ids>,
    map_events: &mut ResMut<MapEvents>,
    _game_events: &mut ResMut<GameEvents>,
    items: &mut ResMut<Items>,
    skills: &mut ResMut<Skills>,
    recipes: &mut ResMut<Recipes>,
    plans: &mut ResMut<Plans>,
    templates: &Res<Templates>,
    game_tick: &Res<GameTick>,
) {
    // Select a start location and remove it from the list
    let start_location = start_locations.get_start_location();

    // Creating hero
    debug!("Creating hero for player: {:?}", player_id);
    let hero_template_name = "Novice".to_string() + " " + class_name.as_str();
    let hero_template = ObjTemplate::get_template(hero_template_name.clone(), templates);

    let hero_id = ids.new_obj_id();

    let hero = Obj {
        id: Id(hero_id),
        player_id: PlayerId(player_id),
        position: Position {
            x: start_location.hero_pos[0],
            y: start_location.hero_pos[1],
        },
        name: Name(account_name.clone()),
        template: Template(hero_template_name),
        class: Class("unit".into()),
        subclass: Subclass("hero".into()),
        state: State::None,
        viewshed: Viewshed { range: 4 },
        misc: Misc {
            image: str::replace(hero_template.template.as_str(), " ", "").to_lowercase(),
            hsl: Vec::new(),
            groups: Vec::new(),
        },
        stats: Stats {
            hp: hero_template.base_hp.unwrap(),
            base_hp: hero_template.base_hp.unwrap(),
            stamina: hero_template.base_stamina,
            base_stamina: hero_template.base_stamina,
            base_def: hero_template.base_def.unwrap(),
            base_damage: hero_template.base_dmg,
            damage_range: hero_template.dmg_range,
            base_speed: hero_template.base_speed,
            base_vision: hero_template.base_vision,
        },
        effects: Effects(HashMap::new()),
    };

    // Create hero items
    items.new(hero_id, "Cragroot Maple Wood".to_string(), 10);
    items.new(hero_id, "Cragroot Maple Wood".to_string(), 5);
    items.new(hero_id, "Cragroot Maple Wood".to_string(), 5);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 25);
    items.new(hero_id, "Valleyrun Copper Ingot".to_string(), 5);
    items.new(hero_id, "Cragroot Maple Timber".to_string(), 5);
    items.new(hero_id, "Gold Coins".to_string(), 100);
    items.new(hero_id, "Copper Helm".to_string(), 1);
    items.new(hero_id, "Seeds".to_string(), 50);
    /*items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);
    items.new(hero_id, "Windstride Raw Hide".to_string(), 1);*/

    let mut item_attrs = HashMap::new();
    item_attrs.insert(item::AttrKey::Damage, item::AttrVal::Num(11.0));
    item_attrs.insert(item::AttrKey::DeepWoundChance, item::AttrVal::Num(0.9));

    items.new_with_attrs(
        hero_id,
        "Copper Training Axe".to_string(),
        1,
        item_attrs.clone(),
    );

    items.new_with_attrs(
        hero_id,
        "Copper Broad Axe".to_string(),
        1,
        item_attrs.clone(),
    );

    let mut item_attrs = HashMap::new();
    item_attrs.insert(item::AttrKey::Feed, item::AttrVal::Num(100.0));

    items.new_with_attrs(
        hero_id,
        "Honeybell Berries".to_string(),
        5,
        item_attrs.clone(),
    );

    let mut item_attrs2 = HashMap::new();
    item_attrs2.insert(item::AttrKey::Healing, item::AttrVal::Num(10.0));

    items.new_with_attrs(hero_id, "Health Potion".to_string(), 1, item_attrs2);

    let hero_attrs = Obj::generate_hero_attrs();

    // Spawn hero
    let hero_entity_id = commands
        .spawn((
            hero,
            hero_attrs,
            SubclassHero, // Hero component tag
        ))
        .id();

    // New Obj mappings
    ids.new_hero(hero_id, player_id, hero_entity_id);

    // Create NewObjEvent
    map_events.new(
        hero_id,
        game_tick.0 + 1,
        VisibleEvent::NewObjEvent { new_player: true },
    );

    debug!("map_events: {:?}", map_events);

    // Villager obj
    let villager_id = ids.new_obj_id();

    let villager_template_name = "Human Villager".to_string();
    let villager_template = ObjTemplate::get_template(villager_template_name.clone(), templates);

    let villager = Obj {
        id: Id(villager_id),
        player_id: PlayerId(player_id),
        position: Position {
            x: start_location.villager_pos[0],
            y: start_location.villager_pos[1],
        },
        name: Name("Villager 1".into()),
        template: Template("Human Villager".into()),
        class: Class("unit".into()),
        subclass: Subclass("villager".into()),
        state: State::None,
        viewshed: Viewshed { range: 2 },
        misc: Misc {
            image: "humanvillager1".into(),
            hsl: Vec::new(),
            groups: Vec::new(),
        },
        stats: Stats {
            hp: villager_template.base_hp.unwrap(),
            base_hp: villager_template.base_hp.unwrap(),
            stamina: villager_template.base_stamina,
            base_stamina: villager_template.base_stamina,
            base_def: villager_template.base_def.unwrap(),
            base_damage: villager_template.base_dmg,
            damage_range: villager_template.dmg_range,
            base_speed: villager_template.base_speed,
            base_vision: villager_template.base_vision,
        },
        effects: Effects(HashMap::new()),
    };

    // Villager generate skills
    Villager::generate_skills(villager_id, skills, &templates.skill_templates);

    // Villager create attributes components ```
    let base_attrs = Villager::generate_attributes(1);

    let villager_attrs = VillagerAttrs {
        shelter: "None".to_string(),
        structure: -1,
        structure_template: "None".to_string(),
        activity: villager::Activity::None,
    };

    let find_move_to_and_drink = Steps::build()
        .label("FindMoveToAndDrink")
        .step(FindDrink)
        .step(MoveToWaterSource)
        .step(TransferDrink)
        .step(Drink { until: 70.0 });

    let find_move_to_and_eat = Steps::build()
        .label("FindMoveToAndEat")
        .step(FindFood)
        .step(MoveToFoodSource)
        .step(TransferFood)
        .step(Eat);

    let find_move_to_and_sleep = Steps::build()
        .label("FindMoveToAndSleep")
        .step(FindShelter)
        .step(MoveToSleepPos)
        .step(Sleep);

    let villager_entity_id = commands
        .spawn((
            villager,
            SubclassVillager,
            base_attrs,
            villager_attrs,
            Thirst::new(0.0, 0.025), //0.1 before
            Hunger::new(0.0, 0.025),
            Tired::new(0.0, 0.025),
            Heat::new(50.0),
            Morale::new(50.0),
            Thinker::build()
                .label("Villager")
                .picker(Highest)
                .when(
                    EnemyDistanceScorer,
                    Flee,
                )
                .when(
                    ThirstyScorer,
                    find_move_to_and_drink,
                )
                .when(
                    HungryScorer,
                    find_move_to_and_eat,
                )
                .when(
                    DrowsyScorer,
                    find_move_to_and_sleep,
                )
                .when(
                    IdleScorer,
                    Idle {
                        start_time: 0,
                        duration: 100,
                    },
                ).when(
                    GoodMorale,
                    ProcessOrder,
                )
        ))
        .id();

    ids.new_obj(villager_id, player_id, villager_entity_id);

    map_events.new(
        villager_id,
        game_tick.0 + 1,
        VisibleEvent::NewObjEvent { new_player: false },
    );

    // Villager obj
    let villager_id = ids.new_obj_id();

    let villager_template_name = "Human Villager".to_string();
    let villager_template = ObjTemplate::get_template(villager_template_name.clone(), templates);

    let villager = Obj {
        id: Id(villager_id),
        player_id: PlayerId(player_id),
        position: Position {
            x: start_location.villager_pos[0] + 1,
            y: start_location.villager_pos[1] + 1,
        },
        name: Name("Villager 2".into()),
        template: Template("Human Villager".into()),
        class: Class("unit".into()),
        subclass: Subclass("villager".into()),
        state: State::None,
        viewshed: Viewshed { range: 2 },
        misc: Misc {
            image: "humanvillager2".into(),
            hsl: Vec::new(),
            groups: Vec::new(),
        },
        stats: Stats {
            hp: villager_template.base_hp.unwrap(),
            base_hp: villager_template.base_hp.unwrap(),
            stamina: villager_template.base_stamina,
            base_stamina: villager_template.base_stamina,
            base_def: villager_template.base_def.unwrap(),
            base_damage: villager_template.base_dmg,
            damage_range: villager_template.dmg_range,
            base_speed: villager_template.base_speed,
            base_vision: villager_template.base_vision,
        },
        effects: Effects(HashMap::new()),
    };

    // Villager generate skills
    Villager::generate_skills(villager_id, skills, &templates.skill_templates);

    // Villager create attributes components ```
    let base_attrs = Villager::generate_attributes(1);

    let villager_attrs = VillagerAttrs {
        shelter: "None".to_string(),
        structure: -1,
        structure_template: "None".to_string(),
        activity: villager::Activity::None,
    };

    let find_move_to_and_drink = Steps::build()
        .label("FindMoveToAndDrink")
        .step(FindDrink)
        .step(MoveToWaterSource)
        .step(TransferDrink)
        .step(Drink { until: 70.0 });

    let find_move_to_and_eat = Steps::build()
        .label("FindMoveToAndEat")
        .step(FindFood)
        .step(MoveToFoodSource)
        .step(TransferFood)
        .step(Eat);

    let find_move_to_and_sleep = Steps::build()
        .label("FindMoveToAndSleep")
        .step(FindShelter)
        .step(MoveToSleepPos)
        .step(Sleep);

    let villager_entity_id = commands
        .spawn((
            villager,
            SubclassVillager,
            base_attrs,
            villager_attrs,
            Thirst::new(0.0, 0.025), //0.1 before
            Hunger::new(0.0, 0.025),
            Tired::new(0.0, 0.025),
            Heat::new(50.0),
            Morale::new(50.0),
            Thinker::build()
                .label("Villager")
                .picker(Highest)
                .when(
                    EnemyDistanceScorer,
                    Flee,
                )
                .when(
                    ThirstyScorer,
                    find_move_to_and_drink,
                )
                .when(
                    HungryScorer,
                    find_move_to_and_eat,
                )
                .when(
                    DrowsyScorer,
                    find_move_to_and_sleep,
                )
                .when(
                    IdleScorer,
                    Idle {
                        start_time: 0,
                        duration: 100,
                    },
                ).when(
                    GoodMorale,
                    ProcessOrder,
                )
        ))
        .id();

    ids.new_obj(villager_id, player_id, villager_entity_id);

    map_events.new(
        villager_id,
        game_tick.0 + 1,
        VisibleEvent::NewObjEvent { new_player: false },
    );

    // Starting recipes
    recipes.create(player_id, "Training Pick Axe".to_string());
    recipes.create(player_id, "Copper Training Axe".to_string());

    //Starting plans
    plans.add(player_id, "Farm".to_string(), 0, 0);
    plans.add(player_id, "Crafting Tent".to_string(), 0, 0);
    plans.add(player_id, "Blacksmith".to_string(), 0, 0);
    plans.add(player_id, "Small Tent".to_string(), 0, 0);
    plans.add(player_id, "Burrow".to_string(), 0, 0);
    plans.add(player_id, "Stockade".to_string(), 0, 0);
    plans.add(player_id, "Mine".to_string(), 0, 0);

    let structure_id = ids.new_obj_id();

    // Create monolith
    Obj::create(
        player_id,
        "Monolith".to_string(),
        Position {
            x: start_location.monolith_pos[0],
            y: start_location.monolith_pos[1],
        },
        State::None,
        commands,
        ids,
        map_events,
        &game_tick,
        &templates,
    );

    let structure_name = "Burrow".to_string();
    let structure_template = ObjTemplate::get_template(structure_name.clone(), templates);

    let structure: Obj = Obj {
        id: Id(structure_id),
        player_id: PlayerId(player_id),
        position: Position {
            x: start_location.burrow_pos[0],
            y: start_location.burrow_pos[1],
        },
        name: Name("Burrow".into()),
        template: Template("Burrow".into()),
        class: Class("structure".into()),
        subclass: Subclass("storage".into()),
        state: State::None,
        viewshed: Viewshed { range: 0 },
        misc: Misc {
            image: "burrow".into(),
            hsl: Vec::new(),
            groups: Vec::new(),
        },
        stats: Stats {
            hp: 1,
            base_hp: structure_template.base_hp.unwrap_or(100), // Convert option to non-option
            stamina: None,
            base_stamina: None,
            base_def: 0,
            base_damage: None,
            damage_range: None,
            base_speed: None,
            base_vision: None,
        },
        effects: Effects(HashMap::new()),
    };

    let structure_attrs = StructureAttrs {
        start_time: 0,
        end_time: 0,
        //build_time: structure_template.build_time.unwrap(), // Structure must have build time
        builder: -1,
        progress: 0,
        //req: structure_template.req.unwrap(),
    };

    let structure_entity_id = commands
        .spawn((structure, structure_attrs, ClassStructure))
        .id();

    // New Obj mappings
    ids.new_obj(structure_id, player_id, structure_entity_id);

    map_events.new(
        structure_id,
        game_tick.0 + 1,
        VisibleEvent::NewObjEvent { new_player: false },
    );

    
    let mut thirst_attr = HashMap::new();
    thirst_attr.insert(item::AttrKey::Thirst, item::AttrVal::Num(90.0));

    let mut feed_attr = HashMap::new();
    feed_attr.insert(item::AttrKey::Feed, item::AttrVal::Num(90.0));

    items.new_with_attrs(structure_id, "Amitanian Grape".to_string(), 50, feed_attr);
    items.new_with_attrs(structure_id, "Spring Water".to_string(), 50, thirst_attr);

    info!("Items: {:?}", items);

    // Villager obj
    let villager_id2 = ids.new_obj_id();
    let merchant_player_id = 2000;

    let empire_pos = Position { x: 1, y: 37 };
    let landing_pos = Position {
        x: start_location.merchant_pos[0],
        y: start_location.merchant_pos[1],
    };

    let merchant = Obj::create_nospawn(
        ids,
        merchant_player_id,
        "Meager Merchant".to_string(),
        empire_pos,
        State::None,
        templates,
    );

    let merchant_id = merchant.id.0;

    // Merchant Items
    items.new(merchant_id, "Gold Coins".to_string(), 500);
    items.new(merchant_id, "Yurt Deed".to_string(), 1);

    let route = vec![empire_pos, landing_pos];

    let move_to_and_idle = Steps::build()
        .label("MoveToPos and Idle")
        // Set destination will set the move to pos
        .step(SetDestination)
        .step(MoveToPos)
        .step(Idle {
            start_time: 0,
            duration: 500,
        });

    /*let merchant_entity_id = commands
        .spawn((
            merchant,
            Merchant,
            Transport {
                route: route,
                next_stop: 0,
                hauling: vec![villager_id2],
            },
            Destination {
                // Set destination will set the move to pos
                pos: Position { x: -1, y: -1 },
            },
            Thinker::build()
                .label("Merchant")
                .picker(Highest)
                .when(MerchantScorer, move_to_and_idle),
        ))
        .id();

    ids.new_obj(merchant_id, merchant_player_id, merchant_entity_id);

    map_events.new(
        merchant_id,
        game_tick.0 + 1,
        VisibleEvent::NewObjEvent { new_player: false },
    );*/

    /*let villager2 = Obj {
        id: Id(villager_id2),
        player_id: PlayerId(merchant_player_id),
        position: empire_pos,
        name: Name("Villager 2".into()),
        template: Template("Human Villager".into()),
        class: Class("unit".into()),
        subclass: Subclass("villager".into()),
        state: State::Aboard,
        viewshed: Viewshed { range: 2 },
        misc: Misc {
            image: "humanvillager2".into(),
            hsl: Vec::new(),
            groups: Vec::new(),
        },
        stats: Stats {
            hp: villager_template.base_hp.expect("Missing hp stat"),
            base_hp: villager_template.base_hp.expect("Missing base_hp stat"),
            stamina: villager_template.base_stamina,
            base_stamina: villager_template.base_stamina,
            base_def: villager_template.base_def.expect("Missing base_def stat"),
            base_damage: villager_template.base_dmg,
            damage_range: villager_template.dmg_range,
            base_speed: villager_template.base_speed,
            base_vision: villager_template.base_vision,
        },
        effects: Effects(HashMap::new()),
    };

    // Villager generate skills
    Villager::generate_skills(villager_id2, skills, &templates.skill_templates);

    // Villager create attributes components ```
    let base_attrs2 = Villager::generate_attributes(1);

    let villager_attrs2 = VillagerAttrs {
        shelter: "None".to_string(),
        structure: -1,
        activity: villager::Activity::None,
    };

    let villager_entity_id2 = commands
        .spawn((villager2, SubclassVillager, base_attrs2, villager_attrs2))
        .id();

    ids.new_obj(villager_id2, player_id, villager_entity_id2);*/

    // Create shipwreck
    Obj::create(
        player_id,
        "Shipwreck".to_string(),
        Position {
            x: start_location.shipwreck_pos[0],
            y: start_location.shipwreck_pos[1],
        },
        State::None,
        commands,
        ids,
        map_events,
        &game_tick,
        &templates,
    );

    // Create human corpse 1
    /*Obj::create(
        999,
        "Human Corpse".to_string(),
        Position {
            x: start_location.corpse1_pos[0],
            y: start_location.corpse1_pos[1],
        },
        State::Dead,
        commands,
        ids,
        map_events,
        &game_tick,
        &templates,
    );

    // Create human corpse
    Obj::create(
        999,
        "Human Corpse".to_string(),
        Position {
            x: start_location.corpse2_pos[0],
            y: start_location.corpse2_pos[1],
        },
        State::Dead,
        commands,
        ids,
        map_events,
        &game_tick,
        &templates,
    );*/

    /*let event_type = GameEventType::NecroEvent {
        pos: Position {
            x: start_location.necromancer_pos[0],
            y: start_location.necromancer_pos[1],
        },
        home: Position {
            x: start_location.mausoleum_pos[0],
            y: start_location.mausoleum_pos[1],
        }
    };
    let event_id = ids.new_map_event_id();

    let event = GameEvent {
        event_id: event_id,
        run_tick: game_tick.0 + 100,
        game_event_type: event_type,
    };

    game_events.insert(event.event_id, event);*/

    /*Encounter::spawn_tax_collector(
        2000,
        landing_pos,
        empire_pos,
        player_id,
        commands,
        ids,
        items,
        &templates,
        &game_tick,
        map_events,
    );*/
}

fn get_current_req_quantities(
    target: ItemTransferQueryItem,
    items: &ResMut<Items>,
    templates: &Res<Templates>,
) -> Vec<ResReq> {
    if target.class.0 == "structure" && *target.state == State::Founded {
        let structure_template =
            ObjTemplate::get_template_by_name(target.name.0.clone(), templates);

        let target_items = items.get_by_owner(target.id.0);
        let mut req_items = structure_template
            .req
            .expect("Template should have req field.");

        // Check current required quantity from structure items
        for req_item in req_items.iter_mut() {
            let mut req_quantity = req_item.quantity;

            for target_item in target_items.iter() {
                if req_item.req_type == target_item.name
                    || req_item.req_type == target_item.class
                    || req_item.req_type == target_item.subclass
                {
                    if req_quantity - target_item.quantity > 0 {
                        req_quantity -= target_item.quantity;
                    } else {
                        req_quantity = 0;
                    }
                }
            }

            req_item.cquantity = Some(req_quantity);
        }

        return req_items;
    }

    // Return empty vector
    return Vec::new();
}

fn process_item_transfer_structure(
    item: Item,
    structure: ItemTransferQueryItem,
    items: &mut ResMut<Items>,
    templates: &Res<Templates>,
) -> Vec<ResReq> {
    let structure_items = items.get_by_owner(structure.id.0);
    let structure_template = ObjTemplate::get_template_by_name(structure.name.0.clone(), templates);
    let structure_req = structure_template
        .req
        .expect("Template should have req field");

    let mut req_items = Structure::process_req_items(structure_items, structure_req);

    // Find first matching req item
    let matching_req_item = req_items.iter_mut().find(|r| {
        r.req_type == item.name || r.req_type == item.class || r.req_type == item.subclass
    });

    if let Some(matching_req_item) = matching_req_item {
        if let Some(match_req_item_cquantity) = &mut matching_req_item.cquantity {
            if *match_req_item_cquantity > 0 {
                if *match_req_item_cquantity == item.quantity {
                    // Transfer entire item
                    items.transfer(item.id, structure.id.0);

                    // Set current quantity to 0
                    *match_req_item_cquantity = 0;
                } else if *match_req_item_cquantity > item.quantity {
                    // Transfer entire item
                    items.transfer(item.id, structure.id.0);

                    // Subtract current quantity
                    *match_req_item_cquantity -= item.quantity;
                } else if *match_req_item_cquantity < item.quantity {
                    // Split to create new item. Required here as item quantity is greater than req quantity
                    if let Some(new_split_item) = items.split(item.id, *match_req_item_cquantity) {
                        // Transfer the new item
                        items.transfer(new_split_item.id, structure.id.0);

                        // Set current quantity to 0
                        *match_req_item_cquantity = 0;
                    }
                }
            }

            // Return required items list
            return req_items;
        } else {
            error!("Matching current quantity is unexpected None.")
        }
    } else {
        error!("Item transfer is invalid due to lack of matching req item")
    }

    return Vec::new();
}

#[derive(Debug, Clone)]
pub enum TimeOfDay {
    Dawn,
    Morning,
    Afternoon,
    Evening,
    Dusk,
    Night,   
}

pub fn get_time_of_day(hour: i32)  -> TimeOfDay {
    match hour {
        1..=4 => TimeOfDay::Night,
        5..=5 => TimeOfDay::Dawn,
        6..=11 => TimeOfDay::Morning,
        12..=16 => TimeOfDay::Afternoon,
        17..=22 => TimeOfDay::Evening,
        23..=23 => TimeOfDay::Dusk,
        18..=24 => TimeOfDay::Night,
        _ => TimeOfDay::Night,
    }
}