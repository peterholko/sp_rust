use bevy::ecs::query::WorldQuery;
use bevy::utils::tracing::{debug, trace};
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};

use big_brain::prelude::{FirstToScore, Highest};
use big_brain::thinker::Thinker;
use rand::Rng;

use std::collections::hash_map::Entry;
use std::fmt::Error;
use std::{
    collections::HashMap,
    collections::HashSet,
    hash::Hash,
    sync::{Arc, Mutex},
};

use crossbeam_channel::{unbounded, Receiver as CBReceiver};
use tokio::sync::mpsc::Sender;

use async_compat::Compat;

use crate::ai::{
    AIPlugin, Chase, ChaseAttack, Drink, HighMorale, Morale, ProcessOrder, Thirst, Thirsty,
    VisibleTarget, VisibleTargetScorerBuilder, NO_TARGET,
};
use crate::encounter::Encounter;
use crate::experiment::{ExperimentPlugin, Experiments, Experiment, self};
use crate::item::{self, Item, ItemPlugin, Items};
use crate::map::{Map, MapPlugin, MapTile, self};
use crate::network::{self, network_obj, send_to_client, BroadcastEvents};
use crate::network::{ResponsePacket, StatsData};
use crate::obj::{self, ObjUtil};
use crate::player::{PlayerEvent, PlayerEvents, PlayerPlugin};
use crate::recipe::{Recipe, RecipePlugin, Recipes};
use crate::resource::{Resource, ResourcePlugin, Resources};
use crate::skill::{Skill, SkillPlugin, Skills};
use crate::structure::{Structure, StructurePlugin};
use crate::templates::{ResReq, Templates, TemplatesPlugin};

pub struct GamePlugin;

//pub type Clients = Arc<Mutex<HashMap<i32, Client>>>;
pub type Accounts = Arc<Mutex<HashMap<i32, Account>>>;

#[derive(Resource, Deref, DerefMut, Clone, Debug)]
pub struct Clients(Arc<Mutex<HashMap<i32, Client>>>);

#[derive(Resource, Deref, DerefMut)]
pub struct NetworkReceiver(CBReceiver<PlayerEvent>);

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct MapEvents(pub HashMap<i32, MapEvent>);

#[derive(Resource, Deref, DerefMut)]
pub struct VisibleEvents(Vec<MapEvent>);

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct GameEvents(pub HashMap<i32, GameEvent>);

#[derive(Resource, Deref, DerefMut, Debug, Default)]
pub struct GameTick(pub i32);

// Indexes for IDs
#[derive(Resource, Clone, Debug)]
pub struct Ids {
    pub map_event: i32,
    pub player_event: i32,
    pub obj: i32,
    pub item: i32,
    pub player_hero_map: HashMap<i32, i32>,
    pub obj_entity_map: HashMap<i32, Entity>,
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

#[derive(Clone, Debug)]
pub struct Account {
    pub player_id: i32,
    pub username: String,
    pub password: String,
    pub class: HeroClassList,
}

#[derive(Debug, Component, Clone)]
pub struct Id(pub i32);

#[derive(Debug, Component, Clone, Copy, Eq, PartialEq, Hash)]
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
pub struct State(pub String);

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
pub struct AI;

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
    pub base_hp: i32,
    pub base_def: i32,
    pub damage_range: Option<i32>,
    pub base_damage: Option<i32>,
    pub base_speed: Option<i32>,
    pub base_vision: Option<i32>,
}

#[derive(Debug, Component, Clone)]
pub struct Misc {
    pub image: String,
    pub hsl: Vec<i32>,
    pub groups: Vec<i32>,
}

#[derive(Debug, Component, Clone)]
pub struct VillagerAttrs {
    pub shelter: String,
    pub structure: i32,
}

#[derive(Debug, Component, Clone)]
pub struct StructureAttrs {
    pub start_time: i32,
    pub end_time: i32,
    pub build_time: i32,
    pub builder: i32,
    pub progress: i32,
    pub req: Vec<ResReq>,
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
}

#[derive(Debug, Component)]
pub struct EventInProgress;

#[derive(Bundle, Clone)]
pub struct Obj {
    pub id: Id,
    pub player_id: PlayerId,
    pub position: Position,
    pub name: Name,
    pub template: Template,
    pub class: Class,
    pub subclass: Subclass,
    pub state: State,
    pub viewshed: Viewshed,
    pub misc: Misc,
    pub stats: Stats,
}

#[derive(WorldQuery)]
pub struct MapObjQuery {
    pub entity: Entity,
    // It is required that all reference lifetimes are explicitly annotated, just like in any
    // struct. Each lifetime should be 'static.
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
    pub pos: &'static Position,
    pub class: &'static Class,
    pub subclass: &'static Subclass,
    pub template: &'static Template,
    pub state: &'static mut State,
    pub misc: &'static Misc,
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

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum HeroClassList {
    Warrior,
    Ranger,
    Mage,
    None,
}

// Attributes
pub const CREATIVITY: &str = "Creativity";
pub const DEXTERITY: &str = "Dexterity";
pub const ENDURANCE: &str = "Endurance";
pub const FOCUS: &str = "Focus";
pub const INTELLECT: &str = "Intellect";
pub const SPIRIT: &str = "Spirit";
pub const STRENGTH: &str = "Strength";
pub const TOUGHNESS: &str = "Toughness";

#[derive(Clone, Debug)]
pub struct MapEvent {
    pub event_id: i32,
    pub entity_id: Entity,
    pub obj_id: i32,
    pub player_id: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub run_tick: i32,
    pub map_event_type: VisibleEvent,
}

#[derive(Clone, Debug)]
pub enum VisibleEvent {
    NewObjEvent {
        new_player: bool,
    },
    RemoveObjEvent,
    UpdateObjEvent {
        attr: String,
        value: String
    },
    StateChangeEvent {
        new_state: String
    },
    MoveEvent {
        dst_x: i32,
        dst_y: i32,
    },
    CooldownEvent {
        duration: i32,
    },
    DamageEvent {
        target_id: i32,
        target_pos: Position,
        attack_type: String,
        damage: i32,
        state: String,
    },
    BuildEvent {
        builder_id: i32,
        structure_id: i32
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
}

#[derive(Clone, Debug)]
pub struct GameEvent {
    pub event_id: i32,
    pub run_tick: i32,
    pub game_event_type: GameEventType,
}

#[derive(Clone, Debug)]
pub enum GameEventType {
    SpawnNPC { npc_type: String, pos: Position },
    RemoveEntity { entity: Entity},
    CancelEvents { events: Vec<i32>}
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(MapPlugin)
            .add_plugin(AIPlugin)
            .add_plugin(PlayerPlugin)
            .add_plugin(TemplatesPlugin)
            .add_plugin(ItemPlugin)
            .add_plugin(ResourcePlugin)
            .add_plugin(SkillPlugin)
            .add_plugin(RecipePlugin)
            .add_plugin(ExperimentPlugin)
            .add_plugin(StructurePlugin)
            .init_resource::<GameTick>()
            .add_startup_system(Game::setup)
            .add_system_to_stage(CoreStage::PreUpdate, update_game_tick)
            .add_system(new_obj_event_system)
            .add_system(remove_obj_event_system)
            .add_system(move_event_system)
            .add_system(state_change_event_system)
            .add_system(update_obj_event_system)
            .add_system(build_event_system)
            .add_system(gather_event_system)
            .add_system(operate_refine_event_system)
            .add_system(craft_event_system)
            .add_system(experiment_event_system)
            .add_system(explore_event_system)
            .add_system(damage_event_system)
            .add_system(cooldown_event_system)
            .add_system(use_item_system)
            .add_system(visible_event_system)
            .add_system(game_event_system)
            .add_system(perception_system);
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
    pub fn setup(mut commands: Commands) {
        println!("Bevy Setup System");

        // Initialize game tick
        let game_tick: GameTick = GameTick(0);

        // Initialize indexes
        let ids: Ids = Ids {
            map_event: 0,
            player_event: 0,
            obj: 0,
            item: 0,
            player_hero_map: HashMap::new(),
            obj_entity_map: HashMap::new(),
        };

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

        //Add accounts
        let account = Account {
            player_id: 1,
            username: "peter".to_string(),
            password: "123123".to_string(),
            class: HeroClassList::None,
        };

        let account2 = Account {
            player_id: 2,
            username: "joe".to_string(),
            password: "123123".to_string(),
            class: HeroClassList::None,
        };

        accounts.lock().unwrap().insert(1, account);
        accounts.lock().unwrap().insert(2, account2);

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

        //Insert the clients and client to game channel into the Bevy resources
        commands.insert_resource(clients);
        commands.insert_resource(network_receiver);
        commands.insert_resource(game_tick);
        commands.insert_resource(map_events);
        commands.insert_resource(processed_map_events);
        commands.insert_resource(game_events);
        commands.insert_resource(perception_updates);
        commands.insert_resource(ids);
        commands.insert_resource(explored_map);
    }
}

fn new_obj_event_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    mut perception_updates: ResMut<PerceptionUpdates>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::NewObjEvent { new_player } => {
                    println!("Processing NewObjEvent");

                    if *new_player {
                        perception_updates.insert(map_event.player_id);
                    }

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

fn remove_obj_event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::RemoveObjEvent => {

                    commands.entity(map_event.entity_id).despawn();

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
    mut set: ParamSet<(
        Query<(
            Entity,
            &Id,
            &PlayerId,
            &mut Position,
            &mut State,
            &Viewshed,
            Option<&AI>,
        )>, // p0 mutable for the event processing
        Query<(
            Entity,
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
        )>, // p1 immutable for looking up other entities
    )>,
) {
    let mut events_to_add: Vec<MapEvent> = Vec::new();

    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::MoveEvent { dst_x, dst_y } => {
                    debug!("Processing MoveEvent: {:?}", map_event);

                    // Check if destination is open
                    let mut is_dst_open = true;
                    let mut all_obj_pos: Vec<(PlayerId, Position)> = Vec::new();

                    for (
                        entity,
                        id,
                        player_id,
                        pos,
                        _name,
                        _template,
                        _class,
                        _subclass,
                        _state,
                        _viewshed,
                        _misc,
                    ) in set.p1().iter()
                    {
                        debug!(
                            "enttiy: {:?} id: {:?} player_id: {:?} pos: {:?}",
                            entity, id, player_id, pos
                        );
                        if (map_event.player_id != player_id.0)
                            && (pos.x == *dst_x && pos.y == *dst_y)
                        {
                            is_dst_open = false;
                        }

                        all_obj_pos.push((player_id.clone(), pos.clone()));
                    }

                    if is_dst_open {
                        // Get entity and update state
                        if let Ok((entity, id, player_id, mut pos, mut state, viewshed, ai)) =
                            set.p0().get_mut(map_event.entity_id)
                        {
                            pos.x = *dst_x;
                            pos.y = *dst_y;
                            state.0 = obj::STATE_NONE.to_string();

                            // Remove EventInProgress component
                            commands.entity(entity).remove::<EventInProgress>();

                            // Adding processed map event
                            visible_events.push(map_event.clone());

                            // If player is moving, TODO improve this
                            if player_id.0 < 1000 {
                                let mut rng = rand::thread_rng();

                                let spawn_chance = 0.25;
                                let random_num = rng.gen::<f32>();

                                if random_num < spawn_chance {
                                    let adjacent_pos = get_random_adjacent_pos(
                                        player_id.0,
                                        *dst_x,
                                        *dst_y,
                                        all_obj_pos,
                                        &map,
                                    );

                                    if let Some(adjacent_pos) = adjacent_pos {

                                        let tile_type = Map::tile_type(adjacent_pos.x, adjacent_pos.y, &map);
                                        let npc_list = Encounter::npc_list(tile_type);
                                        let index = rng.gen_range(0..npc_list.len());
                                        let npc_type = npc_list[index].to_string();

                                        debug!("Spawning a NPC of type: {:?}", npc_type);

                                        let event_type = GameEventType::SpawnNPC {
                                            npc_type: npc_type,
                                            pos: adjacent_pos,
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
                                let viewshed_tiles_pos = Map::range((pos.x, pos.y), viewshed.range);

                                // Adding new maps to explored map
                                // Assume player has some explored map tiles
                                let player_explord_map =
                                    explored_map.get_mut(&player_id.0).unwrap();

                                let mut new_explored_tiles = Vec::new();

                                for tile in viewshed_tiles_pos {
                                    if !player_explord_map.contains(&tile) {
                                        new_explored_tiles.push(tile);
                                    }
                                }

                                // Only send new explored tiles
                                if new_explored_tiles.len() > 0 {
                                    let tiles_to_send =
                                        Map::pos_to_tiles(&new_explored_tiles, &map);
                                    let map_packet = ResponsePacket::Map {
                                        data: tiles_to_send,
                                    };
                                    send_to_client(player_id.0, map_packet, &clients);
                                }
                            }
                        }
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

    for event in events_to_add.iter() {
        map_events.insert(event.event_id, event.clone());
    }
}

fn state_change_event_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    mut query: Query<ObjQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::StateChangeEvent { new_state } => {
                    debug!("Processing StateChangeEvent: {:?}", new_state);

                    // Set state back to none
                    let Ok(mut obj) = query.get_mut(map_event.entity_id) else {
                        error!("Query failed to find entity {:?}", map_event.entity_id);
                        continue;
                    };

                    obj.state.0 = new_state.to_string();

                    println!("Adding processed map event");
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

fn update_obj_event_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    mut query: Query<ObjQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::UpdateObjEvent { attr, value} => {
                    debug!("Processing UpdateObjEvent: {:?} {:?}", attr, value);

                    // Set state back to none
                    let Ok(mut obj) = query.get_mut(map_event.entity_id) else {
                        error!("Query failed to find entity {:?}", map_event.entity_id);
                        continue;
                    };

                    match attr.as_str() {
                        obj::TEMPLATE => {
                            obj.template.0 = value.to_string();
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
    mut ids: ResMut<Ids>,
    mut query: Query<ObjWithStatsQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::BuildEvent { builder_id, structure_id } => {
                    debug!("Processing BuildEvent: builder_id {:?}, structure_id: {:?} ", builder_id, structure_id);
                    events_to_remove.push(*map_event_id);

                    let Some(builder_entity) = ids.get_entity(*builder_id) else {
                        error!("Cannot find builder from {:?}", *builder_id);
                        continue;
                    };

                    let Ok(mut builder) = query.get_mut(builder_entity) else {
                        error!("Query failed to find entity {:?}", builder_entity);
                        continue;
                    };

                    builder.state.0 = obj::STATE_NONE.to_string();

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find structure from {:?}", *structure_id);
                        continue;
                    };                    

                    let Ok(mut structure) = query.get_mut(structure_entity) else {
                        error!("Query failed to find entity {:?}", structure_entity);
                        continue;
                    };            

                    structure.state.0 = obj::STATE_NONE.to_string();
                    structure.stats.hp = structure.stats.base_hp;

                    let state_change_event = VisibleEvent::StateChangeEvent { new_state: obj::STATE_NONE.to_string() };

                    let structure_state_event = MapEvent {
                        event_id: ids.new_map_event_id(),
                        entity_id: structure_entity,
                        obj_id: structure.id.0,
                        player_id: structure.player_id.0,
                        pos_x: structure.pos.x,
                        pos_y: structure.pos.y,
                        run_tick: game_tick.0 + 1,
                        map_event_type: state_change_event,
                    };

                    visible_events.push(structure_state_event);
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
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut resources: ResMut<Resources>,
    mut items: ResMut<Items>,
    skills: ResMut<Skills>,
    templates: Res<Templates>,
    mut map_events: ResMut<MapEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::GatherEvent { res_type } => {
                    println!("Processing GatherEvent");

                    Resource::gather_by_type(
                        map_event.obj_id,
                        map_event.obj_id,
                        Position {
                            x: map_event.pos_x,
                            y: map_event.pos_y,
                        },
                        res_type.to_string(),
                        &skills,
                        &mut items,
                        &templates.item_templates,
                        &resources,
                        &templates.res_templates,
                        &mut ids,
                    );

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
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::RefineEvent { structure_id } => {
                    info!("Processing RefineEvent");
                    events_to_remove.push(*map_event_id);

                    // Set state back to none
                    let Ok(mut villager) = query.get_mut(map_event.entity_id) else {
                        error!("Query failed to find entity {:?}", map_event.entity_id);
                        continue;
                    };

                    // Reset villager state to None
                    villager.state.0 = "none".to_string();

                    // Remove Event In Progress
                    commands
                        .entity(map_event.entity_id)
                        .remove::<EventInProgress>();   

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find entity from structure_id: {:?}", structure_id);
                        continue;
                    };

                    // Set state back to none
                    let Ok(structure) = query.get(structure_entity) else {
                        error!("Query failed to find entity {:?}", map_event.entity_id);
                        continue;
                    };   

                    let Some(structure_template) = Structure::get_template(structure.template.0.clone(), &templates.obj_templates) else {
                        error!("Query failed to find structure template {:?}", structure.template.0);
                        continue;
                    };

                    let Some(structure_refine_list) = structure_template.refine else {
                        error!("Missing refine list on structure template {:?}", structure.template.0);
                        continue; 
                    };                          

                    for item_class in structure_refine_list.iter() {
                        let item_to_refine = Item::get_by_class(*structure_id, item_class.clone(), &items);

                        let Some(item_to_refine) = item_to_refine else {                    
                            continue;
                        };

                        let item_template = Item::get_template(item_to_refine.name, &templates.item_templates);

                        let Some(produces_list) = item_template.produces.clone() else {
                            error!("Missing item produces attribute for item template {:?}", item_template);
                            continue;
                        };

                        let capacity = ObjUtil::get_capacity(&structure_template.template, &templates.obj_templates);

                        for produce_item in produces_list.iter() {
                            let current_total_weight = Item::get_total_weight(*structure_id, &items, &templates.item_templates);
                            let item_weight = Item::get_weight(produce_item.to_string(), 1, &items, &templates.item_templates);

                            if current_total_weight + item_weight > capacity {
                                info!("Refining structure is full {:?}", structure);
                                continue;
                            }

                            let mut items_to_update: Vec<network::Item> = Vec::new();
                            let mut items_to_remove = Vec::new();
                            
                            // Consume item to refine
                            let refined_item = Item::remove_quantity(item_to_refine.id, 1, &mut items);                        

                            // Add item with zero quantity to remove list
                            if let Some(refined_item) = refined_item {
                                let refined_item_packet = Item::to_packet(refined_item);
                                items_to_update.push(refined_item_packet);
                            } else {
                                // Item was removed, add to remove list
                                items_to_remove.push(item_to_refine.id);                                
                            }

                            // Create new item
                            let new_item_id = ids.new_item_id();

                            let (new_item, _merged) = Item::create(
                                new_item_id,
                                *structure_id,
                                produce_item.to_string(),
                                1,
                                &templates.item_templates,
                                &mut items,
                            );

                            // Convert items to be updated to packets
                            let new_item_packet = Item::to_packet(new_item);

                            items_to_update.push(new_item_packet);

                            let item_update_packet: ResponsePacket = ResponsePacket::InfoItemsUpdate {
                                id: *structure_id,
                                items_updated: items_to_update,
                                items_removed: items_to_remove
                            };
            
                            send_to_client(map_event.player_id, item_update_packet, &clients);
                        }                            
                    }
                }
                VisibleEvent::OperateEvent { structure_id } => {
                    info!("Processing OperateEvent");
                    events_to_remove.push(*map_event_id);

                    // Remove Event In Progress
                    commands
                    .entity(map_event.entity_id)
                    .remove::<EventInProgress>();

                    // Set state back to none
                    let Ok(mut villager) = query.get_mut(map_event.entity_id) else {
                        error!("Query failed to find entity {:?}", map_event.entity_id);
                        continue;
                    };

                    // Reset villager state to None
                    villager.state.0 = "none".to_string();

                    let Some(structure_entity) = ids.get_entity(*structure_id) else {
                        error!("Cannot find entity from structure_id: {:?}", structure_id);
                        continue;
                    };

                    // Set state back to none
                    let Ok(mut structure) = query.get(structure_entity) else {
                        error!("Query failed to find entity {:?}", map_event.entity_id);
                        continue;
                    };

                    let res_type = Structure::resource_type(structure.template.0.clone());

                    Resource::gather_by_type(
                        map_event.obj_id,
                        *structure_id,
                        Position {
                            x: map_event.pos_x,
                            y: map_event.pos_y,
                        },
                        res_type.to_string(),
                        &skills,
                        &mut items,
                        &templates.item_templates,
                        &resources,
                        &templates.res_templates,
                        &mut ids,
                    );
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
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut resources: ResMut<Resources>,
    mut items: ResMut<Items>,
    skills: ResMut<Skills>,
    templates: Res<Templates>,
    recipes: Res<Recipes>,
    //mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut state_query: Query<&mut State>,
    mut map_events: ResMut<MapEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::CraftEvent {
                    structure_id,
                    recipe_name,
                } => {
                    info!("Processing CraftEvent");
                    events_to_remove.push(*map_event_id);

                    // Set state back to none
                    let Ok(mut villager_state) = state_query.get_mut(map_event.entity_id) else {
                        error!("Query failed to find entity {:?}", map_event.entity_id);
                        continue;
                    };

                    let recipe = Recipe::get_by_name(recipe_name.clone(), &recipes);

                    if let Some(mut recipe) = recipe {
                        if Structure::has_req(*structure_id, &mut recipe.req, &mut items) {
                            Structure::consume_reqs(*structure_id, recipe.req, &mut items);

                            // Reset villager state to None
                            villager_state.0 = "none".to_string();

                            // Remove Event In Progress
                            commands
                                .entity(map_event.entity_id)
                                .remove::<EventInProgress>();

                            let mut item_attrs = HashMap::new();
                            item_attrs.insert(item::DAMAGE, 11.0);

                            // Create new item
                            Item::craft(
                                ids.new_item_id(),
                                *structure_id,
                                recipe_name.to_string(),
                                1,
                                item_attrs,
                                &templates.recipe_templates,
                                &mut items,
                                None,
                                None,
                            );
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
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut resources: ResMut<Resources>,
    mut items: ResMut<Items>,
    skills: ResMut<Skills>,
    templates: Res<Templates>,
    recipes: Res<Recipes>,
    mut experiments: ResMut<Experiments>,
    //mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut state_query: Query<&mut State>,
    mut map_events: ResMut<MapEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::ExperimentEvent {
                    structure_id,
                } => {
                    info!("Processing ExperimentEvent");
                    events_to_remove.push(*map_event_id);

                    let mut experiment: Option<&mut Experiment> = None;

                    for e in experiments.iter_mut() {
                        if *structure_id == e.structure {
                            experiment = Some(e);
                        }                        
                    }


                    if let Some(experiment) = experiment {
                        if experiment.recipe == experiment::EXP_RECIPE_NONE {


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

fn explore_event_system(
    game_tick: Res<GameTick>,
    mut resources: ResMut<Resources>,
    templates: Res<Templates>,
    mut map_events: ResMut<MapEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::ExploreEvent => {
                    debug!("Processing ExploreEvent");
                    events_to_remove.push(*map_event_id);

                    Resource::explore(
                        map_event.obj_id,
                        Position {
                            x: map_event.pos_x,
                            y: map_event.pos_y,
                        },
                        &mut resources,
                        &templates.res_templates,
                    );
                }
                _ => {}
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn damage_event_system(
    game_tick: Res<GameTick>,
    mut visible_events: ResMut<VisibleEvents>,
    mut map_events: ResMut<MapEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::DamageEvent {
                    target_id,
                    target_pos,
                    attack_type,
                    damage,
                    state,
                } => {
                    debug!("Processing DamageEvent");
                    events_to_remove.push(*map_event_id);

                    // Adding processed map event
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

fn cooldown_event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut state_query: Query<&mut State>,
    mut map_events: ResMut<MapEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::CooldownEvent { duration } => {
                    debug!("Processing CooldownEvent {:?}", duration);
                    events_to_remove.push(*map_event_id);
                    // Set state back to none
                    let Ok(mut obj_state) = state_query.get_mut(map_event.entity_id) else {
                        error!("Query failed to find entity {:?}", map_event.entity_id);
                        continue;
                    };

                    commands
                        .entity(map_event.entity_id)
                        .remove::<EventInProgress>();
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
    mut ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    mut visible_events: ResMut<VisibleEvents>,
    mut map_events: ResMut<MapEvents>,
    mut query: Query<ObjWithStatsQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
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

                    let Some(mut item) = Item::find_by_id(*item_id, &items) else {
                        debug!("Failed to find item: {:?}", item_id);
                        continue;
                    };

                    let Ok(mut item_owner) = query.get_mut(entity) else {
                        error!("Query failed to find entity {:?}", entity);
                        continue;
                    };

                    match (item.class.as_str(), item.subclass.as_str()) {
                        (item::POTION, item::HEALTH) => {
                            let healing_value = *item.attrs.get(item::HEALING).unwrap() as i32;

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

fn visible_event_system(
    clients: Res<Clients>,
    mut visible_events: ResMut<VisibleEvents>,
    // query: Query<(&Id, &PlayerId, &Position, &State, &Viewshed)>,
    mut set: ParamSet<(
        Query<(
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
        )>, // p0 for event entity source
        Query<(&Id, &PlayerId, &Position, &State, &Viewshed)>, // p1 for event observer
    )>,
) {
    // TODO explore using traits in the HashSet to reduce code
    let mut all_change_events: HashMap<i32, HashSet<network::ChangeEvents>> = HashMap::new();

    let mut all_broadcast_events: HashMap<i32, HashSet<BroadcastEvents>> = HashMap::new();

    for map_event in visible_events.iter() {
        debug!("Checking if map_event is visible: {:?}", map_event);

        // Get event object components.  eo => event_object

        match set.p0().get(map_event.entity_id) {
            Ok((
            eo_id,
            eo_player_id,
            eo_pos,
            eo_name,
            eo_template,
            eo_class,
            eo_subclass,
            eo_state,
            eo_viewshed,
            eo_misc)) => {
                let new_obj = network::network_obj(
                    eo_id.0,
                    eo_player_id.0,
                    eo_pos.x,
                    eo_pos.y,
                    eo_name.0.to_owned(),
                    eo_template.0.to_owned(),
                    eo_class.0.to_owned(),
                    eo_subclass.0.to_owned(),
                    eo_state.0.to_owned(),
                    eo_viewshed.range,
                    eo_misc.image.to_owned(),
                    eo_misc.hsl.to_owned(),
                    eo_misc.groups.to_owned(),
                );

                for (id, player_id, pos, state, viewshed) in set.p1().iter() {
                    match &map_event.map_event_type {
                        VisibleEvent::NewObjEvent { new_player } => {
                            let distance =
                                Map::distance((map_event.pos_x, map_event.pos_y), (pos.x, pos.y));

                            if viewshed.range >= distance {
                                debug!("Send obj create to client");

                                let change_event = network::ChangeEvents::ObjCreate {
                                    event: "obj_create".to_string(),
                                    obj: new_obj.to_owned(),
                                };

                                all_change_events
                                    .entry(player_id.0)
                                    .or_default()
                                    .insert(change_event);
                            }
                        }
                        VisibleEvent::MoveEvent { dst_x, dst_y } => {
                            let src_distance =
                                Map::distance((map_event.pos_x, map_event.pos_y), (pos.x, pos.y));

                            if viewshed.range >= src_distance {
                                let change_event = network::ChangeEvents::ObjMove {
                                    event: "obj_move".to_string(),
                                    obj: new_obj.to_owned(),
                                    src_x: *dst_x,
                                    src_y: *dst_y,
                                };

                                all_change_events
                                    .entry(player_id.0)
                                    .or_default()
                                    .insert(change_event);
                            }

                            let dst_distance = Map::distance((*dst_x, *dst_y), (pos.x, pos.y));

                            if viewshed.range >= dst_distance {
                                let change_event = network::ChangeEvents::ObjMove {
                                    event: "obj_move".to_string(),
                                    obj: new_obj.to_owned(),
                                    src_x: *dst_x,
                                    src_y: *dst_y,
                                };

                                all_change_events
                                    .entry(player_id.0)
                                    .or_default()
                                    .insert(change_event);
                            }
                        }
                        VisibleEvent::DamageEvent {
                            target_id,
                            target_pos,
                            attack_type,
                            damage,
                            state,
                        } => {
                            debug!("Processing DamageEvent: {:?}", &map_event.map_event_type);
                            let attacker_distance =
                                Map::distance((map_event.pos_x, map_event.pos_y), (pos.x, pos.y));

                            if viewshed.range >= attacker_distance {
                                let damage_event = BroadcastEvents::Damage {
                                    sourceid: map_event.obj_id,
                                    targetid: *target_id,
                                    attacktype: attack_type.to_string(),
                                    dmg: *damage,
                                    state: state.to_string(),
                                    combo: None,
                                    countered: None,
                                };

                                all_broadcast_events
                                    .entry(player_id.0)
                                    .or_default()
                                    .insert(damage_event);
                            }

                            let target_distance =
                                Map::distance((target_pos.x, target_pos.y), (pos.x, pos.y));

                            if viewshed.range >= target_distance {
                                let damage_event = BroadcastEvents::Damage {
                                    sourceid: map_event.obj_id,
                                    targetid: *target_id,
                                    attacktype: attack_type.to_string(),
                                    dmg: *damage,
                                    state: state.to_string(),
                                    combo: None,
                                    countered: None,
                                };

                                all_broadcast_events
                                    .entry(player_id.0)
                                    .or_default()
                                    .insert(damage_event);
                            }
                        }
                        VisibleEvent::StateChangeEvent { new_state } => {
                            let distance =
                                Map::distance((map_event.pos_x, map_event.pos_y), (pos.x, pos.y));

                            if viewshed.range >= distance {
                                debug!("Send obj update to client");

                                let change_event = network::ChangeEvents::ObjUpdate {
                                    event: "obj_update".to_string(),
                                    obj_id: map_event.obj_id,
                                    attr: "state".to_string(),
                                    value: new_state.clone(),
                                };

                                all_change_events
                                    .entry(player_id.0)
                                    .or_default()
                                    .insert(change_event);
                            }
                        }
                        VisibleEvent::UpdateObjEvent { attr, value} => {
                            let distance =
                                Map::distance((map_event.pos_x, map_event.pos_y), (pos.x, pos.y));

                            if viewshed.range >= distance {
                                debug!("Send obj update to client");

                                let change_event = network::ChangeEvents::ObjUpdate {
                                    event: "obj_update".to_string(),
                                    obj_id: map_event.obj_id,
                                    attr: attr.to_string(),
                                    value: value.clone(),
                                };

                                all_change_events
                                    .entry(player_id.0)
                                    .or_default()
                                    .insert(change_event);
                            }                        
                        }
                        _ => {}
                    }
                }
            }
            Err(error) => {
                debug!("VisibleEventSystem error: {:?}", error);
                for (id, player_id, pos, state, viewshed) in set.p1().iter() {
                    match &map_event.map_event_type {
                        VisibleEvent::RemoveObjEvent => {
                            let distance =
                                Map::distance((map_event.pos_x, map_event.pos_y), (pos.x, pos.y));

                            if viewshed.range >= distance {
                                debug!("Send obj delete to client");

                                let change_event = network::ChangeEvents::ObjDelete { 
                                    event: "obj_delete".to_string(),
                                    obj_id: map_event.obj_id,
                                };

                                all_change_events
                                    .entry(player_id.0)
                                    .or_default()
                                    .insert(change_event);
                            }
                        }
                        _ => {}
                    }   
                }          
            }
        }
    }

    for (player_id, change_events) in all_change_events.iter_mut() {
        let changes_packet = ResponsePacket::Changes {
            events: change_events.clone().into_iter().collect(),
        };

        for (_client_id, client) in clients.lock().unwrap().iter() {
            println!("Player: {:?} == client: {:?}", player_id, client);
            if client.player_id == *player_id {
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
            println!("Player: {:?} == client: {:?}", player_id, client);
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

fn perception_system(
    map: Res<Map>,
    mut explored_map: ResMut<ExploredMap>,
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
    // Could use HashSet here due to the trait `FromIterator<&std::collections::HashSet<(i32, i32)>>` is not implemented for `Vec<(i32, i32)>`
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

                if viewshed1.range >= distance {
                    println!("Adding visible obj to percetion");

                    let visible_obj = network_obj(
                        id2.0,
                        player2.0,
                        pos2.x,
                        pos2.y,
                        name2.0.to_owned(),
                        template2.0.to_owned(),
                        class2.0.to_owned(),
                        subclass2.0.to_owned(),
                        state2.0.to_owned(),
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

                if viewshed2.range >= distance {
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
                        state1.0.to_owned(),
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

            let perception_data = network::PerceptionData {
                map: tiles,
                objs: perception.clone().into_iter().collect(),
            };

            let perception_packet = ResponsePacket::Perception {
                data: perception_data,
            };

            for (_client_id, client) in clients.lock().unwrap().iter() {
                println!("Player: {:?} == client: {:?}", player_id, client);
                if client.player_id == *player_id {
                    client
                        .sender
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
    mut structure_query: Query<StructureQuery>,
) {
    let mut events_to_remove = Vec::new();

    for (event_id, game_event_type) in game_events.iter_mut() {
        if game_event_type.run_tick < game_tick.0 {
            // Execute event
            match &game_event_type.game_event_type {
                GameEventType::SpawnNPC { npc_type, pos } => {
                    debug!("Processing SpawnNPC");
                    events_to_remove.push(*event_id);

                    let spawn_result = spawn_npc(
                        1000,
                        *pos,
                        npc_type.to_string(),
                        &mut commands,
                        &mut ids,
                        &mut items,
                        &templates,
                    );

                    match spawn_result {
                        Ok((entity, npc_id, player_id, pos)) => {
                            let event = create_map_event(
                                entity,
                                npc_id.clone(),
                                player_id,
                                pos,
                                &game_tick,
                                &mut ids,
                            );
                            map_events.insert(event.event_id, event);
                        }
                        Err(err_msg) => error!(err_msg),
                    }
                }
                GameEventType::CancelEvents { events } => {
                    events_to_remove.push(*event_id);

                    for (map_event_id, map_event) in map_events.iter_mut() {
                        match map_event.map_event_type {
                            VisibleEvent::BuildEvent { builder_id, structure_id } => {
                                //TODO: should be able to change state without the need for entity, playerid and position

                                let Some(structure_entity) = ids.get_entity(structure_id) else {
                                    error!("Cannot find entity from structure_id: {:?}", structure_id);
                                    continue;
                                };
            
                                // Set state back to none
                                let Ok(mut structure) = structure_query.get_mut(structure_entity) else {
                                    error!("Query failed to find entity {:?}", structure_entity);
                                    continue;
                                };

                                structure.state.0 = obj::STATE_STALLED.to_string();
                                let ratio = (game_tick.0 - structure.attrs.start_time) as f32 / structure.attrs.build_time as f32;

                                debug!("Ratio: {:?}", ratio);

                                structure.attrs.progress = (ratio * 100.0).round() as i32;

                                debug!("Progress: {:?}", structure.attrs.progress);

                                let new_obj_event = VisibleEvent::StateChangeEvent { new_state: obj::STATE_STALLED.to_string() };
                                let event_id = ids.new_map_event_id();
                            
                                let event = MapEvent {
                                    event_id: event_id,
                                    entity_id: structure_entity,
                                    obj_id: structure.id.0,
                                    player_id: structure.player_id.0,
                                    pos_x: structure.pos.x,
                                    pos_y: structure.pos.y,
                                    run_tick: game_tick.0 + 1, // Add one game tick
                                    map_event_type: new_obj_event,
                                };

                                visible_events.push(event);
                            }
                            _ => {}
                        }
                    }

                    for event_id in events.iter() {
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

fn update_game_tick(mut game_tick: ResMut<GameTick>, mut attrs: Query<(&mut Thirst, &mut Morale)>) {
    game_tick.0 = game_tick.0 + 1;

    // Update thirst
    /*for (mut thirst, mut morale) in &mut attrs {
        thirst.thirst += thirst.per_tick;

        // Is thirsty
        if thirst.thirst >= 80.0 {
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
        }

        if thirst.thirst >= 100.0 {
            thirst.thirst = 100.0;
        }

        // println!("thirst: {:?} morale: {:?}", thirst.thirst, morale.morale);
    } */
}

fn dedup<T: Eq + Hash + Copy>(v: &mut Vec<T>) {
    // note the Copy constraint
    let mut uniques = HashSet::new();
    v.retain(|e| uniques.insert(*e));
}

pub fn is_pos_empty(player_id: i32, x: i32, y: i32, query: &Query<MapObjQuery>) -> bool {
    let mut objs = Vec::new();

    for q in query {
        let is_blocking = is_blocking_state(&q.state.0);

        if player_id != q.player_id.0 && x == q.pos.x && y == q.pos.y && is_blocking {
            objs.push(q.entity);
        }
    }

    return objs.len() == 0;
}

pub fn is_blocking_state(state_str: &str) -> bool {
    let result = match state_str {
        obj::STATE_DEAD => false,
        obj::STATE_FOUNDED => false,
        obj::STATE_PROGRESSING => false,
        _ => true,
    };

    result
}

//TODO remove this function, state == obj::STATE_NONE is much simpler 
pub fn is_none_state(state_str: &str) -> bool {
    let is_none_state = state_str == obj::STATE_NONE;

    return is_none_state;
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

    pub fn new_player_hero_mapping(&mut self, player_id: i32, hero_id: i32) {
        self.player_hero_map.insert(player_id, hero_id);
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

    pub fn new_entity_obj_mapping(&mut self, obj_id: i32, entity: Entity) {
        self.obj_entity_map.insert(obj_id, entity);
    }

    pub fn new_item_id(&mut self) -> i32 {
        self.item = self.item + 1;
        self.item
    }
}

impl MapEvents {
    pub fn new(
        &mut self,
        map_event_id: i32,
        entity_id: Entity,
        obj_id: &Id,
        player_id: &PlayerId,
        pos: &Position,
        game_tick: i32,
        map_event_type: VisibleEvent,
    ) {
        let map_state_event = MapEvent {
            event_id: map_event_id,
            entity_id: entity_id,
            obj_id: obj_id.0,
            player_id: player_id.0,
            pos_x: pos.x,
            pos_y: pos.y,
            run_tick: game_tick,
            map_event_type: map_event_type,
        };

        //self.insert(map_event_id, map_state_event);
        self.insert(map_event_id, map_state_event);
    }
}

impl GameEvents {
    pub fn new(&mut self, event_id: i32, run_tick: i32, game_event_type: GameEventType) {
        let game_event = GameEvent {
            event_id: event_id,
            run_tick: run_tick,
            game_event_type: game_event_type,
        };

        //self.insert(map_event_id, map_state_event);
        self.insert(event_id, game_event);
    }
}

fn spawn_npc(
    player_id: i32,
    pos: Position,
    template: String,
    commands: &mut Commands,
    ids: &mut ResMut<Ids>,
    mut items: &mut ResMut<Items>,
    templates: &Res<Templates>,
) -> Result<(Entity, Id, PlayerId, Position), &'static str> {
    let mut npc_template = None;

    // Look up npc template
    for obj_template in templates.obj_templates.iter() {
        if template == obj_template.template {
            npc_template = Some(obj_template);
        }
    }

    if let Some(npc_template) = npc_template {
        let npc_id = ids.new_obj_id();
        
        let image: String = npc_template.template.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();

        let npc = Obj {
            id: Id(npc_id),
            player_id: PlayerId(player_id),
            position: pos,
            name: Name(npc_template.name.clone()),
            template: Template(npc_template.template.clone()),
            class: Class(npc_template.class.clone()),
            subclass: Subclass(npc_template.subclass.clone()),
            state: State("none".into()),
            viewshed: Viewshed { range: 2 },
            misc: Misc {
                image: image,
                hsl: Vec::new().into(),
                groups: Vec::new().into(),
            },
            stats: Stats {
                hp: npc_template.base_hp.unwrap(),
                base_hp: npc_template.base_hp.unwrap(),
                base_def: npc_template.base_def.unwrap(),
                base_damage: npc_template.base_dmg,
                damage_range: npc_template.dmg_range,
                base_speed: npc_template.base_speed,
                base_vision: npc_template.base_vision,
            },
        };

        let entity = commands
            .spawn((
                npc,
                SubclassNPC,
                Chase,
                VisibleTarget::new(NO_TARGET),
                Thinker::build()
                    .label("NPC Chase")
                    .picker(Highest)
                    .when(VisibleTargetScorerBuilder, ChaseAttack),
            ))
            .id();

        Encounter::generate_loot(npc_id, ids, items, templates);

        debug!("New {:?} entity: {:?}", template, entity);
        ids.new_entity_obj_mapping(npc_id, entity);

        return Ok((entity, Id(npc_id), PlayerId(player_id), pos));
    }

    return Err("Invalid obj template");
}

fn create_map_event(
    entity: Entity,
    npc_id: Id,
    player_id: PlayerId,
    pos: Position,
    game_tick: &Res<GameTick>,
    ids: &mut ResMut<Ids>,
    // mut map_events: ResMut<MapEvents>,
) -> MapEvent {
    // Insert state change event
    let new_obj_event = VisibleEvent::NewObjEvent { new_player: false };
    let map_event_id = ids.new_map_event_id();

    let map_event = MapEvent {
        event_id: map_event_id,
        entity_id: entity,
        obj_id: npc_id.0,
        player_id: player_id.0,
        pos_x: pos.x,
        pos_y: pos.y,
        run_tick: game_tick.0 + 4, // Add one game tick
        map_event_type: new_obj_event,
    };

    return map_event;

    // map_events.insert(map_event_id, map_state_event);
}

fn get_random_adjacent_pos(
    player_id: i32,
    center_x: i32,
    center_y: i32,
    all_obj_pos: Vec<(PlayerId, Position)>,
    map: &Map,
) -> Option<Position> {
    let mut selected_pos = None;

    let neighbours = Map::range((center_x, center_y), 1);

    for (x, y) in neighbours {
        let is_passable = Map::is_passable(x, y, &map);
        let is_valid_pos = Map::is_valid_pos((x, y));
        let is_not_blocked = is_not_blocked(player_id, x, y, &all_obj_pos);

        if is_passable && is_valid_pos && is_not_blocked {
            selected_pos = Some(Position { x: x, y: y });
        }
    }

    return selected_pos;
}

fn is_not_blocked(player_id: i32, x: i32, y: i32, all_obj_pos: &Vec<(PlayerId, Position)>) -> bool {
    for (obj_player_id, obj_pos) in all_obj_pos.iter() {
        if player_id != obj_player_id.0 && x == obj_pos.x && y == obj_pos.y {
            // found blocking obj
            return false;
        }
    }

    return true;
}
