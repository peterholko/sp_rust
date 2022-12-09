use bevy::ecs::query::WorldQuery;
use bevy::ecs::schedule::ShouldRun;
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};

use itertools::{Itertools, Update};
use serde_json::{Number, Value};
// use tungstenite::handshake::client::Response;

use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::{
    collections::HashMap,
    collections::HashSet,
    hash::Hash,
    sync::{Arc, Mutex},
};

use crossbeam_channel::{unbounded, Receiver as CBReceiver};
use tokio::sync::mpsc::Sender;

use async_compat::Compat;

use crate::map::{Map, MapPlugin, MapTile};
use crate::network;
use crate::network::ResponsePacket;

pub struct GamePlugin;

//pub type Clients = Arc<Mutex<HashMap<i32, Client>>>;
pub type Accounts = Arc<Mutex<HashMap<i32, Account>>>;

#[derive(Resource, Deref, DerefMut, Clone, Debug)]
pub struct Clients(Arc<Mutex<HashMap<i32, Client>>>);

#[derive(Resource, Deref, DerefMut)]
struct NetworkReceiver(CBReceiver<PlayerEvent>);

#[derive(Resource, Deref, DerefMut)]
struct MapEvents(HashMap<i32, MapEvent>);

#[derive(Resource, Deref, DerefMut)]
struct ProcessedMapEvents(Vec<MapEvent>);

#[derive(Resource, Deref, DerefMut, Debug, Default)]
struct GameTick(i32);

// Indexes for IDs
#[derive(Resource, Clone, Debug)]
struct Ids {
    map_event: i32,
    obj: i32,
    item: i32,
}

#[derive(Resource, Deref, DerefMut, Debug)]
struct PerceptionUpdates(HashSet<i32>);

#[derive(Resource, Deref, DerefMut, Debug)]
struct Items(Vec<Item>);

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
    pub class: HeroClass,
}

#[derive(Debug, Component, Clone)]
struct Id(i32);

#[derive(Debug, Component)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Debug, Component)]
struct Hp(i32);

#[derive(Debug, Component)]
struct PlayerId(i32);

#[derive(Debug, Component)]
struct Name(String);

#[derive(Debug, Component)]
struct Template(String);

#[derive(Debug, Component)]
struct Class(String);

#[derive(Debug, Component)]
struct Subclass(String);

#[derive(Debug, Component)]
struct State(String);

#[derive(Debug, Component, Clone)]
struct Viewshed {
    entities: HashSet<i32>,
    range: u32,
}

#[derive(Debug, Component)]
struct Hero;

#[derive(Debug, Component)]
struct Misc {
    image: String,
    hsl: Vec<i32>,
    groups: Vec<i32>,
}

#[derive(Bundle)]
struct Obj {
    id: Id,
    player_id: PlayerId,
    position: Position,
    name: Name,
    template: Template,
    class: Class,
    subclass: Subclass,
    state: State,
    viewshed: Viewshed,
    misc: Misc,
}

#[derive(Debug, Component)]
pub struct ItemContainer {
  slots: Vec<Option<Item>>,
}

#[derive(Debug)]
struct Item {
    id: i32,
    owner: i32,
    name: String,
    quantity: i32,
    class: String,
    subclass: String,
    image: String,
    weight: i32,
    equipped: bool,
}

#[derive(WorldQuery)]
struct MapObjQuery {
    entity: Entity,
    // It is required that all reference lifetimes are explicitly annotated, just like in any
    // struct. Each lifetime should be 'static.
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    name: &'static Name,
    template: &'static Template,
    class: &'static Class,
    subclass: &'static Subclass,
    state: &'static State,
    viewshed: &'static Viewshed,
    misc: &'static Misc,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum HeroClass {
    Warrior,
    Ranger,
    Mage,
    None,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum States {
    None,
    Dead,
    Founded,
    Progressing,
    Moving,
}

// States
// const NONE: String = String::from("none");
// const MOVING: String = String::from("moving");

#[derive(Clone, Debug)]
pub enum PlayerEvent {
    NewPlayer {
        player_id: i32,
    },
    Move {
        player_id: i32,
        x: i32,
        y: i32,
    },
    Attack {
        player_id: i32,
        attacktype: String,
        sourceid: i32,
        targetid: i32,
    },
    InfoObj {
        player_id: i32,
        id: i32,
    },
    InfoTile {
        player_id: i32,
        x: i32,
        y: i32,
    },
    InfoInventory {
        player_id: i32,
        id: i32
    }
}

#[derive(Clone, Debug)]
struct MapEvent {
    event_id: i32,
    entity_id: Entity,
    obj_id: i32,
    player_id: i32,
    pos_x: i32,
    pos_y: i32,
    run_tick: i32,
    map_event_type: MapEventType,
}

#[derive(Clone, Debug)]
enum MapEventType {
    NewObjEvent,
    MoveEvent { dst_x: i32, dst_y: i32 },
    StateChangeEvent { new_state: String },
}

#[derive(Clone, Debug)]
struct ExploredMap {
    player_id: i32,
    tiles: HashSet<i32>,
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(MapPlugin)
            .init_resource::<GameTick>()
            .add_startup_system(Game::setup)
            .add_system_to_stage(CoreStage::PreUpdate, update_game_tick)
            .add_system(message_system)
            .add_system(event_system)
            .add_system(processed_event_system)
            .add_system(perception_system)
            .add_system(print_items_system);
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

        // Initialize game events vector

        let map_events: MapEvents = MapEvents(HashMap::new());
        let processed_map_events: ProcessedMapEvents = ProcessedMapEvents(Vec::new());

        let perception_updates: PerceptionUpdates = PerceptionUpdates(HashSet::new());

        let items = Items(Vec::new());

        //Initialize Arc Mutex Hashmap to store the client to game channel per connected client

        let clients = Clients(Arc::new(Mutex::new(HashMap::new())));
        let accounts = Accounts::new(Mutex::new(HashMap::new()));

        //Add accounts
        let account = Account {
            player_id: 1,
            username: "peter".to_string(),
            password: "123123".to_string(),
            class: HeroClass::None,
        };

        let account2 = Account {
            player_id: 2,
            username: "joe".to_string(),
            password: "123123".to_string(),
            class: HeroClass::None,
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
        commands.insert_resource(items);
    }
}

fn message_system(
    commands: Commands,
    game_tick: ResMut<GameTick>,
    clients: Res<Clients>,
    client_to_game_receiver: Res<NetworkReceiver>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    map: Res<Map>,
    items: Res<Items>,
    hero_query: Query<
        (
            Entity,
            &Id,
            &Position,
            &PlayerId,
            &Name,
            &Template,
            &Class,
            &Subclass,
            &State,
        ),
        With<Hero>,
    >,
    query: Query<MapObjQuery>,
) {
    //Attempts to receive a message from the channel without blocking.
    if let Ok(evt) = client_to_game_receiver.try_recv() {
        println!("{:?}", evt);
        let _res = match evt {
            PlayerEvent::NewPlayer { player_id } => {
                new_player(player_id, commands, ids, map_events, game_tick);
            }
            PlayerEvent::Move { player_id, x, y } => {
                println!("looking for obj");
                for (
                    entity_id,
                    obj_id,
                    pos,
                    obj_player_id,
                    _name,
                    _template,
                    _class,
                    _subclass,
                    _state,
                ) in hero_query.iter()
                {
                    // Check find hero from Move Event player
                    if player_id != obj_player_id.0 {
                        continue;
                    }

                    if !Map::is_passable(x, y, &map) {
                        println!("Position is not passable");
                        let error = ResponsePacket::Error {
                            errmsg: "Tile is not passable.".to_owned(),
                        };
                        send_to_client(player_id, error, &clients);
                        return;
                    };

                    if !is_pos_empty(player_id, x, y, &query) {
                        println!("Position is not empty");
                        let error = ResponsePacket::Error {
                            errmsg: "Tile is occupied.".to_owned(),
                        };
                        send_to_client(player_id, error, &clients);
                        return;
                    }

                    // Add State Change Event to Moving
                    let state_change_event = MapEventType::StateChangeEvent {
                        new_state: "moving".to_string(),
                    };

                    map_events.new(
                        ids.new_map_event_id(),
                        entity_id,
                        obj_id,
                        obj_player_id,
                        pos,
                        game_tick.0,
                        state_change_event,
                    );

                    // Add Move Event
                    let move_event = MapEventType::MoveEvent { dst_x: x, dst_y: y };

                    map_events.new(
                        ids.new_map_event_id(),
                        entity_id,
                        obj_id,
                        obj_player_id,
                        pos,
                        game_tick.0 + 4, // in the future
                        move_event,
                    );

                    // Add State Change Event back to None
                    let state_change_event = MapEventType::StateChangeEvent {
                        new_state: "none".to_string(),
                    };

                    map_events.new(
                        ids.new_map_event_id(),
                        entity_id,
                        obj_id,
                        obj_player_id,
                        pos,
                        game_tick.0 + 4, // in the future
                        state_change_event,
                    );
                }
            }
            PlayerEvent::Attack {
                player_id,
                attacktype: String,
                sourceid,
                targetid,
            } => {
                println!("PlayerEvent::Attack");
            }
            PlayerEvent::InfoObj { player_id, id } => {
                println!(
                    "PlayerEvent::InfoObj player_id: {:?} id: {:?}",
                    player_id, id
                );

                for q in &query {
                    if q.id.0 == id {
                        let info_obj_packet: ResponsePacket = ResponsePacket::InfoObj {
                            id: q.id.0,
                            name: q.name.0.to_owned(),
                            template: q.template.0.to_owned(),
                            class: q.class.0.to_owned(),
                            subclass: q.subclass.0.to_owned(),
                            state: q.state.0.to_owned(),
                        };

                        send_to_client(player_id, info_obj_packet, &clients);
                    }
                }
            }
            PlayerEvent::InfoTile { player_id, x, y } => {
                println!("PlayerEvent::InfoTile x: {:?} y: {:?}", x, y);

                let info_tile_packet: ResponsePacket = ResponsePacket::InfoTile {
                    x: x,
                    y: y,
                    name: "Tile Name".to_owned(),
                    mc: 2,
                    def: 2,
                    unrevealed: "true".to_owned(),
                    sanctuary: "true".to_owned(),
                    passable: "true".to_owned(),
                    wildness: "high".to_owned(),
                    resources: "none".to_owned(),
                };

                send_to_client(player_id, info_tile_packet, &clients);
            }
            PlayerEvent::InfoInventory { player_id, id } => {
                println!("PlayerEvent::InfoInventory id: {:?}", id);


                
                let info_inventory_packet: ResponsePacket = ResponsePacket::InfoInventory { id: id, cap: 100, tw: 100, items: () };
            }
        };
    }
}

fn event_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    clients: Res<Clients>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    mut processed_map_events: ResMut<ProcessedMapEvents>,
    mut perception_updates: ResMut<PerceptionUpdates>,
    mut set: ParamSet<(
        Query<(&Id, &PlayerId, &mut Position, &mut State, &Viewshed)>, // p0 mutable for the event processing
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
        )>, // p1 immutable for looking up other entities
    )>,
) {
    println!("Game Tick {:?}", game_tick.0);

    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        println!("Processing {:?}", map_event);

        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                MapEventType::NewObjEvent => {
                    println!("Processing NewObjEvent");
                    perception_updates.insert(map_event.player_id);

                    processed_map_events.push(map_event.clone());
                    events_to_remove.push(*map_event_id);
                }
                MapEventType::MoveEvent { dst_x, dst_y } => {
                    println!("Processing MoveEvent");

                    // Check if destination is open
                    let mut is_dst_open = true;

                    for (
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
                        if let Ok((id, player_id, mut pos, mut state, viewshed)) =
                            set.p0().get_mut(map_event.entity_id)
                        {
                            pos.x = *dst_x;
                            pos.y = *dst_y;

                            println!("Adding processed map event");
                            // Adding processed map event
                            processed_map_events.push(map_event.clone());

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
                MapEventType::StateChangeEvent { new_state } => {
                    println!("Processing StateChangeEvent");

                    // Get entity and update state
                    if let Ok((id, playerId, mut pos, mut state, _viewshed)) =
                        set.p0().get_mut(map_event.entity_id)
                    {
                        state.0 = new_state.to_string();

                        println!("Adding processed map event");
                        processed_map_events.push(map_event.clone());
                    }

                    events_to_remove.push(*map_event_id);
                }
            }
        }
    }

    for event_id in events_to_remove.iter() {
        map_events.remove(event_id);
    }
}

fn processed_event_system(
    clients: Res<Clients>,
    mut processed_map_events: ResMut<ProcessedMapEvents>,
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
    let mut all_change_events: HashMap<i32, HashSet<network::ChangeEvents>> = HashMap::new();

    for map_event in processed_map_events.iter() {
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
            let new_obj = network_obj(
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
                    MapEventType::NewObjEvent => {
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
                    MapEventType::MoveEvent { dst_x, dst_y } => {
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
                    MapEventType::StateChangeEvent { new_state } => {
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

    processed_map_events.clear();
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

    println!("Perceptions to update: {:?}", perception_updates);

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

//fn update_game_tick(mut game_tick: ResMut<GameTick>, query: Query<(Entity, &MapObj)>) {
fn update_game_tick(
    mut game_tick: ResMut<GameTick>,
    query: Query<(Entity, &Id, &Name, &Position)>,
) {
    game_tick.0 = game_tick.0 + 1;
}

fn print_items_system(
    query: Query<(Entity, &Id, &Name, &Position, &ItemContainer)>,
) {
    for (e, id, name, pos, items) in query.iter() {
        println!("id: {:?} name: {:?} items: {:?}", id, name, items.slots);
    }
}


fn new_player(
    player_id: i32,
    mut commands: Commands,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    game_tick: ResMut<GameTick>,
) {
    let start_x = 16;
    let start_y = 36;
    let range = 2;

    // Create Hero Obj
    let hero_id = ids.new_obj_id();

    let hero = Obj {
        id: Id(hero_id),
        player_id: PlayerId(player_id),
        position: Position {
            x: start_x,
            y: start_y,
        },
        name: Name("Peter".into()),
        template: Template("Novice Warrior".into()),
        class: Class("unit".into()),
        subclass: Subclass("hero".into()),
        state: State("none".into()),
        viewshed: Viewshed {
            entities: HashSet::new(),
            range: range,
        },
        misc: Misc {
            image: "novicewarrior".into(),
            hsl: Vec::new().into(),
            groups: Vec::new().into(),
        },
    };

    // Create hero items
    let berries = Item::new(ids.new_item_id(), hero_id, "Honeybell Berries".to_string(), 25);
    let water = Item::new(ids.new_item_id(), hero_id, "Spring Water".to_string(), 25);

    let hero_items = ItemContainer {
        slots: vec![Some(berries), Some(water)]
    };

    // Spawn hero
    let hero_entity_id = commands.spawn((
        hero,
        hero_items,
        Hero
    )).id();

    // Insert new obj event
    let new_obj_event = MapEventType::NewObjEvent;
    let map_event_id = ids.new_map_event_id();

    let map_state_event = MapEvent {
        event_id: map_event_id,
        entity_id: hero_entity_id,
        obj_id: hero_id,
        player_id: player_id,
        pos_x: start_x,
        pos_y: start_y,
        run_tick: game_tick.0 + 1, // Add one game tick
        map_event_type: new_obj_event,
    };

    map_events.insert(map_event_id, map_state_event);

    // Villager obj

    let villager_id = ids.new_obj_id();

    let villager = Obj {
        id: Id(villager_id),
        player_id: PlayerId(player_id),
        position: Position { x: 16, y: 35 },
        name: Name("Villager 1".into()),
        template: Template("Human Villager".into()),
        class: Class("unit".into()),
        subclass: Subclass("villager".into()),
        state: State("none".into()),
        viewshed: Viewshed {
            entities: HashSet::new(),
            range: 2,
        },
        misc: Misc {
            image: "humanvillager1".into(),
            hsl: Vec::new().into(),
            groups: Vec::new().into(),
        },
    };

    let villager_entity_id = commands.spawn(villager).id();

    // Insert state change event
    let new_obj_event = MapEventType::NewObjEvent;
    let map_event_id = ids.new_map_event_id();

    let map_state_event = MapEvent {
        event_id: map_event_id,
        entity_id: villager_entity_id,
        obj_id: villager_id,
        player_id: player_id,
        pos_x: 16,
        pos_y: 35,
        run_tick: game_tick.0 + 1, // Add one game tick
        map_event_type: new_obj_event,
    };

    map_events.insert(map_event_id, map_state_event);

    //create_item(commands, heroId, "Honeybell Berries".to_owned(), "Food".to_owned(), "Berry".to_owned(), "honeybellberries".to_owned(), 5, 10);
}

fn network_obj(
    id: i32,
    player_id: i32,
    x: i32,
    y: i32,
    name: String,
    template: String,
    class: String,
    subclass: String,
    state: String,
    vision: u32,
    image: String,
    hsl: Vec<i32>,
    groups: Vec<i32>,
) -> network::MapObj {
    let network_obj = network::MapObj {
        id: id,
        player: player_id,
        x: x,
        y: y,
        name: name,
        template: template,
        class: class,
        subclass: subclass,
        state: state,
        vision: vision,
        image: image,
        hsl: hsl,
        groups: groups,
    };

    network_obj
}

fn dedup<T: Eq + Hash + Copy>(v: &mut Vec<T>) {
    // note the Copy constraint
    let mut uniques = HashSet::new();
    v.retain(|e| uniques.insert(*e));
}

fn is_pos_empty(player_id: i32, x: i32, y: i32, query: &Query<MapObjQuery>) -> bool {
    let mut objs = Vec::new();

    for q in query {
        let is_blocking = is_blocking_state(&q.state.0);

        if player_id != q.player_id.0 && x == q.pos.x && y == q.pos.y && is_blocking {
            objs.push(q.entity);
        }
    }

    return objs.len() == 0;
}

fn from_string(state: &str) -> States {
    match state {
        "Dead" => States::Dead,
        "Founded" => States::Founded,
        "Progressing" => States::Progressing,
        "Moving" => States::Moving,
        _ => States::None,
    }
}

fn is_blocking_state(state_str: &str) -> bool {
    let state = from_string(state_str);

    let result = match state {
        States::Dead => false,
        States::Founded => false,
        States::Progressing => false,
        _ => true,
    };

    result
}

fn send_to_client(player_id: i32, packet: ResponsePacket, clients: &Res<Clients>) {
    for (_client_id, client) in clients.lock().unwrap().iter() {
        println!("Player: {:?} == client: {:?}", player_id, client);
        if client.player_id == player_id {
            client
                .sender
                .try_send(serde_json::to_string(&packet).unwrap())
                .expect("Could not send message");
        }
    }
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
        map_event_type: MapEventType,
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

impl Item {
    pub fn new(id: i32, owner: i32, name: String, quantity: i32)-> Item {

        Item {
            id: id,
            owner: owner,
            name: name,
            quantity: quantity,
            class: "Food".to_string(),
            subclass: "Berries".to_string(),
            image: "berries".to_string(),
            weight: 1,
            equipped: false
        }
    }
}