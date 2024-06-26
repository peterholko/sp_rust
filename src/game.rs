use bevy::ecs::query::WorldQuery;
use bevy::utils::tracing::debug;
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use bevy_save::prelude::*;

use rand::Rng;
use serde::Serialize;

use std::collections::hash_map::Entry;
use std::{
    collections::HashMap,
    collections::HashSet,
    hash::Hash,
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use crossbeam_channel::{unbounded, Receiver as CBReceiver};
use tokio::sync::mpsc::Sender;

use async_compat::Compat;

use crate::account::Accounts;
use crate::combat::{Combat, CombatSpellQuery};
use crate::components::npc::Transport;
use crate::components::villager::{Dehydrated, Exhausted, Hunger, Starving, Thirst, Tired, Heat};
use crate::constants::{COMFORT_TEMPERATURE, DAWN, DUSK, EVENING, GAME_HOUR, GAME_TICKS_PER_DAY, MORNING, NIGHT};
use crate::effect::Effects;
use crate::encounter::Encounter;
use crate::event::{
    GameEvent, GameEventType, GameEvents, MapEvent, MapEvents, VisibleEvent, VisibleEvents,
};
use crate::experiment::{Experiment, ExperimentPlugin, ExperimentState, Experiments};
use crate::farm::{Crops, FarmPlugin};
use crate::ids::Ids;
use crate::item::{self, Item, ItemPlugin, Items};
use crate::map::{Map, MapPlugin, Season};
use crate::network::{self, network_obj, send_to_client, BroadcastEvents};
use crate::network::{ResponsePacket, StatsData};
use crate::obj::{self, Obj};
use crate::player::{self, ActiveInfos, PlayerEvent, PlayerPlugin};
use crate::plugins::ai::AIPlugin;
use crate::recipe::{RecipePlugin, Recipes};
use crate::resource::{Resource, ResourcePlugin, Resources};
use crate::skill::{Skill, SkillPlugin, Skills};
use crate::structure::{Plans, Structure, StructurePlugin};
use crate::templates::{ObjTemplate, Templates, TemplatesPlugin};
use crate::terrain_feature::{TerrainFeature, TerrainFeaturePlugin, TerrainFeatures};
use crate::villager;
use crate::world::{Weather, WeatherAreas, WorldPlugin};

pub struct GamePlugin;

#[derive(Resource, Deref, DerefMut, Clone, Debug)]
pub struct Clients(Arc<Mutex<HashMap<i32, Client>>>);

#[derive(Resource, Deref, DerefMut)]
pub struct NetworkReceiver(CBReceiver<PlayerEvent>);

#[derive(Resource, Deref, DerefMut, Debug, Default)]
pub struct GameTick(pub i32);

impl GameTick {
    pub fn to_hour(&self) -> i32 {
        
        let ticks_in_day = self.0 % GAME_TICKS_PER_DAY;
        let hour = (ticks_in_day / 100) + 1;

        return hour;
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct ExploredMap(pub HashMap<i32, Vec<(i32, i32)>>);

#[derive(Resource, Deref, DerefMut, Debug)]
struct PerceptionUpdates(HashSet<i32>);

#[derive(Debug, Clone)]
pub struct Client {
    pub id: i32,
    pub player_id: i32,
    pub sender: Sender<String>,
}

#[derive(Debug, Component, Clone)]
pub struct Id(pub i32);

#[derive(Debug, Reflect, Component, Default, Clone, Copy, Eq, PartialEq, Hash)]
#[reflect(Component)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Component, Clone)]
pub struct PlayerId(pub i32);

#[derive(Debug, Component, Clone)]
pub struct Name(pub String);

#[derive(Debug, Component, Clone)]
pub struct Template(pub String);

#[derive(Debug, Component, Clone)]
pub struct Class(pub String);

#[derive(Debug, Component, Clone)]
pub struct Subclass(pub String);

#[derive(Debug, Component, Clone, Eq, PartialEq, Hash)]
pub enum State {
    None,
    Dead,
    Moving,
    Founded,
    Progressing,
    Building,
    Upgrading,
    Stalled,
    Gathering,
    Refining,
    Operating,
    Mining,
    Lumberjacking,
    Crafting,
    Exploring,
    Experimenting,
    Planting,
    Harvesting,
    Drinking,
    Eating,
    Sleeping,
    Aboard,
    Casting,
    Hiding,
}

#[derive(Debug, Component, Clone)]
pub struct StateDead {
    pub dead_at: i32,
}

#[derive(Debug, Component, Clone)]
pub struct StateAboard {
    pub transport_id: i32,
}

#[derive(Debug, Component, Clone)]
pub struct Viewshed {
    pub range: u32,
}

#[derive(Debug, Component)]
pub struct SubclassHero; //Subclass Hero

#[derive(Debug, Component)]
pub struct SubclassVillager; //Subclass Villager

#[derive(Debug, Component)]
pub struct SubclassNPC; //Subclass Villager

#[derive(Debug, Component)]
pub struct ClassStructure; //Class Structure

#[derive(Debug, Component)]
pub struct ClassCorpse; //Class Corpse

#[derive(Debug, Component)]
pub struct AI;

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct Merchant;

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct Minions {
    pub ids: Vec<i32>,
}

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct Home {
    pub pos: Position,
}

#[derive(Debug, Component, Clone)]
pub struct BaseAttrs {
    pub creativity: i32,
    pub dexterity: i32,
    pub endurance: i32,
    pub focus: i32,
    pub intellect: i32,
    pub spirit: i32,
    pub strength: i32,
    pub toughness: i32,
}

#[derive(Debug, Component, Clone)]
pub struct Stats {
    pub hp: i32,
    pub stamina: Option<i32>,
    pub base_hp: i32,
    pub base_stamina: Option<i32>,
    pub base_def: i32,
    pub damage_range: Option<i32>,
    pub base_damage: Option<i32>,
    pub base_speed: Option<i32>,
    pub base_vision: Option<u32>,
}

#[derive(Debug, Component, Clone)]
pub struct Misc {
    pub image: String,
    pub hsl: Vec<i32>,
    pub groups: Vec<String>,
}

#[derive(Debug, Component, Clone)]
pub struct VillagerAttrs {
    pub shelter: String,
    pub structure: i32,
    pub structure_template: String, // Quick hack to determine the type of structure
    pub activity: villager::Activity, //Todo turn into solo component
}

#[derive(Debug, Component, Clone)]
pub struct StructureAttrs {
    pub start_time: i32,
    pub end_time: i32,
    pub builder: i32,
    pub progress: i32,
}

#[derive(Debug, Component, Clone)]
pub struct NPCAttrs {
    pub target: i32,
}

#[derive(Debug, Component, Eq, PartialEq)]
pub enum Order {
    Follow { target: Entity },
    Gather { res_type: String },
    Operate,
    Refine,
    Craft { recipe_name: String },
    Experiment,
    Explore,
    Plant,
    Tend,
    Harvest
}

#[derive(Debug, Component)]
pub struct EventInProgress {
    pub event_id: uuid::Uuid,
}

#[derive(Debug, Component)]
pub struct DrinkEventCompleted {
    pub item: Item,
}

#[derive(Debug, Component)]
pub struct EatEventCompleted {
    pub item: Item,
}

#[derive(Debug, Component)]
pub struct SleepEventCompleted;

#[derive(WorldQuery)]
#[world_query(derive(Debug))]
pub struct MapObjQuery {
    pub entity: Entity,
    pub id: &'static Id,
    pub player_id: &'static PlayerId,
    pub pos: &'static Position,
    pub name: &'static Name,
    pub template: &'static Template,
    pub class: &'static Class,
    pub subclass: &'static Subclass,
    pub state: &'static State,
    pub viewshed: &'static Viewshed,
    pub misc: &'static Misc,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct ObjWithStatsQuery {
    pub entity: Entity,
    pub id: &'static Id,
    pub player_id: &'static PlayerId,
    pub pos: &'static mut Position,
    pub name: &'static mut Name,
    pub template: &'static mut Template,
    pub class: &'static mut Class,
    pub subclass: &'static mut Subclass,
    pub state: &'static mut State,
    pub misc: &'static mut Misc,
    pub stats: &'static mut Stats,
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
    state: &'static mut State,
    attrs: &'static mut StructureAttrs,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct ObjQuery {
    pub entity: Entity,
    pub id: &'static Id,
    pub player_id: &'static PlayerId,
    pub pos: &'static Position,
    pub name: &'static Name,
    pub template: &'static mut Template,
    pub class: &'static Class,
    pub subclass: &'static Subclass,
    pub state: &'static mut State,
    pub viewshed: &'static Viewshed,
    pub misc: &'static Misc,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct ObjQueryMut {
    pub entity: Entity,
    pub id: &'static Id,
    pub player_id: &'static PlayerId,
    pub pos: &'static mut Position,
    pub name: &'static mut Name,
    pub template: &'static mut Template,
    pub class: &'static Class,
    pub subclass: &'static mut Subclass,
    pub state: &'static mut State,
    pub viewshed: &'static Viewshed,
    pub misc: &'static Misc,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct VillagerQuery {
    pub id: &'static Id,
    pub player_id: &'static PlayerId,
    pub pos: &'static Position,
    pub state: &'static mut State,
    pub attrs: &'static mut VillagerAttrs,
    pub order: &'static Order,
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MapPlugin)
            .add_plugins(AIPlugin)
            .add_plugins(PlayerPlugin)
            .add_plugins(TemplatesPlugin)
            .add_plugins(ItemPlugin)
            .add_plugins(ResourcePlugin)
            .add_plugins(TerrainFeaturePlugin)
            .add_plugins(SkillPlugin)
            .add_plugins(RecipePlugin)
            .add_plugins(ExperimentPlugin)
            .add_plugins(StructurePlugin)
            .add_plugins(FarmPlugin)
            .add_plugins(WorldPlugin)
            .init_resource::<GameTick>()
            .add_systems(Startup, Game::setup)
            .add_systems(PreUpdate, update_game_tick)
            .add_systems(PreUpdate, snapshot_system)
            .add_systems(Update, new_obj_event_system)
            .add_systems(Update, remove_obj_event_system)
            .add_systems(Update, move_event_system)
            .add_systems(Update, hide_event_system)
            .add_systems(Update, state_change_event_system)
            .add_systems(Update, update_obj_event_system)
            .add_systems(Update, build_event_system)
            .add_systems(Update, gather_event_system)
            .add_systems(Update, operate_refine_event_system)
            .add_systems(Update, craft_event_system)
            .add_systems(Update, experiment_event_system)
            .add_systems(Update, explore_event_system)
            .add_systems(Update, farm_event_system)
            .add_systems(Update, spell_raise_dead_event_system)
            .add_systems(Update, spell_damage_event_system)
            .add_systems(Update, broadcast_event_system)
            .add_systems(Update, effect_expired_event_system)
            .add_systems(Update, cooldown_event_system)
            .add_systems(Update, use_item_system)
            .add_systems(Update, drink_eat_system)
            .add_systems(Update, visible_event_system)
            .add_systems(Update, game_event_system)
            .add_systems(Update, resurrect_system)
            .add_systems(Update, remove_dead_system)
            .add_systems(Update, perception_system);

        // .add_system(task_move_to_target_system);
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    pub num_players: u32,
}

#[derive(Component)]
struct NetworkHandler(Task<IoTaskPool>);

impl Game {
    // pub fn setup(mut commands: Commands, task_pool: Res<IoTaskPool>) {
    pub fn setup(
        mut commands: Commands,
        mut items: ResMut<Items>,
        mut recipes: ResMut<Recipes>,
        mut resources: ResMut<Resources>,
        mut terrain_features: ResMut<TerrainFeatures>,
        templates: Res<Templates>,
        map: Res<Map>,
    ) {
        println!("Bevy Setup System");

        // Initialize game tick
        let game_tick: GameTick = GameTick(0);

        // Initialize map events vector
        let map_events: MapEvents = MapEvents(HashMap::new());
        let processed_map_events: VisibleEvents = VisibleEvents(Vec::new());

        let game_events: GameEvents = GameEvents(HashMap::new());

        let perception_updates: PerceptionUpdates = PerceptionUpdates(HashSet::new());

        // Initialize explored map
        let explored_map: ExploredMap = ExploredMap(HashMap::new());

        //Initialize Arc Mutex Hashmap to store the client to game channel per connected client
        let clients = Clients(Arc::new(Mutex::new(HashMap::new())));
        let accounts = Accounts::new(Mutex::new(HashMap::new()));

        //Create the client to game channel, note the sender will be cloned by each connected client
        let (client_to_game_sender, client_to_game_receiver) = unbounded::<PlayerEvent>();

        let thread_pool = IoTaskPool::get();

        //Spawn the tokio runtime setup using a Compat with the clients and client to game channel
        thread_pool
            .spawn(Compat::new(network::tokio_setup(
                client_to_game_sender,
                clients.clone(),
                accounts,
            )))
            .detach();

        let network_receiver = NetworkReceiver(client_to_game_receiver);

        // Initialize indexes
        let ids: Ids = Ids {
            map_event: 0,
            player_event: 0,
            obj: 0,
            item: 0,
            player_hero_map: HashMap::new(),
            obj_entity_map: HashMap::new(),
            obj_player_map: HashMap::new(),
        };

        //Insert the clients and client to game channel into the Bevy resources
        commands.insert_resource(ids);
        commands.insert_resource(clients);
        commands.insert_resource(network_receiver);
        commands.insert_resource(game_tick);
        commands.insert_resource(map_events);
        commands.insert_resource(processed_map_events);
        commands.insert_resource(game_events);
        commands.insert_resource(perception_updates);
        commands.insert_resource(explored_map);

        // Initialize game world
        Resource::spawn_all_resources(&mut resources, &templates, &map);
        TerrainFeature::spawn(&mut terrain_features, &templates, &map);

        // Initialize items, recipes
        items.set_templates(templates.item_templates.clone());
        recipes.set_templates(templates.recipe_templates.clone());
    }
}

fn new_obj_event_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    mut perception_updates: ResMut<PerceptionUpdates>,
    ids: Res<Ids>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::NewObjEvent { new_player } => {
                    debug!("Processing NewObjEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(player_id) = ids.get_player(map_event.obj_id) else {
                        error!("Cannot find player from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    if *new_player {
                        perception_updates.insert(player_id);
                    }

                    visible_events.push(map_event.clone());
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn remove_obj_event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    ids: Res<Ids>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::RemoveObjEvent { pos: _ } => {
                    debug!("RemoveObjEvent: {:?}", map_event);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    // Remove entity
                    commands.entity(entity).despawn();

                    // TODO Remove Ids entry for obj

                    visible_events.push(map_event.clone());
                    events_to_remove.push(*map_event_id);
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

// TODO modernize this system
fn move_event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    mut explored_map: ResMut<ExploredMap>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    active_infos: Res<ActiveInfos>,
    mut query: Query<ObjQueryMut>,
    mut transport_query: Query<&mut Transport>,
    aboard_query: Query<&StateAboard>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::MoveEvent { src: _, dst } => {
                    info!("Processing MoveEvent: {:?}", map_event);
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!(
                            "Cannot find entity from id: {:?} ids: {:?}",
                            map_event.obj_id, ids
                        );
                        continue;
                    };

                    let Some(player_id) = ids.get_player(map_event.obj_id) else {
                        error!("Cannot find player from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    debug!("MoveEvent - Removing EventInProgress...");
                    commands.entity(entity).remove::<EventInProgress>();

                    let mut is_dst_open = true;
                    let mut objs_on_tile = Vec::new();
                    let mut all_map_objs: Vec<network::MapObj> = Vec::new();

                    //TODO Move this logic to another function
                    for obj in query.iter() {
                        if (player_id != obj.player_id.0)
                            && obj.pos == dst
                            && Obj::is_blocking_state(obj.state.clone())
                        {
                            is_dst_open = false;
                        }

                        if obj.pos == dst {
                            objs_on_tile.push((
                                obj.player_id.clone(),
                                obj.id.clone(),
                                obj.subclass.clone(),
                            ));
                        }

                        all_map_objs.push(network::map_obj(obj));
                    }

                    // Get entity and update state
                    let Ok(mut mover) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    // Check if tile is open
                    if !is_dst_open {
                        error!("Tile is not opened.");

                        if *mover.state != State::Aboard {
                            *mover.state = State::None;
                        }

                        continue;
                    }

                    // Check if moving object is leaving a transport
                    if let Ok(aboard) = aboard_query.get(entity) {
                        debug!("Mover is leaving a transport: {:?}", mover.id.0);

                        // Get transport entity
                        let transport_entity = ids.get_entity(aboard.transport_id).unwrap();

                        // Get transport
                        let Ok(mut transport) = transport_query.get_mut(transport_entity) else {
                            error!("Query failed to find transport {:?}", transport_entity);
                            continue;
                        };

                        // Remove object from transport
                        transport.hauling.retain(|&x| x != mover.id.0);

                        // Remove StateAboard component from mover
                        commands.entity(entity).remove::<StateAboard>();
                    }

                    // Reset state and move object
                    *mover.state = State::None;
                    *mover.pos = dst.clone();

                    // Check if moving object is entering a transport
                    for (player_id, obj_id, subclass) in objs_on_tile.iter() {
                        if mover.player_id.0 != player_id.0 {
                            continue;
                        }

                        if subclass.0 == "transport".to_string() {
                            debug!("Transport is entering: {:?}", mover.id.0);

                            // Set state to aboard
                            *mover.state = State::Aboard;

                            // Get transport entity
                            let transport_entity = ids.get_entity(obj_id.0).unwrap();

                            // Get transport
                            let Ok(mut transport) = transport_query.get_mut(transport_entity)
                            else {
                                error!("Query failed to find transport {:?}", transport_entity);
                                continue;
                            };

                            // Add object to transport
                            transport.hauling.push(mover.id.0);

                            // Add StateAboard component to mover
                            commands.entity(entity).insert(StateAboard {
                                transport_id: obj_id.0,
                            });
                        }
                    }

                    // Adding processed map event
                    visible_events.push(map_event.clone());

                    // Check if moving object is a transport
                    if let Ok(transport) = transport_query.get_mut(entity) {
                        debug!("Transport is moving: {:?}", transport.hauling);
                        // Loop through transport hauling and update their positions
                        for obj_id in transport.hauling.iter() {
                            debug!("Adding game event to update pos for obj_id: {:?}", obj_id);
                            let event_type = GameEventType::UpdatePos {
                                obj_id: *obj_id,
                                pos: mover.pos.clone(),
                            };

                            let event_id = ids.new_map_event_id();

                            let event = GameEvent {
                                event_id: event_id,
                                run_tick: game_tick.0, // Add one game tick
                                game_event_type: event_type,
                            };

                            game_events.insert(event.event_id, event);
                        }
                    }

                    // If player is moving, TODO improve this
                    if mover.player_id.0 < 1000 {
                        let mut rng = rand::thread_rng();

                        let spawn_chance = 0.0001;
                        let random_num = rng.gen::<f32>();
                        debug!("random_num: {:?}", random_num);

                        // TODO move to encounter module
                        if random_num < spawn_chance {
                            let adjacent_pos = get_random_adjacent_pos(
                                mover.player_id.0,
                                dst.x,
                                dst.y,
                                Vec::new(),
                                &map,
                            );

                            if let Some(adjacent_pos) = adjacent_pos {
                                let tile_type =
                                    Map::tile_type(adjacent_pos.x, adjacent_pos.y, &map);
                                let npc_list = Encounter::npc_list(tile_type);
                                let mut rng = rand::thread_rng();
                                let index = rng.gen_range(0..npc_list.len());
                                let npc_type = npc_list[index].to_string();

                                debug!("Spawning a NPC of type: {:?}", npc_type);

                                let event_type = GameEventType::SpawnNPC {
                                    npc_type: npc_type,
                                    pos: adjacent_pos,
                                    npc_id: None,
                                };
                                let event_id = ids.new_map_event_id();

                                let event = GameEvent {
                                    event_id: event_id,
                                    run_tick: game_tick.0 + 4, // Add one game tick
                                    game_event_type: event_type,
                                };

                                game_events.insert(event.event_id, event);
                            }
                        }

                        // Getting new map tiles
                        info!("Mover Viewshed Range: {:?}", mover.viewshed.range);

                        let viewshed_tiles_pos =
                            Map::range((mover.pos.x, mover.pos.y), mover.viewshed.range);
                        info!("Viewshed Tiles: {:?}", viewshed_tiles_pos);

                        // Adding new maps to explored map
                        // Assume player has some explored map tiles
                        let player_explord_map = explored_map.get_mut(&mover.player_id.0).unwrap();

                        let mut new_explored_tiles = Vec::new();

                        for tile in viewshed_tiles_pos {
                            if !player_explord_map.contains(&tile) {
                                new_explored_tiles.push(tile);
                            }
                        }

                        let mut new_objs = Vec::new();

                        // Get new objs in viewshed
                        for map_obj in all_map_objs.iter() {
                            if mover.id.0 != map_obj.id {
                                let distance = Map::distance(
                                    (mover.pos.x, mover.pos.y),
                                    (map_obj.x, map_obj.y),
                                );

                                if mover.viewshed.range >= distance
                                    && Obj::is_visible(Obj::state_to_enum(map_obj.state.clone()))
                                {
                                    new_objs.push(map_obj.clone());
                                }
                            }
                        }

                        // Only send new explored tiles
                        if new_explored_tiles.len() > 0 {
                            let tiles_to_send = Map::pos_to_tiles(&new_explored_tiles, &map);
                            let map_packet = ResponsePacket::ObjPerception {
                                new_objs: new_objs,
                                new_tiles: tiles_to_send,
                            };
                            send_to_client(mover.player_id.0, map_packet, &clients);
                        }

                        // Check if player has an active info for this mover
                        let active_info_key = (mover.player_id.0, mover.id.0, "obj".to_string());

                        if let Some(_active_info) = active_infos.get(&active_info_key) {
                            let response_packet = ResponsePacket::InfoStateUpdate {
                                id: mover.id.0,
                                state: Obj::state_to_str(mover.state.clone()),
                            };

                            send_to_client(mover.player_id.0, response_packet, &clients);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn hide_event_system(
    game_tick: Res<GameTick>,
    ids: Res<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    mut state_query: Query<&mut State>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::HideEvent => {
                    debug!("Processing HideEvent {:?}", map_event);
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find corpse from {:?}", map_event.obj_id);
                        continue;
                    };

                    // Set state back to none
                    let Ok(mut state) = state_query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    *state = State::Hiding;

                    visible_events.push(map_event.clone());
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn state_change_event_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    ids: Res<Ids>,
    mut query: Query<ObjQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::StateChangeEvent { new_state } => {
                    debug!("Processing StateChangeEvent: {:?}", new_state);
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    // Set state back to none
                    let Ok(mut obj) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    *obj.state = Obj::state_to_enum(new_state.to_string());

                    visible_events.push(map_event.clone());
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn update_obj_event_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    ids: Res<Ids>,
    mut query: Query<ObjQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::UpdateObjEvent { attr, value } => {
                    debug!("Processing UpdateObjEvent: {:?} {:?}", attr, value);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    // Set state back to none
                    let Ok(mut obj) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    match attr.as_str() {
                        obj::TEMPLATE => {
                            obj.template.0 = value.to_string();
                            visible_events.push(map_event.clone());
                        }
                        obj::VISION => {
                            visible_events.push(map_event.clone());
                        }
                        _ => {}
                    }

                    events_to_remove.push(*map_event_id);
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn build_event_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    ids: ResMut<Ids>,
    templates: Res<Templates>,
    mut query: Query<ObjWithStatsQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::BuildEvent {
                    builder_id,
                    structure_id,
                } => {
                    debug!(
                        "Processing BuildEvent: builder_id {:?}, structure_id: {:?} ",
                        builder_id, structure_id
                    );
                    events_to_remove.push(*map_event_id);

                    let Some(builder_entity) = ids.get_entity(*builder_id) else {
                        error!("Cannot find builder from {:?}", *builder_id);
                        continue;
                    };

                    let Ok(mut builder) = query.get_mut(builder_entity) else {
                        error!("Query failed to find entity {:?}", builder_entity);
                        continue;
                    };

                    *builder.state = State::None;

                    visible_events.new(
                        builder.id.0,
                        game_tick.0 + 1,
                        VisibleEvent::StateChangeEvent {
                            new_state: obj::STATE_NONE.to_string(),
                        },
                    );

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find structure from {:?}", *structure_id);
                        continue;
                    };

                    let Ok(mut structure) = query.get_mut(structure_entity) else {
                        error!("Query failed to find entity {:?}", structure_entity);
                        continue;
                    };

                    // Set structure state to none
                    *structure.state = State::None;
                    structure.stats.hp = structure.stats.base_hp;

                    let structure_state_event = MapEvent {
                        event_id: Uuid::new_v4(),
                        obj_id: structure.id.0,
                        run_tick: game_tick.0 + 1,
                        event_type: VisibleEvent::StateChangeEvent {
                            new_state: obj::STATE_NONE.to_string(),
                        },
                    };

                    visible_events.push(structure_state_event);
                }
                VisibleEvent::UpgradeEvent {
                    builder_id,
                    structure_id,
                    selected_upgrade,
                } => {
                    debug!(
                        "Processing UpgradeEvent: builder_id {:?}, structure_id: {:?} ",
                        builder_id, structure_id
                    );

                    events_to_remove.push(*map_event_id);

                    let Some(builder_entity) = ids.get_entity(*builder_id) else {
                        error!("Cannot find builder from {:?}", *builder_id);
                        continue;
                    };

                    let Ok(mut builder) = query.get_mut(builder_entity) else {
                        error!("Query failed to find entity {:?}", builder_entity);
                        continue;
                    };

                    *builder.state = State::None;

                    // None visible state change
                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: obj::STATE_NONE.to_string(),
                    };

                    // Builder visible state change
                    let builder_visible_state_change = MapEvent {
                        event_id: Uuid::new_v4(),
                        obj_id: builder.id.0,
                        run_tick: game_tick.0 + 1,
                        event_type: state_change_event.clone(),
                    };

                    visible_events.push(builder_visible_state_change);

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find structure from {:?}", *structure_id);
                        continue;
                    };

                    let Ok(mut structure) = query.get_mut(structure_entity) else {
                        error!("Query failed to find entity {:?}", structure_entity);
                        continue;
                    };

                    //Get current template
                    let current_template =
                        ObjTemplate::get_template_by_name(structure.name.0.clone(), &templates);

                    if let Some(upgrade_to_list) = current_template.upgrade_to {
                        if !upgrade_to_list.contains(selected_upgrade) {
                            error!("Invalid upgrade selection");
                            continue;
                        }

                        let upgrade_template =
                            ObjTemplate::get_template_by_name(selected_upgrade.clone(), &templates);

                        //TODO Fix image code across project
                        let image: String = upgrade_template
                            .template
                            .to_lowercase()
                            .chars()
                            .filter(|c| !c.is_whitespace())
                            .collect();

                        *structure.state = State::None;
                        *structure.name = Name(upgrade_template.name);
                        *structure.template = Template(upgrade_template.template);
                        *structure.class = Class(upgrade_template.class);
                        *structure.subclass = Subclass(upgrade_template.subclass);
                        structure.misc.image = image;

                        //Add obj update event
                        let obj_update_event = VisibleEvent::UpdateObjEvent {
                            attr: obj::TEMPLATE.to_string(),
                            value: structure.template.0.clone(),
                        };

                        // Structure visible templat change
                        let structure_visible_template_change = MapEvent {
                            event_id: Uuid::new_v4(),
                            obj_id: structure.id.0,
                            run_tick: game_tick.0 + 1,
                            event_type: obj_update_event.clone(),
                        };

                        visible_events.push(structure_visible_template_change);
                    } else {
                        error!(
                            "Missing upgrade_to field on template for {:?}",
                            structure.name.0.clone()
                        );
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn gather_event_system(
    clients: Res<Clients>,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    resources: ResMut<Resources>,
    mut items: ResMut<Items>,
    skills: ResMut<Skills>,
    templates: Res<Templates>,
    mut map_events: ResMut<MapEvents>,
    query: Query<ObjQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::GatherEvent { res_type } => {
                    debug!("Processing GatherEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(gatherer_entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find gatherer from {:?}", map_event.obj_id);
                        continue;
                    };

                    let Ok(gatherer) = query.get(gatherer_entity) else {
                        error!("Query failed to find entity {:?}", gatherer_entity);
                        continue;
                    };

                    let capacity =
                        Obj::get_capacity(&gatherer.template.0, &templates.obj_templates);

                    let new_items = Resource::gather_by_type(
                        map_event.obj_id,
                        map_event.obj_id,
                        Position {
                            x: gatherer.pos.x,
                            y: gatherer.pos.y,
                        },
                        res_type.to_string(),
                        &skills,
                        capacity,
                        &mut items,
                        &templates.item_templates,
                        &resources,
                        &templates.res_templates,
                        &mut ids,
                    );

                    if new_items.len() > 0 {
                        let notification_packet: ResponsePacket = ResponsePacket::NewItems {
                            action: obj::STATE_GATHERING.to_string(),
                            sourceid: map_event.obj_id, // Villager Id
                            item_name: new_items[0].name.clone(),
                        };

                        send_to_client(gatherer.player_id.0, notification_packet, &clients);
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn operate_refine_event_system(
    mut commands: Commands,
    clients: Res<Clients>,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    resources: ResMut<Resources>,
    mut items: ResMut<Items>,
    skills: ResMut<Skills>,
    templates: Res<Templates>,
    //mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    //mut state_query: Query<&mut State>,
    mut query: Query<ObjQuery>,
    mut map_events: ResMut<MapEvents>,
    active_infos: Res<ActiveInfos>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::RefineEvent { structure_id } => {
                    info!("Processing RefineEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find entity from structure_id: {:?}", structure_id);
                        continue;
                    };

                    let entities = [entity, structure_entity];

                    let Ok([mut villager, structure]) = query.get_many_mut(entities) else {
                        error!(
                            "Cannot find villager or structure from entities {:?}",
                            entities
                        );
                        continue;
                    };

                    // Remove Event In Progress
                    commands.entity(entity).remove::<EventInProgress>();

                    // Reset villager state to None
                    *villager.state = State::None;

                    let Some(structure_template) = Structure::get_template(
                        structure.template.0.clone(),
                        &templates.obj_templates,
                    ) else {
                        error!(
                            "Query failed to find structure template {:?}",
                            structure.template.0
                        );
                        continue;
                    };

                    let Some(structure_refine_list) = structure_template.refine else {
                        error!(
                            "Missing refine list on structure template {:?}",
                            structure.template.0
                        );
                        continue;
                    };

                    for item_class in structure_refine_list.iter() {
                        debug!("Item class to refine: {:?}", item_class);
                        let item_to_refine = items.get_by_class(*structure_id, item_class.clone());

                        let Some(item_to_refine) = item_to_refine else {
                            continue;
                        };

                        let item_template =
                            Item::get_template(item_to_refine.name, &templates.item_templates);

                        let Some(produces_list) = item_template.produces.clone() else {
                            error!(
                                "Missing item produces attribute for item template {:?}",
                                item_template
                            );
                            continue;
                        };

                        let capacity = Obj::get_capacity(
                            &structure_template.template,
                            &templates.obj_templates,
                        );

                        // Consume item to refine
                        let refined_item = items.remove_quantity(item_to_refine.id, 1);

                        let mut items_to_update: Vec<network::Item> = Vec::new();
                        let mut items_to_remove = Vec::new();

                        // Add item with zero quantity to remove list
                        if let Some(refined_item) = refined_item {
                            let refined_item_packet = Item::to_packet(refined_item);
                            items_to_update.push(refined_item_packet);
                        } else {
                            // Item was removed, add to remove list
                            items_to_remove.push(item_to_refine.id);
                        }

                        // Create new items
                        for produce_item in produces_list.iter() {
                            let current_total_weight = items.get_total_weight(*structure_id);
                            let item_weight = Item::get_weight_from_template(
                                produce_item.to_string(),
                                1,
                                &templates.item_templates,
                            );

                            if current_total_weight + item_weight > capacity {
                                info!("Refining structure is full {:?}", structure);
                                continue;
                            }

                            let (new_item, _merged) = items.new_with_attrs(
                                *structure_id,
                                produce_item.to_string(),
                                1,
                                item_to_refine.attrs.clone(),
                            );

                            // Convert items to be updated to packets
                            let new_item_packet = Item::to_packet(new_item.clone());

                            items_to_update.push(new_item_packet);

                            let notification_packet: ResponsePacket = ResponsePacket::NewItems {
                                action: "refining".to_string(),
                                sourceid: map_event.obj_id, // Villager Id
                                item_name: new_item.name.clone(),
                            };

                            send_to_client(villager.player_id.0, notification_packet, &clients);
                        }

                        let active_info_key = (
                            structure.player_id.0,
                            structure.id.0,
                            "inventory".to_string(),
                        );

                        if let Some(_active_info) = active_infos.get(&active_info_key) {
                            let item_update_packet: ResponsePacket =
                                ResponsePacket::InfoItemsUpdate {
                                    id: *structure_id,
                                    items_updated: items_to_update,
                                    items_removed: items_to_remove,
                                };

                            send_to_client(villager.player_id.0, item_update_packet, &clients);
                        }
                    }
                }
                VisibleEvent::OperateEvent { structure_id } => {
                    info!("Processing OperateEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    // Remove Event In Progress
                    commands.entity(entity).remove::<EventInProgress>();

                    // Set state back to none
                    let Ok(mut villager) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    // Reset villager state to None
                    *villager.state = State::None;

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find entity from structure_id: {:?}", structure_id);
                        continue;
                    };

                    let Ok(structure) = query.get(structure_entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    let res_type = Structure::resource_type(structure.template.0.clone());

                    let capacity =
                        Obj::get_capacity(&structure.template.0, &templates.obj_templates);

                    let items_to_update = Resource::gather_by_type(
                        map_event.obj_id,
                        *structure_id,
                        Position {
                            x: structure.pos.x,
                            y: structure.pos.y,
                        },
                        res_type.to_string(),
                        &skills,
                        capacity,
                        &mut items,
                        &templates.item_templates,
                        &resources,
                        &templates.res_templates,
                        &mut ids,
                    );

                    let active_info_key = (
                        structure.player_id.0,
                        structure.id.0,
                        "inventory".to_string(),
                    );

                    if let Some(_active_info) = active_infos.get(&active_info_key) {
                        let item_update_packet: ResponsePacket = ResponsePacket::InfoItemsUpdate {
                            id: *structure_id,
                            items_updated: items_to_update,
                            items_removed: Vec::new(),
                        };

                        send_to_client(structure.player_id.0, item_update_packet, &clients);
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn craft_event_system(
    mut commands: Commands,
    clients: Res<Clients>,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    _resources: ResMut<Resources>,
    mut items: ResMut<Items>,
    mut skills: ResMut<Skills>,
    templates: Res<Templates>,
    recipes: Res<Recipes>,
    //mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut query: Query<ObjQuery>,
    mut map_events: ResMut<MapEvents>,
    active_infos: Res<ActiveInfos>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::CraftEvent {
                    structure_id,
                    recipe_name,
                } => {
                    info!("Processing CraftEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    let Ok(mut crafter) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    let recipe = recipes.get_by_name(recipe_name.clone());

                    if let Some(mut recipe) = recipe {
                        if Structure::has_req(*structure_id, &mut recipe.req, &mut items) {
                            let consumed_items =
                                Structure::consume_reqs(*structure_id, recipe.req, &mut items);

                            // Reset villager state to None
                            *crafter.state = State::None;

                            // Remove Event In Progress
                            commands.entity(entity).remove::<EventInProgress>();

                            let mut item_attrs = HashMap::new();

                            for consumed_item in consumed_items.iter() {
                                item_attrs.extend(consumed_item.attrs.clone());
                            }

                            // Create new item
                            let new_item = items.craft(
                                *structure_id,
                                recipe_name.to_string(),
                                1,
                                item_attrs,
                                &templates.recipe_templates,
                                None,
                                None,
                            );

                            debug!("recipe: {:?}", recipe.class);
                            let skill_name = Skill::item_class_to_skill(recipe.class);

                            Skill::update(
                                map_event.obj_id,
                                skill_name,
                                100,
                                &mut skills,
                                &templates.skill_templates,
                            );

                            let notification_packet: ResponsePacket = ResponsePacket::NewItems {
                                action: obj::STATE_CRAFTING.to_string(),
                                sourceid: map_event.obj_id, // Crafter Id
                                item_name: new_item.name.clone(),
                            };

                            send_to_client(crafter.player_id.0, notification_packet, &clients);

                            let active_info_key =
                                (crafter.player_id.0, *structure_id, "inventory".to_string());

                            if let Some(_active_info) = active_infos.get(&active_info_key) {
                                let item_update_packet: ResponsePacket =
                                    ResponsePacket::InfoItemsUpdate {
                                        id: *structure_id,
                                        items_updated: vec![Item::to_packet(new_item)],
                                        items_removed: Vec::new(),
                                    };

                                send_to_client(crafter.player_id.0, item_update_packet, &clients);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn experiment_event_system(
    mut commands: Commands,
    clients: Res<Clients>,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    _resources: ResMut<Resources>,
    mut items: ResMut<Items>,
    _skills: ResMut<Skills>,
    templates: Res<Templates>,
    mut recipes: ResMut<Recipes>,
    mut experiments: ResMut<Experiments>,
    mut map_events: ResMut<MapEvents>,
    active_infos: Res<ActiveInfos>,
    //mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut query: Query<ObjQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::ExperimentEvent { structure_id } => {
                    info!("Processing ExperimentEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find structure from {:?}", map_event.obj_id);
                        continue;
                    };

                    let Ok(structure) = query.get(structure_entity) else {
                        error!("Query failed to find entity {:?}", structure_entity);
                        continue;
                    };

                    let structure_name = structure.name.0.clone();

                    let Some(villager_entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find structure from {:?}", map_event.obj_id);
                        continue;
                    };

                    let Ok(_villager) = query.get_mut(villager_entity) else {
                        error!("Query failed to find entity {:?}", villager_entity);
                        continue;
                    };

                    let entities = [villager_entity, structure_entity];

                    let Ok([mut villager, structure]) = query.get_many_mut(entities) else {
                        error!(
                            "Cannot find villager or structure from entities {:?}",
                            entities
                        );
                        continue;
                    };

                    // Reset villager state
                    *villager.state = State::None;

                    // Remove Event In Progress
                    commands.entity(villager_entity).remove::<EventInProgress>();

                    if let Some(experiment) = experiments.get_mut(structure_id) {
                        debug!("Finding experiment recipe... {:?}", experiment.recipe);

                        // If recipe is none, find a valid recipe for experimentation
                        if experiment.recipe == None {
                            let recipe = Experiment::find_recipe(
                                *structure_id,
                                structure_name,
                                &items,
                                &recipes,
                                &templates,
                            );

                            if let Some(recipe) = recipe {
                                Experiment::set_recipe(recipe, experiment);
                            } else {
                                Experiment::set_trivial_source(experiment);
                            }
                        }

                        // Check res reqs
                        debug!("Checking experiment reagents");
                        if Experiment::check_reqs(*structure_id, experiment, &items) {
                            // Check discovery and create new recipe
                            let exp_state = Experiment::check_discovery(
                                structure.player_id.0,
                                *structure_id,
                                experiment,
                                &mut items,
                                &templates.recipe_templates,
                                &mut recipes,
                            );

                            if exp_state == ExperimentState::Discovery {
                                // Remove Villager order
                                commands.entity(villager_entity).remove::<Order>();
                            }

                            player::active_info_experiment(
                                structure.player_id.0,
                                *structure_id,
                                experiment.clone(),
                                &items,
                                &active_infos,
                                &clients,
                            );
                        } else {
                            debug!("Not enough reagents to continue experiment");
                        }
                    } else {
                        error!("No experiment found for {:?}", structure_id);
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn explore_event_system(
    clients: Res<Clients>,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    mut resources: ResMut<Resources>,
    templates: Res<Templates>,
    mut query: Query<(&PlayerId, &Position, &mut State)>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::ExploreEvent => {
                    debug!("Processing ExploreEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    let Ok((player_id, position, mut explorer_state)) = query.get_mut(entity)
                    else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    let pos = Position {
                        x: position.x,
                        y: position.y,
                    };

                    let revealed_resources = Resource::explore(
                        map_event.obj_id,
                        pos,
                        &mut resources,
                        &templates.res_templates,
                    );

                    if revealed_resources.len() > 0 {
                        // Set explorer state to none
                        *explorer_state = State::None;

                        // None visible state change
                        let state_change_event = VisibleEvent::StateChangeEvent {
                            new_state: obj::STATE_NONE.to_string(),
                        };

                        // Builder visible state change
                        let explorer_visible_state_change = MapEvent {
                            event_id: Uuid::new_v4(),
                            obj_id: map_event.obj_id,
                            run_tick: game_tick.0 + 1,
                            event_type: state_change_event.clone(),
                        };

                        visible_events.push(explorer_visible_state_change);

                        let notification_packet: ResponsePacket = ResponsePacket::NewItems {
                            action: obj::STATE_EXPLORING.to_string(),
                            sourceid: map_event.obj_id, // Villager Id
                            item_name: revealed_resources[0].name.clone(),
                        };

                        send_to_client(player_id.0, notification_packet, &clients);
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn farm_event_system(
    mut commands: Commands,
    clients: Res<Clients>,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut crops: ResMut<Crops>,
    resources: ResMut<Resources>,
    mut items: ResMut<Items>,
    skills: ResMut<Skills>,
    templates: Res<Templates>,
    //mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    //mut state_query: Query<&mut State>,
    mut query: Query<ObjQuery>,
    mut map_events: ResMut<MapEvents>,
    active_infos: Res<ActiveInfos>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::PlantEvent { structure_id } => {
                    info!("Processing PlantEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find entity from structure_id: {:?}", structure_id);
                        continue;
                    };

                    let entities = [entity, structure_entity];

                    let Ok([mut villager, structure]) = query.get_many_mut(entities) else {
                        error!(
                            "Cannot find villager or structure from entities {:?}",
                            entities
                        );
                        continue;
                    };

                    // Remove Event In Progress
                    commands.entity(entity).remove::<EventInProgress>();

                    // Reset villager state to None
                    *villager.state = State::None;

                    // Get seeds
                    let seeds = items.get_by_class(*structure_id, item::SEEDS.to_string());

                    // Determine how many seeds the villager can plant TODO
                    let mut seeds_to_plant = 2;

                    let Some(seeds) = seeds else {
                        debug!("No seeds found to plant");
                        continue;
                    };

                    if seeds.quantity < seeds_to_plant {
                        seeds_to_plant = seeds.quantity;                                                
                    }

                    info!("Planting Wheat crops: {:?}", seeds_to_plant);
                    crops.plant(game_tick.0, *structure_id, "Wheat".to_string(), seeds_to_plant);

                    // Consume item to refine
                    let new_seeds = items.remove_quantity(seeds.id, seeds_to_plant);

                    let mut items_to_update: Vec<network::Item> = Vec::new();
                    let mut items_to_remove = Vec::new();

                    // Add item with zero quantity to remove list
                    if let Some(new_seeds) = new_seeds {
                        let new_seeds_packet = Item::to_packet(new_seeds);
                        items_to_update.push(new_seeds_packet);
                    } else {
                        // Item was removed, add to remove list
                        items_to_remove.push(seeds.id);
                    }

                    let active_info_key = (
                        structure.player_id.0,
                        structure.id.0,
                        "inventory".to_string(),
                    );

                    if let Some(_active_info) = active_infos.get(&active_info_key) {
                        let item_update_packet: ResponsePacket =
                            ResponsePacket::InfoItemsUpdate {
                                id: *structure_id,
                                items_updated: items_to_update,
                                items_removed: items_to_remove,
                            };

                        send_to_client(villager.player_id.0, item_update_packet, &clients);
                    }

                }
                VisibleEvent::HarvestEvent { structure_id } => {
                    info!("Processing HarvestEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from id: {:?}", map_event.obj_id);
                        continue;
                    };

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find entity from structure_id: {:?}", structure_id);
                        continue;
                    };

                    let entities = [entity, structure_entity];

                    let Ok([mut villager, structure]) = query.get_many_mut(entities) else {
                        error!(
                            "Cannot find villager or structure from entities {:?}",
                            entities
                        );
                        continue;
                    };

                    // Remove Event In Progress
                    commands.entity(entity).remove::<EventInProgress>();

                    // Reset villager state to None
                    *villager.state = State::None;   

                    if let Some(crop) = crops.harvest(*structure_id, 1) {
                        info!("Harvesting crop: {:?}", crop);
                        let item_template = Item::get_template(crop.crop_type.clone(), &templates.item_templates);

                        let capacity = Obj::get_capacity(&structure.template.0, &templates.obj_templates);

                        let current_total_weight = items.get_total_weight(*structure_id);
                        let item_weight = Item::get_weight_from_template(crop.crop_type.clone(), 1, &templates.item_templates);

                        if current_total_weight + item_weight > capacity {
                            info!("Harvest structure is full {:?}", structure);
                            continue;
                        }

                        let (new_item, _merged) = items.new_with_attrs(
                            *structure_id,
                            crop.crop_type.clone(),
                            1,
                            HashMap::new(),
                        );

                        // Convert items to be updated to packets
                        let new_item_packet = Item::to_packet(new_item.clone());

                        let active_info_key = (
                            structure.player_id.0,
                            structure.id.0,
                            "inventory".to_string(),
                        );

                        if let Some(_active_info) = active_infos.get(&active_info_key) {
                            let item_update_packet: ResponsePacket = ResponsePacket::InfoItemsUpdate {
                                id: *structure_id,
                                items_updated: vec![new_item_packet],
                                items_removed: Vec::new(),
                            };

                            send_to_client(villager.player_id.0, item_update_packet, &clients);
                        }
                    } else {
                        info!("No crops to harvest");
                    }


                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}                

// Each spell requires a separate system
fn spell_raise_dead_event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    pos_query: Query<&Position>,
    mut caster_query: Query<(&mut State, &mut Minions)>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    mut visible_events: ResMut<VisibleEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::SpellRaiseDeadEvent { corpse_id } => {
                    debug!("Processing CastSpellEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(corpse_entity) = ids.get_entity(*corpse_id) else {
                        error!("Cannot find corpse from {:?}", corpse_id);
                        continue;
                    };

                    let Ok(corpse_pos) = pos_query.get(corpse_entity) else {
                        error!("Cannot find corpse position {:?}", corpse_entity);
                        continue;
                    };

                    let Some(caster_entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find caster from {:?}", map_event.obj_id);
                        continue;
                    };

                    let Ok((mut caster_state, mut caster_minions)) =
                        caster_query.get_mut(caster_entity)
                    else {
                        error!("Cannot find caster state {:?}", caster_entity);
                        continue;
                    };

                    // Change state to casting
                    *caster_state = State::None;

                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: obj::STATE_NONE.to_string(),
                    };

                    // Caster visible state change
                    let visible_state_change = MapEvent {
                        event_id: Uuid::new_v4(),
                        obj_id: map_event.obj_id,
                        run_tick: game_tick.0 + 1,
                        event_type: state_change_event.clone(),
                    };

                    visible_events.push(visible_state_change);

                    let minion_id = ids.new_obj_id();

                    // Add to list of minions
                    caster_minions.ids.push(minion_id);

                    let event_type = GameEventType::SpawnNPC {
                        npc_type: "Zombie".to_string(),
                        pos: *corpse_pos,
                        npc_id: Some(minion_id),
                    };

                    let event_id = ids.new_map_event_id();

                    let event = GameEvent {
                        event_id: event_id,
                        run_tick: game_tick.0 + 1, // Add one game tick
                        game_event_type: event_type,
                    };

                    game_events.insert(event.event_id, event);

                    // Remove corpse
                    commands.entity(corpse_entity).despawn();

                    let remove_obj_event = MapEvent {
                        event_id: Uuid::new_v4(),
                        obj_id: *corpse_id,
                        run_tick: game_tick.0 + 1,
                        event_type: VisibleEvent::RemoveObjEvent {
                            pos: corpse_pos.clone(),
                        },
                    };

                    visible_events.push(remove_obj_event);

                    // Add event in progress to caster
                    commands.entity(caster_entity).remove::<EventInProgress>();
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn spell_damage_event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    mut query: Query<CombatSpellQuery>,
    mut map_events: ResMut<MapEvents>,
    _game_events: ResMut<GameEvents>,
    mut visible_events: ResMut<VisibleEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::SpellDamageEvent {
                    spell: _,
                    target_id,
                } => {
                    debug!("Processing CastSpellEvent");
                    events_to_remove.push(*map_event_id);

                    let Some(caster_entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find caster from {:?}", map_event.obj_id);
                        continue;
                    };

                    let Some(target_entity) = ids.get_entity(*target_id) else {
                        error!("Cannot find caster from {:?}", target_id);
                        continue;
                    };

                    let entities = [caster_entity, target_entity];

                    let Ok([mut caster, mut target]) = query.get_many_mut(entities) else {
                        error!("Cannot find caster or target from entities {:?}", entities);
                        continue;
                    };

                    if Obj::is_dead(&caster.state) {
                        continue;
                    }

                    // Process spell damage
                    Combat::process_spell_damage(&mut commands, &game_tick, &mut target);

                    let target_state_str = Obj::state_to_str(target.state.clone());

                    let damage_event = VisibleEvent::DamageEvent {
                        target_id: target.id.0,
                        target_pos: target.pos.clone(),
                        attack_type: "Shadow Bolt".to_string(),
                        damage: 1,
                        combo: None,
                        state: target_state_str,
                    };

                    let damage_map_event = MapEvent {
                        event_id: Uuid::new_v4(),
                        obj_id: map_event.obj_id,
                        run_tick: game_tick.0 + 1,
                        event_type: damage_event.clone(),
                    };

                    visible_events.push(damage_map_event);

                    // Change state to casting
                    *caster.state = State::None;

                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: obj::STATE_NONE.to_string(),
                    };

                    // Caster visible state change
                    let visible_state_change = MapEvent {
                        event_id: Uuid::new_v4(),
                        obj_id: map_event.obj_id,
                        run_tick: game_tick.0 + 1,
                        event_type: state_change_event.clone(),
                    };

                    visible_events.push(visible_state_change);

                    // Add event in progress to caster
                    commands.entity(caster.entity).remove::<EventInProgress>();
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn broadcast_event_system(
    game_tick: Res<GameTick>,
    mut visible_events: ResMut<VisibleEvents>,
    mut map_events: ResMut<MapEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::DamageEvent { .. } => {
                    debug!("Processing DamageEvent");
                    events_to_remove.push(*map_event_id);
                    visible_events.push(map_event.clone());
                }
                VisibleEvent::SoundObjEvent { .. } => {
                    debug!("Processing SoundObjEvent");
                    events_to_remove.push(*map_event_id);
                    visible_events.push(map_event.clone());
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn effect_expired_event_system(
    game_tick: Res<GameTick>,
    ids: Res<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut effect_query: Query<&mut Effects>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::EffectExpiredEvent { effect } => {
                    debug!("Processing EffectExpiredEvent {:?}", effect);
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find entity from {:?}", map_event.obj_id);
                        continue;
                    };

                    if let Ok(mut effects) = effect_query.get_mut(entity) {
                        debug!("Effects on {:?}", map_event.obj_id);
                        effects.0.remove(effect);
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn cooldown_event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    ids: Res<Ids>,
    mut map_events: ResMut<MapEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::CooldownEvent { duration } => {
                    debug!("Processing CooldownEvent {:?}", duration);
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(map_event.obj_id) else {
                        error!("Cannot find corpse from {:?}", map_event.obj_id);
                        continue;
                    };

                    //TODO why isn't the state reset to none?
                    // Set state back to none
                    /*let Ok(mut obj_state) = query.get_mut(map_event.entity_id) else {
                        error!("Query failed to find entity {:?}", map_event.entity_id);
                        continue;
                    };*/

                    commands.entity(entity).remove::<EventInProgress>();
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn use_item_system(
    game_tick: Res<GameTick>,
    clients: Res<Clients>,
    ids: ResMut<Ids>,
    templates: Res<Templates>,
    mut items: ResMut<Items>,
    mut plans: ResMut<Plans>,
    _visible_events: ResMut<VisibleEvents>,
    mut map_events: ResMut<MapEvents>,
    mut query: Query<ObjWithStatsQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::UseItemEvent {
                    item_id,
                    item_owner_id,
                } => {
                    debug!("Processing UseItemEvent {:?}", item_id);
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(*item_owner_id) else {
                        error!("Cannot find item owner entity from id: {:?}", item_owner_id);
                        continue;
                    };

                    let Some(item) = items.find_by_id(*item_id) else {
                        debug!("Failed to find item: {:?}", item_id);
                        continue;
                    };

                    let Ok(mut item_owner) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    match (item.class.as_str(), item.subclass.as_str()) {
                        (item::POTION, item::HEALTH) => {
                            let healing_attrval = item
                                .attrs
                                .get(&item::AttrKey::Healing)
                                .expect("Missing Healing attribute.");

                            debug!("Healing AttrVal: {:?}", healing_attrval);

                            let healing_value = match healing_attrval {
                                item::AttrVal::Num(val) => *val as i32,
                                _ => panic!("Invalid healing attribute value"),
                            };

                            if item_owner.stats.hp < item_owner.stats.base_hp {
                                if (item_owner.stats.hp + healing_value) > item_owner.stats.base_hp
                                {
                                    item_owner.stats.hp = item_owner.stats.base_hp;
                                } else {
                                    item_owner.stats.hp += healing_value;
                                }

                                debug!("Entity: {:?} Hp: {:?}", item_owner_id, item_owner.stats.hp);

                                let packet = ResponsePacket::Stats {
                                    data: StatsData {
                                        id: *item_owner_id,
                                        hp: item_owner.stats.hp,
                                        base_hp: item_owner.stats.base_hp,
                                        stamina: 10000, // TODO missing stamina
                                        base_stamina: 10000,
                                        effects: Vec::new(),
                                    },
                                };

                                send_to_client(item_owner.player_id.0, packet, &clients);
                            }
                        }
                        (item::DEED, _) => {
                            plans.add(item.owner, item.subclass, 0, 0);

                            items.remove_item(item.id);

                            let inventory_items = items.get_by_owner_packet(item.owner);

                            let info_inventory_packet: ResponsePacket =
                                ResponsePacket::InfoInventory {
                                    id: item.owner,
                                    cap: Obj::get_capacity(
                                        &item_owner.template.0,
                                        &templates.obj_templates,
                                    ),
                                    tw: items.get_total_weight(item.owner),
                                    items: inventory_items,
                                };

                            send_to_client(item.owner, info_inventory_packet, &clients);

                            let packet = ResponsePacket::Error {
                                errmsg: format!("You have learnt how to build a {:?}", item.name),
                            };

                            send_to_client(item.owner, packet, &clients);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn drink_eat_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    mut visible_events: ResMut<VisibleEvents>,
    mut map_events: ResMut<MapEvents>,
    mut thirsts: Query<&mut Thirst>,
    mut hungers: Query<&mut Hunger>,
    mut query: Query<ObjQuery>,
    _villager_attrs: Query<&mut VillagerAttrs>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.event_type {
                VisibleEvent::DrinkEvent { item_id, obj_id } => {
                    debug!("Processing DrinkEvent {:?}", item_id);
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(*obj_id) else {
                        error!("Cannot find item owner entity from id: {:?}", obj_id);
                        continue;
                    };

                    let Some(item) = items.find_by_id(*item_id) else {
                        debug!("Failed to find item: {:?}", item_id);
                        continue;
                    };

                    let Ok(mut obj) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    *obj.state = State::None;

                    commands.entity(entity).remove::<EventInProgress>();

                    // None visible state change
                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: obj::STATE_NONE.to_string(),
                    };

                    let drinking_visible_event = MapEvent {
                        event_id: Uuid::new_v4(),
                        obj_id: map_event.obj_id,
                        run_tick: game_tick.0 + 1,
                        event_type: state_change_event.clone(),
                    };

                    debug!(
                        "Removed EventInProgress {:?} and set State back to None",
                        map_event_id
                    );
                    visible_events.push(drinking_visible_event);

                    // If villager reset the activity to none
                    if obj.subclass.0 == obj::SUBCLASS_VILLAGER {
                        debug!("Inserting DrinkEventCompleted");
                        commands
                            .entity(entity)
                            .insert(DrinkEventCompleted { item: item });
                    } else if obj.subclass.0 == obj::SUBCLASS_HERO {
                        if let Ok(mut thirst) = thirsts.get_mut(obj.entity) {
                            if let Some(thirst_attrval) = item.attrs.get(&item::AttrKey::Thirst) {
                                let thirst_value = match thirst_attrval {
                                    item::AttrVal::Num(val) => *val,
                                    _ => panic!("Invalid thirst attribute value"),
                                };

                                thirst.thirst -= thirst_value;

                                items.update_quantity_by_class(
                                    *obj_id,
                                    item::WATER.to_string(),
                                    -1,
                                );
                            }
                        }
                    }
                }
                VisibleEvent::EatEvent { item_id, obj_id } => {
                    debug!("Processing EatEvent {:?}", item_id);
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(*obj_id) else {
                        error!("Cannot find item owner entity from id: {:?}", obj_id);
                        continue;
                    };

                    let Some(item) = items.find_by_id(*item_id) else {
                        debug!("Failed to find item: {:?}", item_id);
                        continue;
                    };

                    let Ok(mut obj) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    *obj.state = State::None;

                    commands.entity(entity).remove::<EventInProgress>();

                    // None visible state change
                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: obj::STATE_NONE.to_string(),
                    };

                    let eating_visible_event = MapEvent {
                        event_id: Uuid::new_v4(),
                        obj_id: map_event.obj_id,
                        run_tick: game_tick.0 + 1,
                        event_type: state_change_event.clone(),
                    };

                    debug!(
                        "Removed EventInProgress {:?} and set State back to None",
                        map_event_id
                    );
                    visible_events.push(eating_visible_event);

                    // If villager reset the activity to none
                    if obj.subclass.0 == obj::SUBCLASS_VILLAGER {
                        debug!("Inserting DrinkEventCompleted");
                        commands
                            .entity(entity)
                            .insert(EatEventCompleted { item: item });
                    } else if obj.subclass.0 == obj::SUBCLASS_HERO {
                        if let Ok(mut hunger) = hungers.get_mut(obj.entity) {
                            if let Some(feed_attrval) = item.attrs.get(&item::AttrKey::Feed) {
                                let feed_value = match feed_attrval {
                                    item::AttrVal::Num(val) => *val,
                                    _ => panic!("Invalid feed attribute value"),
                                };

                                hunger.hunger -= feed_value;

                                items.update_quantity_by_class(*obj_id, item::FOOD.to_string(), -1);
                            }
                        }
                    }
                }
                VisibleEvent::SleepEvent { obj_id } => {
                    debug!("Processing SleepEvent {:?}", obj_id);
                    events_to_remove.push(*map_event_id);

                    let Some(entity) = ids.get_entity(*obj_id) else {
                        error!("Cannot find entity from id: {:?}", obj_id);
                        continue;
                    };

                    let Ok(mut obj) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    *obj.state = State::None;

                    commands.entity(obj.entity).remove::<EventInProgress>();

                    // None visible state change
                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: obj::STATE_NONE.to_string(),
                    };

                    let sleep_visible_event = MapEvent {
                        event_id: Uuid::new_v4(),
                        obj_id: map_event.obj_id,
                        run_tick: game_tick.0 + 1,
                        event_type: state_change_event.clone(),
                    };

                    debug!(
                        "Removed EventInProgress {:?} and set State back to None",
                        map_event_id
                    );
                    visible_events.push(sleep_visible_event);

                    if obj.subclass.0 == obj::SUBCLASS_VILLAGER {
                        debug!("Inserting DrinkEventCompleted");
                        commands.entity(entity).insert(SleepEventCompleted);
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn visible_event_system(
    clients: Res<Clients>,
    mut visible_events: ResMut<VisibleEvents>,
    ids: Res<Ids>,
    map_obj_query: Query<MapObjQuery>,
) {
    // TODO explore using traits in the HashSet to reduce code
    let mut all_change_events: HashMap<i32, HashSet<network::ChangeEvents>> = HashMap::new();
    let mut all_broadcast_events: HashMap<i32, HashSet<BroadcastEvents>> = HashMap::new();

    for map_event in visible_events.iter() {
        debug!("Checking if map_event is visible: {:?}", map_event);

        let Some(entity) = ids.get_entity(map_event.obj_id) else {
            error!("Cannot entity from id: {:?}", map_event.obj_id);
            continue;
        };

        if let Ok(event_obj) = map_obj_query.get(entity) {
            let network_obj = network::create_network_obj(&event_obj);

            for observer in map_obj_query.iter() {
                match &map_event.event_type {
                    VisibleEvent::NewObjEvent { new_player: _ } => {
                        let distance = Map::distance(
                            (event_obj.pos.x, event_obj.pos.y),
                            (observer.pos.x, observer.pos.y),
                        );

                        if observer.viewshed.range >= distance {
                            debug!("Send obj create to client");

                            let change_event = network::ChangeEvents::ObjCreate {
                                event: "obj_create".to_string(),
                                obj: network_obj.to_owned(),
                            };

                            // Notify observer
                            all_change_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(change_event);
                        }
                    }
                    VisibleEvent::MoveEvent { src, dst } => {
                        let src_distance = Map::dist(*observer.pos, *src);

                        if observer.viewshed.range >= src_distance {
                            let change_event = network::ChangeEvents::ObjMove {
                                event: "obj_move".to_string(),
                                obj: network_obj.to_owned(),
                                src_x: dst.x,
                                src_y: dst.y,
                            };

                            // Notify observer
                            all_change_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(change_event);
                        }

                        let dst_distance = Map::dist(*observer.pos, *dst);

                        if observer.viewshed.range >= dst_distance {
                            let change_event = network::ChangeEvents::ObjMove {
                                event: "obj_move".to_string(),
                                obj: network_obj.to_owned(),
                                src_x: dst.x,
                                src_y: dst.y,
                            };

                            all_change_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(change_event);
                        }
                    }
                    VisibleEvent::HideEvent => {
                        let distance = Map::dist(*event_obj.pos, *observer.pos);

                        if observer.viewshed.range >= distance {
                            debug!("Send obj delete to client");

                            let change_event = network::ChangeEvents::ObjDelete {
                                event: "obj_delete".to_string(),
                                obj_id: map_event.obj_id,
                            };

                            all_change_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(change_event);
                        }
                    }
                    VisibleEvent::DamageEvent {
                        target_id,
                        target_pos,
                        attack_type,
                        damage,
                        combo,
                        state,
                    } => {
                        debug!("Processing DamageEvent: {:?}", &map_event.event_type);
                        let attacker_distance = Map::distance(
                            (event_obj.pos.x, event_obj.pos.y),
                            (observer.pos.x, observer.pos.y),
                        );

                        if observer.viewshed.range >= attacker_distance {
                            let damage_event = BroadcastEvents::Damage {
                                sourceid: map_event.obj_id,
                                targetid: *target_id,
                                attacktype: attack_type.to_string(),
                                dmg: *damage,
                                state: state.to_string(),
                                combo: combo.clone(),
                                countered: None,
                            };

                            all_broadcast_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(damage_event);
                        }

                        let target_distance = Map::distance(
                            (target_pos.x, target_pos.y),
                            (observer.pos.x, observer.pos.y),
                        );

                        if observer.viewshed.range >= target_distance {
                            let damage_event = BroadcastEvents::Damage {
                                sourceid: map_event.obj_id,
                                targetid: *target_id,
                                attacktype: attack_type.to_string(),
                                dmg: *damage,
                                state: state.to_string(),
                                combo: combo.clone(),
                                countered: None,
                            };

                            all_broadcast_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(damage_event);
                        }
                    }
                    VisibleEvent::SoundObjEvent { sound, intensity } => {
                        debug!("Processing SoundObjEvent: {:?}", &map_event.event_type);
                        let distance = Map::distance(
                            (event_obj.pos.x, event_obj.pos.y),
                            (observer.pos.x, observer.pos.y),
                        );

                        if *intensity >= distance as i32 {
                            let sound_obj_event = BroadcastEvents::SoundObjEvent {
                                source: map_event.obj_id,
                                text: sound.clone(),
                            };

                            all_broadcast_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(sound_obj_event);
                        }
                    }
                    VisibleEvent::StateChangeEvent { new_state } => {
                        let distance = Map::distance(
                            (event_obj.pos.x, event_obj.pos.y),
                            (observer.pos.x, observer.pos.y),
                        );

                        if observer.viewshed.range >= distance {
                            debug!("Send obj update to client");

                            let change_event = network::ChangeEvents::ObjUpdate {
                                event: "obj_update".to_string(),
                                obj_id: map_event.obj_id,
                                attr: "state".to_string(),
                                value: new_state.clone(),
                            };

                            all_change_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(change_event);
                        }
                    }
                    VisibleEvent::UpdateObjEvent { attr, value } => {
                        let distance = Map::distance(
                            (event_obj.pos.x, event_obj.pos.y),
                            (observer.pos.x, observer.pos.y),
                        );

                        if observer.viewshed.range >= distance {
                            debug!("Send obj update to client");

                            let change_event = network::ChangeEvents::ObjUpdate {
                                event: "obj_update".to_string(),
                                obj_id: map_event.obj_id,
                                attr: attr.to_string(),
                                value: value.clone(),
                            };

                            all_change_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(change_event);
                        }
                    }
                    VisibleEvent::UpdateObjPosEvent { src, dst } => {
                        let src_distance = Map::dist(*observer.pos, *src);

                        if observer.viewshed.range >= src_distance {
                            let change_event = network::ChangeEvents::ObjMove {
                                event: "obj_move".to_string(),
                                obj: network_obj.to_owned(),
                                src_x: dst.x,
                                src_y: dst.y,
                            };

                            // Notify observer
                            all_change_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(change_event);
                        }

                        let dst_distance = Map::dist(*observer.pos, *dst);

                        if observer.viewshed.range >= dst_distance {
                            let change_event = network::ChangeEvents::ObjMove {
                                event: "obj_move".to_string(),
                                obj: network_obj.to_owned(),
                                src_x: dst.x,
                                src_y: dst.y,
                            };

                            all_change_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(change_event);
                        }
                    }
                    _ => {}
                }
            }
        } else {
            debug!(
                "VisibleEventSystem no entity found, assuming removed: {:?}",
                entity
            );
            for observer in map_obj_query.iter() {
                match &map_event.event_type {
                    VisibleEvent::RemoveObjEvent { pos } => {
                        let distance =
                            Map::distance((pos.x, pos.y), (observer.pos.x, observer.pos.y));

                        if observer.viewshed.range >= distance {
                            let change_event = network::ChangeEvents::ObjDelete {
                                event: "obj_delete".to_string(),
                                obj_id: map_event.obj_id,
                            };

                            all_change_events
                                .entry(observer.player_id.0)
                                .or_default()
                                .insert(change_event);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    for (player_id, change_events) in all_change_events.iter_mut() {
        let changes_packet = ResponsePacket::Changes {
            events: change_events.clone().into_iter().collect(),
        };

        for (_client_id, client) in clients.lock().unwrap().iter() {
            if client.player_id == *player_id {
                debug!("Changes: {:?}", changes_packet);
                client
                    .sender
                    .try_send(serde_json::to_string(&changes_packet).unwrap())
                    .expect("Could not send message");
            }
        }
    }

    // TODO reconsider these 3 loops
    for (player_id, broadcast_events) in all_broadcast_events.iter_mut() {
        for (_client_id, client) in clients.lock().unwrap().iter() {
            if client.player_id == *player_id {
                for broadcast_event in broadcast_events.iter() {
                    client
                        .sender
                        .try_send(serde_json::to_string(&broadcast_event).unwrap())
                        .expect("Could not send message");
                }
            }
        }
    }

    visible_events.clear();
}

// TODO refactor to use WorldQuery
fn perception_system(
    map: Res<Map>,
    mut explored_map: ResMut<ExploredMap>,
    weather_areas: Res<WeatherAreas>,
    clients: Res<Clients>,
    mut perception_updates: ResMut<PerceptionUpdates>,
    query: Query<(
        &Id,
        &PlayerId,
        &Position,
        &Name,
        &Template,
        &Class,
        &Subclass,
        &State,
        &Viewshed,
        &Misc,
    )>,
) {
    let mut perceptions_to_send: HashMap<i32, HashSet<network::MapObj>> = HashMap::new();
    // Could not use HashSet here due to the trait `FromIterator<&std::collections::HashSet<(i32, i32)>>` is not implemented for `Vec<(i32, i32)>`
    let mut tiles_to_send: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();

    for perception_player in perception_updates.iter() {
        for [obj1, obj2] in query.iter_combinations() {
            let (id1, player1, pos1, name1, template1, class1, subclass1, state1, viewshed1, misc1) =
                obj1;
            let (id2, player2, pos2, name2, template2, class2, subclass2, state2, viewshed2, misc2) =
                obj2;

            // Check if obj1 is owned by perception_player
            if *perception_player == player1.0 {
                let distance = Map::distance((pos1.x, pos1.y), (pos2.x, pos2.y));

                if viewshed1.range >= distance && Obj::is_visible(state2.clone()) {
                    debug!("Adding visible obj to percetion");

                    let visible_obj = network_obj(
                        id2.0,
                        player2.0,
                        pos2.x,
                        pos2.y,
                        name2.0.to_owned(),
                        template2.0.to_owned(),
                        class2.0.to_owned(),
                        subclass2.0.to_owned(),
                        Obj::state_to_str(state2.to_owned()),
                        viewshed2.range,
                        misc2.image.to_owned(),
                        misc2.hsl.to_owned(),
                        misc2.groups.to_owned(),
                    );

                    perceptions_to_send
                        .entry(*perception_player)
                        .or_default()
                        .insert(visible_obj);
                }

                // Add observer to perception data
                let observer_obj = network_obj(
                    id1.0,
                    player1.0,
                    pos1.x,
                    pos1.y,
                    name1.0.to_owned(),
                    template1.0.to_owned(),
                    class1.0.to_owned(),
                    subclass1.0.to_owned(),
                    Obj::state_to_str(state1.to_owned()),
                    viewshed1.range,
                    misc1.image.to_owned(),
                    misc1.hsl.to_owned(),
                    misc1.groups.to_owned(),
                );

                perceptions_to_send
                    .entry(*perception_player)
                    .or_default()
                    .insert(observer_obj);

                // Get visible tiles by player owned obj
                let visible_tiles_pos = Map::range((pos1.x, pos1.y), viewshed1.range);

                // Add explored map
                match explored_map.entry(*perception_player) {
                    Entry::Occupied(mut o) => {
                        o.get_mut().extend(visible_tiles_pos.clone());
                        o.get_mut().sort_unstable();
                        o.get_mut().dedup();
                    }
                    Entry::Vacant(v) => {
                        v.insert(visible_tiles_pos.clone());
                    }
                };

                tiles_to_send
                    .entry(*perception_player)
                    .or_default()
                    .extend(visible_tiles_pos);
            }

            // Check if obj2 is owned by perception_player
            if *perception_player == player2.0 {
                let distance = Map::distance((pos1.x, pos1.y), (pos2.x, pos2.y));

                if viewshed2.range >= distance && Obj::is_visible(state1.clone()) {
                    println!("Adding visible obj to percetion");

                    let visible_obj = network_obj(
                        id1.0,
                        player1.0,
                        pos1.x,
                        pos1.y,
                        name1.0.to_owned(),
                        template1.0.to_owned(),
                        class1.0.to_owned(),
                        subclass1.0.to_owned(),
                        Obj::state_to_str(state1.to_owned()),
                        viewshed1.range,
                        misc1.image.to_owned(),
                        misc1.hsl.to_owned(),
                        misc1.groups.to_owned(),
                    );

                    perceptions_to_send
                        .entry(*perception_player)
                        .or_default()
                        .insert(visible_obj);
                }

                // Add observer to perception data
                let observer_obj = network_obj(
                    id2.0,
                    player2.0,
                    pos2.x,
                    pos2.y,
                    name2.0.to_owned(),
                    template2.0.to_owned(),
                    class2.0.to_owned(),
                    subclass2.0.to_owned(),
                    Obj::state_to_str(state2.to_owned()),
                    viewshed2.range,
                    misc2.image.to_owned(),
                    misc2.hsl.to_owned(),
                    misc2.groups.to_owned(),
                );

                perceptions_to_send
                    .entry(*perception_player)
                    .or_default()
                    .insert(observer_obj);

                // Get visible tiles by player owned obj
                let visible_tiles_pos = Map::range((pos2.x, pos2.y), viewshed2.range);

                // Add explored map
                match explored_map.entry(*perception_player) {
                    Entry::Occupied(mut o) => {
                        o.get_mut().extend(visible_tiles_pos.clone());
                        o.get_mut().sort_unstable();
                        o.get_mut().dedup();
                    }
                    Entry::Vacant(v) => {
                        v.insert(visible_tiles_pos.clone());
                    }
                };

                tiles_to_send
                    .entry(*perception_player)
                    .or_default()
                    .extend(visible_tiles_pos);
            }
        }

        for (player_id, perception) in perceptions_to_send.iter_mut() {
            println!(
                "Perceptions to send player: {:?} perception: {:?}",
                player_id, perception
            );
            let mut visible_tiles: &mut Vec<(i32, i32)> = tiles_to_send.get_mut(player_id).unwrap();

            dedup(&mut visible_tiles);

            let tiles = Map::pos_to_tiles(&visible_tiles.clone(), &map); // Used for network obj

            let weather_tiles = weather_areas.get_visible_weather_tiles(&visible_tiles.clone());

            let perception_data = network::PerceptionData {
                map: tiles,
                objs: perception.clone().into_iter().collect(),
                weather: weather_tiles,
            };

            let perception_packet = ResponsePacket::Perception {
                data: perception_data,
            };

            debug!("clients: {:?}", clients);
            for (_client_id, client) in clients.lock().unwrap().iter() {
                if client.player_id == *player_id {
                    client
                        .sender
                        //TODO handle disconnection
                        .try_send(serde_json::to_string(&perception_packet).unwrap())
                        .expect("Could not send message");
                }
            }
        }
    }

    perception_updates.clear();
}

fn game_event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
    mut game_events: ResMut<GameEvents>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    mut structure_attrs_query: Query<&mut StructureAttrs>,
    mut query: Query<ObjQueryMut>,
    mut perception_updates: ResMut<PerceptionUpdates>,
) {
    let mut events_to_remove = Vec::new();

    for (event_id, game_event_type) in game_events.iter_mut() {
        if game_event_type.run_tick < game_tick.0 {
            // Execute event
            match &game_event_type.game_event_type {
                GameEventType::Login { player_id } => {
                    events_to_remove.push(*event_id);
                    perception_updates.insert(*player_id);
                }
                GameEventType::SpawnNPC {
                    npc_type,
                    pos,
                    npc_id,
                } => {
                    debug!("Processing SpawnNPC");
                    events_to_remove.push(*event_id);

                    let result;

                    if let Some(npc_id) = npc_id {
                        result = Encounter::spawn_npc_with_id(
                            *npc_id,
                            1000,
                            *pos,
                            npc_type.to_string(),
                            &mut commands,
                            &mut ids,
                            &mut items,
                            &templates,
                        );
                    } else {
                        result = Encounter::spawn_npc(
                            1000,
                            *pos,
                            npc_type.to_string(),
                            &mut commands,
                            &mut ids,
                            &mut items,
                            &templates,
                        );
                    }

                    let (_entity, npc_id, _player_id, _pos) = result;

                    map_events.new(
                        npc_id.0,
                        game_tick.0 + 1,
                        VisibleEvent::NewObjEvent { new_player: false },
                    );
                }
                GameEventType::UpdatePos { obj_id, pos } => {
                    debug!("Processing UpdatePos");
                    events_to_remove.push(*event_id);

                    // Update object position
                    let Some(entity) = ids.get_entity(*obj_id) else {
                        error!("Cannot find entity from id: {:?}", obj_id);
                        continue;
                    };

                    let Ok(mut obj) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    // Store src for event visibility check
                    let src_pos = *obj.pos;

                    // Update position
                    *obj.pos = pos.clone();

                    visible_events.new(
                        *obj_id,
                        game_tick.0 + 1,
                        VisibleEvent::UpdateObjPosEvent {
                            src: src_pos,
                            dst: *pos,
                        },
                    );
                }
                GameEventType::NecroEvent { pos, home } => {
                    debug!("Processing NecroEvent");
                    events_to_remove.push(*event_id);

                    let (_entity, npc_id, _player_id, _pos) = Encounter::spawn_necromancer(
                        1000,
                        *pos,
                        *home,
                        &mut commands,
                        &mut ids,
                        &mut items,
                        &templates,
                    );

                    map_events.new(
                        npc_id.0,
                        game_tick.0 + 1,
                        VisibleEvent::NewObjEvent { new_player: false },
                    );
                }
                GameEventType::CancelEvents { event_ids } => {
                    debug!("Processing CancelEvents: {:?}", event_ids);
                    events_to_remove.push(*event_id);

                    let mut events_to_cancel = Vec::new();

                    for event_id in event_ids.iter() {
                        if let Some(event) = map_events.get(event_id) {
                            events_to_cancel.push(event.clone());
                        }
                    }

                    debug!("Canceling map events: {:?}", events_to_cancel);
                    for map_event in events_to_cancel.iter() {
                        match map_event.event_type {
                            VisibleEvent::BuildEvent {
                                builder_id: _,
                                structure_id,
                            } => {
                                //TODO: should be able to change state without the need for entity, playerid and position

                                let Some(structure_entity) = ids.get_entity(structure_id) else {
                                    error!(
                                        "Cannot find entity from structure_id: {:?}",
                                        structure_id
                                    );
                                    continue;
                                };

                                // Set state back to none
                                let Ok(mut structure) = query.get_mut(structure_entity) else {
                                    error!("Query failed to find entity {:?}", structure_entity);
                                    continue;
                                };

                                let Ok(mut structure_attrs) =
                                    structure_attrs_query.get_mut(structure_entity)
                                else {
                                    error!(
                                        "Cannot query structure attrs of {:?}",
                                        structure_entity
                                    );
                                    continue;
                                };

                                let structure_template = ObjTemplate::get_template_by_name(
                                    structure.name.0.clone(),
                                    &templates,
                                );
                                let structure_build_time = structure_template
                                    .build_time
                                    .expect("Template should have build_time field");

                                *structure.state = State::Stalled;

                                let ratio = (game_tick.0 - structure_attrs.start_time) as f32
                                    / structure_build_time as f32;

                                debug!("Ratio: {:?}", ratio);

                                structure_attrs.progress = (ratio * 100.0).round() as i32;

                                debug!("Progress: {:?}", structure_attrs.progress);

                                let new_obj_event = VisibleEvent::StateChangeEvent {
                                    new_state: obj::STATE_STALLED.to_string(),
                                };

                                //TODO add a visible events new trait
                                let event = MapEvent {
                                    event_id: Uuid::new_v4(),
                                    obj_id: structure.id.0,
                                    run_tick: game_tick.0 + 1, // Add one game tick
                                    event_type: new_obj_event,
                                };

                                visible_events.push(event);
                            }
                            _ => {
                                let Some(entity) = ids.get_entity(map_event.obj_id) else {
                                    error!(
                                        "Cannot find item owner entity from id: {:?}",
                                        map_event.obj_id
                                    );
                                    continue;
                                };

                                let Ok(mut obj) = query.get_mut(entity) else {
                                    error!("Query failed to find entity {:?}", entity);
                                    continue;
                                };

                                debug!("Cancel event - reseting obj state to none.");
                                *obj.state = State::None;

                                debug!(
                                    "Cancel event - removing EventInProgress for entity: {:?}",
                                    obj.entity
                                );
                                commands.entity(obj.entity).remove::<EventInProgress>();

                                /*debug!("Cancel event - removing drink, eat, sleep completed events {:?}", map_event.entity_id);
                                commands
                                    .entity(map_event.entity_id)
                                    .remove::<DrinkEventCompleted>()
                                    .remove::<EatEventCompleted>()
                                    .remove::<SleepEventCompleted>();  */

                                // None visible state change
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: obj::STATE_NONE.to_string(),
                                };

                                let visible_event = MapEvent {
                                    event_id: Uuid::new_v4(),
                                    obj_id: map_event.obj_id,
                                    run_tick: game_tick.0 + 1,
                                    event_type: state_change_event.clone(),
                                };

                                visible_events.push(visible_event);
                            }
                        }
                    }

                    debug!("Removing map events {:?} from queue", event_ids);
                    for event_id in event_ids.iter() {
                        map_events.remove(event_id);
                    }
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        game_events.remove(event_id);
    }
}

fn resurrect_system(
    mut commands: Commands,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    templates: Res<Templates>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    game_tick: Res<GameTick>,
    mut items: ResMut<Items>,
    mut hero_query: Query<ObjWithStatsQuery, (With<StateDead>, With<SubclassHero>)>,
    dead_state_query: Query<&StateDead>,
) {
    for mut hero in hero_query.iter_mut() {
        let Ok(dead_state) = dead_state_query.get(hero.entity) else {
            error!("No dead state found for entity: {:?}", hero.entity);
            continue;
        };

        if (game_tick.0 - dead_state.dead_at) > 100 {
            debug!("Resurrecting hero {:?}", hero.id);

            // Create human corpse
            let (corpse_id, _entity) = Obj::create(
                hero.player_id.0,
                "Human Corpse".to_string(),
                *hero.pos,
                State::Dead,
                &mut commands,
                &mut ids,
                &mut map_events,
                &game_tick,
                &templates,
            );

            // Transfer all items to corpse
            items.transfer_all_items(hero.id.0, corpse_id);

            //Reset hp & state
            hero.stats.hp = hero.stats.base_hp;
            *hero.state = State::None;

            //TODO replace with monolith location
            let src = hero.pos.clone();
            let dst = Position { x: 16, y: 36 };

            *hero.pos = dst.clone();

            commands.entity(hero.entity).remove::<StateDead>();

            let packet = ResponsePacket::Stats {
                data: StatsData {
                    id: hero.id.0,
                    hp: hero.stats.hp,
                    base_hp: hero.stats.base_hp,
                    stamina: hero.stats.stamina.unwrap_or(100),
                    base_stamina: hero.stats.base_stamina.unwrap_or(100),
                    effects: Vec::new(),
                },
            };

            send_to_client(hero.player_id.0, packet, &clients);

            // None visible state change
            let state_change_event = VisibleEvent::StateChangeEvent {
                new_state: obj::STATE_NONE.to_string(),
            };

            // State change
            let state_change = MapEvent {
                event_id: Uuid::new_v4(),
                obj_id: hero.id.0,
                run_tick: game_tick.0 + 5,
                event_type: state_change_event.clone(),
            };

            visible_events.push(state_change);

            // Move event
            let move_event = VisibleEvent::MoveEvent { src: src, dst: dst };

            // Move change
            let move_map_event = MapEvent {
                event_id: Uuid::new_v4(),
                obj_id: hero.id.0,
                run_tick: game_tick.0 + 2,
                event_type: move_event.clone(),
            };

            visible_events.push(move_map_event);
        }
    }
}

fn remove_dead_system(
    game_tick: ResMut<GameTick>,
    dead_state_query: Query<(&Id, &Position, &StateDead)>,
    items: ResMut<Items>,
    mut map_events: ResMut<MapEvents>,
) {
    // Every 10 ticks
    if (game_tick.0 % 10) == 0 {
        for (id, pos, dead_state) in dead_state_query.iter() {
            if (game_tick.0 - dead_state.dead_at) > 500 {
                map_events.new(
                    id.0,
                    game_tick.0 + 1,
                    VisibleEvent::RemoveObjEvent {
                        pos: pos.to_owned(),
                    },
                );
            } else if (game_tick.0 - dead_state.dead_at) > 100 {
                // Remove dead object faster if it contains no items
                if items.get_by_owner(id.0).is_empty() {
                    map_events.new(
                        id.0,
                        game_tick.0 + 1,
                        VisibleEvent::RemoveObjEvent {
                            pos: pos.to_owned(),
                        },
                    );
                }
            }
        }
    }
}

fn snapshot_system(world: &mut World) {
    let game_tick = world.resource::<GameTick>();
    if game_tick.0 % 100 == 0 {
        debug!("Taking snapshot...");

        fn serialize(snapshot: &Snapshot, registry: &AppTypeRegistry) -> String {
            let serializer = SnapshotSerializer { snapshot, registry };

            let mut buf = Vec::new();
            let format = serde_json::ser::PrettyFormatter::with_indent(b"    ");
            let mut ser = serde_json::Serializer::with_formatter(&mut buf, format);

            serializer.serialize(&mut ser).unwrap();

            String::from_utf8(buf).unwrap()
        }

        let snapshot = Snapshot::builder(world)
            .extract_resource::<Items>()
            .extract_resource::<MapEvents>()
            .extract_resource::<GameEvents>()
            /* .extract_entities_matching(|e| {
                e.contains::<Merchant>()
            }) */
            .build();

        let registry = world.resource::<AppTypeRegistry>();

        let _output = serialize(&snapshot, registry);

        //debug!("snapshot: {:?}", output);
    }
}

fn update_game_tick(
    mut commands: Commands,
    mut game_tick: ResMut<GameTick>,
    map: Res<Map>,
    mut attrs: Query<(Entity, &mut Thirst, &mut Hunger, &mut Tired, &mut Heat)>,
    pos: Query<&Position>,
    dehydrated: Query<&Dehydrated>,
    starving: Query<&Starving>,
    exhausted: Query<&Exhausted>,
    state_query: Query<&State>,
) {
    game_tick.0 = game_tick.0 + 1;



    // Update thirst
    for (entity, mut thirst, mut hunger, mut tired, mut heat) in &mut attrs {
        if let Ok(state) = state_query.get(entity) {
            if *state != State::Drinking {
                thirst.update_by_tick_amount(2.0);
            }
        }

        if let Ok(state) = state_query.get(entity) {
            if *state != State::Eating {
                hunger.update_by_tick_amount(2.0);
            }
        }

        if let Ok(state) = state_query.get(entity) {
            if *state != State::Sleeping {
                tired.update_by_tick_amount(2.0);
            }
        }

        // Update heat attribute every hour
        if game_tick.0 % GAME_HOUR == 0 {

            let Ok(pos) = pos.get(entity) else {
                error!("No position found for entity: {:?}", entity);
                continue;
            };

            let tile_temperature= map.tile_temperature(pos.x, pos.y);
            let tile_moisture = map.tile_moisture(pos.x, pos.y);

            debug!("tile_temperature: {:?} tile_moisture: {:?}", tile_temperature, tile_moisture);
            let current_temperature = Map::get_temperature(Season::Winter, 1, tile_temperature, tile_moisture, Weather::ClearSunny);
            info!("Current temperature: {:?}", current_temperature);

            let clothing_mod = 1.0;

            let heat_level_change = (current_temperature - COMFORT_TEMPERATURE) * clothing_mod;
            info!("Heat level change: {:?}", heat_level_change);

            heat.update(heat_level_change);

            info!("Heat level: {:?}", heat.heat);
        }


        if thirst.thirst > 80.0 {
            if let Ok(_dehydrated) = dehydrated.get(entity) {
                // Do nothing
            } else {
                commands.entity(entity).insert(Dehydrated);
            }
        }

        if hunger.hunger > 80.0 {
            if let Ok(_starving) = starving.get(entity) {
                // Do nothing
            } else {
                commands.entity(entity).insert(Starving);
            }
        }

        if tired.tired > 80.0 {
            if let Ok(_exhausted) = exhausted.get(entity) {
                // Do nothing
            } else {
                commands.entity(entity).insert(Exhausted);
            }
        }

        /*debug!(
            "Thirst: {:?} Hunger: {:?} Tired: {:?}",
            thirst.thirst, hunger.hunger, tired.tired
        );*/
        // Is thirsty
        /*if thirst.thirst >= 80.0 {
            morale.morale -= morale.per_tick;
        } else if thirst.thirst >= 90.0 {
            morale.morale -= 2.0 * morale.per_tick;
        } else if thirst.thirst >= 95.0 {
            morale.morale -= 5.0 * morale.per_tick;
        } else {
            morale.morale += morale.per_tick;

            if morale.morale >= 100.0 {
                morale.morale = 100.0;
            }
        }*/

        //debug!("thirst: {:?} morale: {:?}", thirst.thirst, morale.morale);
    }
}



fn dedup<T: Eq + Hash + Copy>(v: &mut Vec<T>) {
    // note the Copy constraint
    let mut uniques = HashSet::new();
    v.retain(|e| uniques.insert(*e));
}

pub fn is_pos_empty(player_id: i32, x: i32, y: i32, query: &Query<MapObjQuery>) -> bool {
    let mut objs = Vec::new();

    for q in query {
        let is_blocking = Obj::is_blocking_state(q.state.to_owned());

        if player_id != q.player_id.0 && x == q.pos.x && y == q.pos.y && is_blocking {
            objs.push(q.entity);
        }
    }

    return objs.len() == 0;
}

/*impl GameEvents {
    pub fn new(&mut self, event_id: i32, run_tick: i32, game_event_type: GameEventType) {
        let game_event = GameEvent {
            event_id: event_id,
            run_tick: run_tick,
            game_event_type: game_event_type,
        };

        //self.insert(map_event_id, map_state_event);
        self.insert(event_id, game_event);
    }
}*/

fn get_random_adjacent_pos(
    player_id: i32,
    center_x: i32,
    center_y: i32,
    all_obj_pos: Vec<(PlayerId, Id, Position)>,
    map: &Map,
) -> Option<Position> {
    let mut selected_pos;

    // Check for a valid stop within 2 tiles
    let mut neighbours = Map::range((center_x, center_y), 2);
    selected_pos = find_valid_pos(neighbours, player_id, &all_obj_pos, map);

    // If none found, check for a valid spot on the 3rd and 4th ring
    if selected_pos.is_none() {
        neighbours = Map::ring((center_x, center_y), 3);
        selected_pos = find_valid_pos(neighbours, player_id, &all_obj_pos, map);

        if selected_pos.is_none() {
            neighbours = Map::ring((center_x, center_y), 4);
            selected_pos = find_valid_pos(neighbours, player_id, &all_obj_pos, map);
        }
    }

    // If no valid tile can be selected return center x,y
    if selected_pos.is_none() {
        selected_pos = Some(Position {
            x: center_x,
            y: center_y,
        });
    }

    return selected_pos;
}

fn find_valid_pos(
    neighbours: Vec<(i32, i32)>,
    player_id: i32,
    all_obj_pos: &Vec<(PlayerId, Id, Position)>,
    map: &Map,
) -> Option<Position> {
    let valid_neighbours: Vec<(i32, i32)> = neighbours
        .into_iter()
        .filter(|(x, y)| is_valid_pos(*x, *y, player_id, all_obj_pos, map))
        .collect();

    if valid_neighbours.len() > 0 {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..valid_neighbours.len());
        let (pos_x, pos_y) = valid_neighbours[index];

        return Some(Position { x: pos_x, y: pos_y });
    } else {
        return None;
    }
}

fn is_valid_pos(
    x: i32,
    y: i32,
    player_id: i32,
    all_obj_pos: &Vec<(PlayerId, Id, Position)>,
    map: &Map,
) -> bool {
    let is_passable = Map::is_passable(x, y, &map);
    let is_valid_pos = Map::is_valid_pos((x, y));
    let is_not_blocked = is_not_blocked(player_id, x, y, &all_obj_pos);

    if is_passable && is_valid_pos && is_not_blocked {
        return true;
    }

    return false;
}

fn is_not_blocked(
    _player_id: i32, 
    x: i32,
    y: i32,
    all_obj_pos: &Vec<(PlayerId, Id, Position)>,
) -> bool {
    // TODO reconsider if player id should be compared
    for (_obj_player_id, _obj_id, obj_pos) in all_obj_pos.iter() {
        if x == obj_pos.x && y == obj_pos.y {
            // found blocking obj
            return false;
        }
    }

    return true;
}

