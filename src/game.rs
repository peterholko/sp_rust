use bevy::ecs::query::WorldQuery;
use bevy::utils::tracing::{debug, trace};
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};

use std::{
    collections::HashMap,
    collections::HashSet,
    hash::Hash,
    sync::{Arc, Mutex},
};

use crossbeam_channel::{unbounded, Receiver as CBReceiver};
use tokio::sync::mpsc::Sender;

use async_compat::Compat;

use crate::ai::{AIPlugin, Drink, HighMorale, Morale, ProcessOrder, Thirst, Thirsty};
use crate::item::{Item, ItemPlugin, Items};
use crate::map::{Map, MapPlugin, MapTile};
use crate::network::{self, BroadcastEvents, send_to_client, network_obj};
use crate::network::ResponsePacket;
use crate::player::{PlayerEvent, PlayerEvents, PlayerPlugin};
use crate::resource::{Resource, ResourcePlugin, Resources};
use crate::skill::{Skill, SkillPlugin, Skills};
use crate::templates::{Templates, TemplatesPlugin};

pub struct GamePlugin;

//pub type Clients = Arc<Mutex<HashMap<i32, Client>>>;
pub type Accounts = Arc<Mutex<HashMap<i32, Account>>>;

#[derive(Resource, Deref, DerefMut, Clone, Debug)]
pub struct Clients(Arc<Mutex<HashMap<i32, Client>>>);

#[derive(Resource, Deref, DerefMut)]
pub struct NetworkReceiver(CBReceiver<PlayerEvent>);

#[derive(Resource, Deref, DerefMut)]
pub struct MapEvents(pub HashMap<i32, MapEvent>);

#[derive(Resource, Deref, DerefMut)]
pub struct VisibleEvents(Vec<MapEvent>);

#[derive(Resource, Deref, DerefMut, Debug, Default)]
pub struct GameTick(pub i32);

// Indexes for IDs
#[derive(Resource, Clone, Debug)]
pub struct Ids {
    pub map_event: i32,
    pub obj: i32,
    pub item: i32,
}

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

#[derive(Debug, Component,)]
pub struct Hp(pub i32);

#[derive(Debug, Component, Clone)]
pub struct PlayerId(pub i32);

#[derive(Debug, Component)]
pub struct Name(pub String);

#[derive(Debug, Component)]
pub struct Template(pub String);

#[derive(Debug, Component)]
pub struct Class(pub String);

#[derive(Debug, Component)]
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
pub struct ClassStructure; //Class Structure

#[derive(Debug, Component)]
pub struct AI;

#[derive(Debug, Component)]
pub struct Misc {
    pub image: String,
    pub hsl: Vec<i32>,
    pub groups: Vec<i32>,
}

#[derive(Debug, Component)]
pub struct StructureAttrs {
    pub start_time: i32,
    pub end_time: i32,
    pub build_time: i32,
    pub builder: i32,
    pub progress: i32,
}

#[derive(Debug, Component)]
pub struct OrderFollow {
    pub target: Entity,
}

#[derive(Debug, Component)]
pub struct EventInProgress;

#[derive(Bundle)]
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
    pub hp: Hp,
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

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum HeroClassList {
    Warrior,
    Ranger,
    Mage,
    None,
}

// States
pub const NONE: &str = "none";
pub const MOVING: &str = "moving";
pub const DEAD: &str = "dead";
pub const FOUNDED: &str = "founded";
pub const PROGRESSING: &str = "progressing";
pub const BUILDING: &str = "building";



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
    NewObjEvent { new_player: bool },
    StateChangeEvent { new_state: String },
    MoveEvent { dst_x: i32, dst_y: i32 },
    DamageEvent { target_id: i32, target_pos: Position, attack_type: String, damage: i32, state: String},
    GatherEvent { res_type: String },
    ExploreEvent,

}

#[derive(Clone, Debug)]
struct ExploredMap {
    player_id: i32,
    tiles: HashSet<i32>,
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
            .init_resource::<GameTick>()
            .add_startup_system(Game::setup)
            .add_system_to_stage(CoreStage::PreUpdate, update_game_tick)
            .add_system(new_obj_event_system)
            .add_system(move_event_system)
            .add_system(state_change_event_system)
            .add_system(gather_event_system)
            .add_system(explore_event_system)
            .add_system(visible_event_system)
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
            obj: 0,
            item: 0,
        };

        // Initialize map events vector
        let map_events: MapEvents = MapEvents(HashMap::new());
        let processed_map_events: VisibleEvents = VisibleEvents(Vec::new());

        let perception_updates: PerceptionUpdates = PerceptionUpdates(HashSet::new());

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
        commands.insert_resource(perception_updates);
        commands.insert_resource(ids);
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
        println!("Processing {:?}", map_event);

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

fn move_event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    clients: Res<Clients>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
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
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        println!("Processing {:?}", map_event);

        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::MoveEvent { dst_x, dst_y } => {
                    println!("Processing MoveEvent");

                    // Check if destination is open
                    let mut is_dst_open = true;

                    for (
                        entity,
                        _id,
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
                        if (map_event.player_id != player_id.0)
                            && (pos.x == *dst_x && pos.y == *dst_y)
                        {
                            is_dst_open = false;
                        }
                    }

                    if is_dst_open {
                        // Get entity and update state
                        if let Ok((entity, id, player_id, mut pos, mut state, viewshed, ai)) =
                            set.p0().get_mut(map_event.entity_id)
                        {
                            pos.x = *dst_x;
                            pos.y = *dst_y;
                            state.0 = NONE.to_string();

                            // Remove EventInProgress component
                            commands.entity(entity).remove::<EventInProgress>();

                            println!("Adding processed map event");
                            // Adding processed map event
                            visible_events.push(map_event.clone());

                            // Adding new map tiles
                            let new_tiles_pos = Map::range((pos.x, pos.y), viewshed.range);

                            let tiles = Map::pos_to_tiles(&new_tiles_pos.clone(), &map);

                            // TODO reconsider sending map packet here
                            let map_packet = ResponsePacket::Map { data: tiles };

                            send_to_client(player_id.0, map_packet, &clients);
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
}

fn state_change_event_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
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
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        println!("Processing {:?}", map_event);

        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::StateChangeEvent { new_state } => {
                    println!("Processing StateChangeEvent: {:?}", new_state);

                    // Get entity and update state
                    if let Ok((_entity, id, playerId, mut pos, mut state, _viewshed, ai)) =
                        set.p0().get_mut(map_event.entity_id)
                    {
                        state.0 = new_state.to_string();

                        println!("Adding processed map event");
                        visible_events.push(map_event.clone());
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
        println!("Processing {:?}", map_event);

        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::GatherEvent { res_type } => {
                    println!("Processing GatherEvent");

                    Resource::gather_by_type(
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

fn explore_event_system(
    game_tick: Res<GameTick>,
    mut resources: ResMut<Resources>,
    templates: Res<Templates>,
    mut map_events: ResMut<MapEvents>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        println!("Processing {:?}", map_event);

        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                VisibleEvent::ExploreEvent => {
                    debug!("Processing ExploreEvent");

                    Resource::explore(
                        map_event.obj_id,
                        Position {
                            x: map_event.pos_x,
                            y: map_event.pos_y,
                        },
                        &mut resources,
                        &templates.res_templates,
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
        println!("Checking if processed map event is visible...");

        // Get event object components.  eo => event_object
        if let Ok((
            eo_id,
            eo_player_id,
            eo_pos,
            eo_name,
            eo_template,
            eo_class,
            eo_subclass,
            eo_state,
            eo_viewshed,
            eo_misc,
        )) = set.p0().get(map_event.entity_id)
        {
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
                            println!("Send obj create to client");

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
                    VisibleEvent::DamageEvent {target_id, target_pos, attack_type , damage, state } => {
                        let attacker_distance =  Map::distance((map_event.pos_x, map_event.pos_y), (pos.x, pos.y));

                        if viewshed.range >= attacker_distance {
                            let damage_event = BroadcastEvents::Damage {
                                sourceid: map_event.obj_id, 
                                targetid: *target_id, 
                                attacktype: attack_type.to_string(), 
                                dmg: *damage, 
                                state: state.to_string(), 
                                combo: "false".to_string(), 
                                countered: "false".to_string() };

                            all_broadcast_events
                                .entry(player_id.0)
                                .or_default()
                                .insert(damage_event);
                        }

                        let target_distance =  Map::distance((target_pos.x, target_pos.y), (pos.x, pos.y));

                        if viewshed.range >= target_distance {
                            let damage_event = BroadcastEvents::Damage {
                                sourceid: map_event.obj_id, 
                                targetid: *target_id, 
                                attacktype: attack_type.to_string(), 
                                dmg: *damage, 
                                state: state.to_string(), 
                                combo: "false".to_string(), 
                                countered: "false".to_string() };

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
                            println!("Send obj update to client");

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

    trace!("Perceptions to update: {:?}", perception_updates);

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

fn update_game_tick(mut game_tick: ResMut<GameTick>, mut attrs: Query<(&mut Thirst, &mut Morale)>) {
    game_tick.0 = game_tick.0 + 1;

    // Update thirst
    for (mut thirst, mut morale) in &mut attrs {
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
        let is_blocking = is_blocking_state(&q.state.0);

        if player_id != q.player_id.0 && x == q.pos.x && y == q.pos.y && is_blocking {
            objs.push(q.entity);
        }
    }

    return objs.len() == 0;
}

pub fn is_blocking_state(state_str: &str) -> bool {
    let result = match state_str {
        DEAD => false,
        FOUNDED => false,
        PROGRESSING => false,
        _ => true,
    };

    result
}

pub fn is_none_state(state_str: &str) -> bool {
    let is_none_state = state_str == NONE;

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
