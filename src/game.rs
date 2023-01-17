use bevy::ecs::query::WorldQuery;
use bevy::utils::tracing::{debug, trace};
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};

use big_brain::prelude::*;

use itertools::{Itertools, Update};

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

use crate::ai::{AIPlugin, Drink, HighMorale, Morale, ProcessOrder, Thirst, Thirsty};
use crate::item::{Item, ItemPlugin, Items};
use crate::map::{Map, MapPlugin, MapTile};
use crate::network;
use crate::network::ResponsePacket;
use crate::resource::{Resource, ResourcePlugin, Resources};
use crate::skill::{Skill, SkillPlugin, Skills};
use crate::structure::Structure;
use crate::templates::{Templates, TemplatesPlugin};

pub struct GamePlugin;

//pub type Clients = Arc<Mutex<HashMap<i32, Client>>>;
pub type Accounts = Arc<Mutex<HashMap<i32, Account>>>;

#[derive(Resource, Deref, DerefMut, Clone, Debug)]
pub struct Clients(Arc<Mutex<HashMap<i32, Client>>>);

#[derive(Resource, Deref, DerefMut)]
struct NetworkReceiver(CBReceiver<PlayerEvent>);

#[derive(Resource, Deref, DerefMut)]
pub struct Events(pub Vec<PlayerEvent>);

#[derive(Resource, Deref, DerefMut)]
pub struct MapEvents(pub HashMap<i32, MapEvent>);

#[derive(Resource, Deref, DerefMut)]
struct ProcessedMapEvents(Vec<MapEvent>);

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

#[derive(Debug, Component)]
struct Hp(i32);

#[derive(Debug, Component)]
pub struct PlayerId(pub i32);

#[derive(Debug, Component)]
struct Name(String);

#[derive(Debug, Component)]
struct Template(String);

#[derive(Debug, Component)]
struct Class(String);

#[derive(Debug, Component)]
struct Subclass(String);

#[derive(Debug, Component, Clone, Eq, PartialEq, Hash)]
pub struct State(pub String);

#[derive(Debug, Component, Clone)]
struct Viewshed {
    entities: HashSet<i32>,
    range: u32,
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
struct Misc {
    image: String,
    hsl: Vec<i32>,
    groups: Vec<i32>,
}

#[derive(Debug, Component)]
struct StructureAttrs {
    start_time: i32,
    end_time: i32,
    build_time: i32,
    builder: i32,
    progress: i32,
}

#[derive(Debug, Component)]
pub struct OrderFollow {
    pub target: Entity,
}

#[derive(Debug, Component)]
pub struct EventInProgress;

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
pub enum HeroClassList {
    Warrior,
    Ranger,
    Mage,
    None,
}

// States
const NONE: &str = "none";
const MOVING: &str = "moving";
const DEAD: &str = "dead";
const FOUNDED: &str = "founded";
const PROGRESSING: &str = "progressing";
const BUILDING: &str = "building";

#[derive(Resource, Clone, Debug)]
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
    Combo {
        player_id: i32,
        source_id: i32,
        combo_type: String,
    },
    Gather {
        player_id: i32,
        source_id: i32,
        res_type: String,
    },
    InfoObj {
        player_id: i32,
        id: i32,
    },
    InfoSkills {
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
        id: i32,
    },
    InfoItem {
        player_id: i32,
        id: i32,
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
    Survey {
        player_id: i32,
        source_id: i32,
    },
    Explore {
        player_id: i32,
    },
}

#[derive(Clone, Debug)]
pub struct MapEvent {
    pub event_id: i32,
    pub entity_id: Entity,
    pub obj_id: i32,
    pub player_id: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub run_tick: i32,
    pub map_event_type: MapEventType,
}

#[derive(Clone, Debug)]
pub enum MapEventType {
    NewObjEvent { new_player: bool },
    StateChangeEvent { new_state: String },
    MoveEvent { dst_x: i32, dst_y: i32 },
    GatherEvent { res_type: String },
    ExploreEvent,
}

#[derive(Clone, Debug)]
struct ExploredMap {
    player_id: i32,
    tiles: HashSet<i32>,
}

// Used as temporary obj storage for system
pub struct TmpObj {
    pub entity: Entity,
    pub obj_id: i32,
    pub player_id: i32,
    pub pos: Position,
    pub state: State,
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(MapPlugin)
            .add_plugin(AIPlugin)
            .add_plugin(TemplatesPlugin)
            .add_plugin(ItemPlugin)
            .add_plugin(ResourcePlugin)
            .add_plugin(SkillPlugin)
            .init_resource::<GameTick>()
            .add_startup_system(Game::setup)
            .add_system_to_stage(CoreStage::PreUpdate, update_game_tick)
            .add_system(new_player_system)
            .add_system(move_system)
            .add_system(gather_system)
            .add_system(info_obj_system)
            .add_system(info_tile_system)
            .add_system(info_item_system)
            .add_system(item_split_system)
            .add_system(order_follow_system)
            .add_system(structure_list_system)
            .add_system(create_foundation_system)
            .add_system(build_system)
            .add_system(explore_system)
            .add_system(new_obj_event_system)
            .add_system(move_event_system)
            .add_system(state_change_event_system)
            .add_system(gather_event_system)
            .add_system(explore_event_system)
            .add_system(message_broker_system)
            .add_system(processed_event_system)
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

        // Initialize events
        let events: Events = Events(Vec::new());

        // Initialize map events vector
        let map_events: MapEvents = MapEvents(HashMap::new());
        let processed_map_events: ProcessedMapEvents = ProcessedMapEvents(Vec::new());

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
        commands.insert_resource(events);
        commands.insert_resource(map_events);
        commands.insert_resource(processed_map_events);
        commands.insert_resource(perception_updates);
        commands.insert_resource(ids);
    }
}

fn message_broker_system(
    client_to_game_receiver: Res<NetworkReceiver>,
    mut events: ResMut<Events>,
    /*mut new_player_events: EventWriter<NewPlayerEvent>,
    mut move_events: EventWriter<MoveEvent>,
    mut gather_events: EventWriter<GatherEvent>,
    mut info_obj_events: EventWriter<InfoObjEvent>,
    mut info_tile_events: EventWriter<InfoTileEvent>,
    mut info_inventory_events: EventWriter<InfoInventoryEvent>,
    mut info_item_events: EventWriter<InfoItemEvent>,
    mut info_item_by_name_events: EventWriter<InfoItemByNameEvent>,
    mut info_item_transfer_events: EventWriter<InfoItemTransferEvent>,
    mut item_transfer_events: EventWriter<ItemTransferEvent>,
    mut item_split_events: EventWriter<ItemSplitEvent>,
    mut order_follow_events: EventWriter<OrderFollowEvent>,
    mut structure_list_events: EventWriter<StructureListEvent>,
    mut create_foundation_events: EventWriter<CreateFoundationEvent>,*/
) {
    if let Ok(evt) = client_to_game_receiver.try_recv() {
        println!("{:?}", evt);

        events.push(evt.clone());

        /*let _res = match evt {
            PlayerEvent::NewPlayer { player_id } => {
                new_player_events.send(NewPlayerEvent {
                    player_id: player_id,
                });
            }
            PlayerEvent::Move { player_id, x, y } => {
                move_events.send(MoveEvent {
                    player_id: player_id,
                    pos: Position { x: x, y: y },
                });
            }
            PlayerEvent::Attack {
                player_id,
                attacktype: String,
                sourceid,
                targetid,
            } => {}
            PlayerEvent::Combo {
                player_id,
                source_id,
                combo_type,
            } => {
                debug!("PlayerEvent::Combo");
            }
            PlayerEvent::Gather {
                player_id,
                source_id,
                res_type,
            } => {
                debug!("PlayerEvent::Gather");
                gather_events.send(GatherEvent {
                    player_id: player_id,
                    source_id: source_id,
                    res_type: res_type,
                })
            }
            PlayerEvent::InfoObj { player_id, id } => {
                info_obj_events.send(InfoObjEvent {
                    player_id: player_id,
                    id: id,
                });
            }
            PlayerEvent::InfoSkills { player_id, id } => {}
            PlayerEvent::InfoTile { player_id, x, y } => {
                info_tile_events.send(InfoTileEvent {
                    player_id: player_id,
                    x: x,
                    y: y,
                });
            }
            PlayerEvent::InfoInventory { player_id, id } => {
                info_inventory_events.send(InfoInventoryEvent {
                    player_id: player_id,
                    id: id,
                });
            }
            PlayerEvent::InfoItem { player_id, id } => {
                info_item_events.send(InfoItemEvent {
                    player_id: player_id,
                    id: id,
                });
            }
            PlayerEvent::InfoItemByName { player_id, name } => {
                info_item_by_name_events.send(InfoItemByNameEvent {
                    player_id: player_id,
                    name: name,
                });
            }
            PlayerEvent::InfoItemTransfer {
                player_id,
                source_id,
                target_id,
            } => {
                info_item_transfer_events.send(InfoItemTransferEvent {
                    player_id: player_id,
                    source_id: source_id,
                    target_id: target_id,
                });
            }
            PlayerEvent::ItemTransfer {
                player_id,
                target_id,
                item_id,
            } => {
                item_transfer_events.send(ItemTransferEvent {
                    player_id: player_id,
                    target_id: target_id,
                    item_id: item_id,
                });
            }
            PlayerEvent::ItemSplit {
                player_id,
                item_id,
                quantity,
            } => {
                item_split_events.send(ItemSplitEvent {
                    player_id: player_id,
                    item_id: item_id,
                    quantity: quantity,
                });
            }
            PlayerEvent::OrderFollow {
                player_id,
                source_id,
            } => {
                order_follow_events.send(OrderFollowEvent {
                    player_id: player_id,
                    source_id: source_id,
                });
            }
            PlayerEvent::StructureList { player_id } => {
                structure_list_events.send(StructureListEvent {
                    player_id: player_id,
                });
            }
            PlayerEvent::CreateFoundation {
                player_id,
                source_id,
                structure_name,
            } => {
                create_foundation_events.send(CreateFoundationEvent {
                    player_id: player_id,
                    source_id: source_id,
                    structure_name: structure_name,
                });
            }
            PlayerEvent::Build {
                player_id,
                source_id,
                structure_id,
            } => {
                /*build_events.send(BuildEvent {
                    player_id: player_id,
                    source_id: source_id,
                    structure_id: structure_id,
                });*/
            }
            PlayerEvent::Explore { player_id } => {
                /*explore_events.send(ExploreEvent {
                    player_id: player_id,
                }); */
            }
            PlayerEvent::Survey {
                player_id,
                source_id,
            } => {}
        }; */
    }
}

fn new_player_system(
    mut events: ResMut<Events>,
    mut commands: Commands,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::NewPlayer { player_id } => {
                new_player(
                    *player_id,
                    &mut commands,
                    &mut ids,
                    &mut map_events,
                    &mut items,
                    &templates,
                    &game_tick,
                );

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn move_system(
    mut events: ResMut<Events>,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    map: Res<Map>,
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
        With<SubclassHero>,
    >,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::Move { player_id, x, y } => {
                debug!("Move Event: {:?}", event);
                let player_id = player_id;

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
                    if *player_id != obj_player_id.0 {
                        continue;
                    }

                    if !Map::is_passable(*x, *y, &map) {
                        println!("Position is not passable");
                        let error = ResponsePacket::Error {
                            errmsg: "Tile is not passable.".to_owned(),
                        };
                        send_to_client(*player_id, error, &clients);
                        return;
                    };

                    if !is_pos_empty(*player_id, *x, *y, &query) {
                        println!("Position is not empty");
                        let error = ResponsePacket::Error {
                            errmsg: "Tile is occupied.".to_owned(),
                        };
                        send_to_client(*player_id, error, &clients);
                        return;
                    }

                    // Add State Change Event to Moving
                    let state_change_event = MapEventType::StateChangeEvent {
                        new_state: MOVING.to_string(),
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
                    let move_event = MapEventType::MoveEvent {
                        dst_x: *x,
                        dst_y: *y,
                    };

                    map_events.new(
                        ids.new_map_event_id(),
                        entity_id,
                        obj_id,
                        obj_player_id,
                        pos,
                        game_tick.0 + 4, // in the future
                        move_event,
                    );
                }

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn gather_system(
    mut events: ResMut<Events>,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut skills: ResMut<Skills>,
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
        With<SubclassHero>,
    >,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::Gather {
                player_id,
                source_id,
                res_type,
            } => {
                debug!("PlayerEvent::Gather");

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
                    // Check find hero from Gather event
                    if *player_id != obj_player_id.0 {
                        continue;
                    }

                    let gather_event = MapEventType::GatherEvent {
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
                        ids.new_map_event_id(),
                        entity_id,
                        obj_id,
                        obj_player_id,
                        pos,
                        game_tick.0 + 8, // in the future
                        gather_event,
                    );

                    debug!("Skills: {:?}", skills);
                }

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn info_obj_system(mut events: ResMut<Events>, clients: Res<Clients>, query: Query<MapObjQuery>) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::InfoObj { player_id, id } => {
                for q in &query {
                    if q.id.0 == *id {
                        let info_obj_packet: ResponsePacket = ResponsePacket::InfoObj {
                            id: q.id.0,
                            name: q.name.0.to_owned(),
                            template: q.template.0.to_owned(),
                            class: q.class.0.to_owned(),
                            subclass: q.subclass.0.to_owned(),
                            state: q.state.0.to_owned(),
                        };

                        send_to_client(*player_id, info_obj_packet, &clients);
                    }
                }

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn info_tile_system(
    mut events: ResMut<Events>,
    clients: Res<Clients>,
    map: Res<Map>,
    resources: Res<Resources>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::InfoTile { player_id, x, y } => {
                debug!("PlayerEvent::InfoTile x: {:?} y: {:?}", *x, *y);

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
                };

                send_to_client(*player_id, info_tile_packet, &clients);

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn info_item_system(mut events: ResMut<Events>, clients: Res<Clients>, mut items: ResMut<Items>) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::InfoInventory { player_id, id } => {
                debug!("PlayerEvent::InfoInventory id: {:?}", id);

                let inventory_items = Item::get_by_owner_packet(*id, &items);

                let info_inventory_packet: ResponsePacket = ResponsePacket::InfoInventory {
                    id: *id,
                    cap: 100,
                    tw: 100,
                    items: inventory_items,
                };

                send_to_client(*player_id, info_inventory_packet, &clients);

                events_to_remove.push(index);
            }
            PlayerEvent::InfoItem { player_id, id } => {
                let item = Item::get_packet(*id, &items);

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
                    };

                    send_to_client(*player_id, info_item_packet, &clients);
                }

                events_to_remove.push(index);
            }
            PlayerEvent::InfoItemByName { player_id, name } => {
                debug!("PlayerEvent::InfoItemByName name: {:?}", name.clone());

                let item = Item::get_by_name_packet(name.clone(), &items);

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
                    };

                    send_to_client(*player_id, info_item_packet, &clients);
                }

                events_to_remove.push(index);
            }
            PlayerEvent::InfoItemTransfer {
                player_id,
                source_id,
                target_id,
            } => {
                debug!(
                    "PlayerEvent::InfoItemTransfer sourceid: {:?} targetid: {:?}",
                    *source_id, *target_id
                );

                let source_items = Item::get_by_owner_packet(*source_id, &items);
                let target_items = Item::get_by_owner_packet(*target_id, &items);

                let source_inventory = network::Inventory {
                    id: *source_id,
                    cap: 100,
                    tw: 5,
                    items: source_items,
                };

                let target_inventory = network::Inventory {
                    id: *target_id,
                    cap: 100,
                    tw: 5,
                    items: target_items,
                };

                let info_item_transfer_packet: ResponsePacket = ResponsePacket::InfoItemTransfer {
                    sourceid: *source_id,
                    sourceitems: source_inventory,
                    targetid: *target_id,
                    targetitems: target_inventory,
                    reqitems: Vec::new(),
                };

                send_to_client(*player_id, info_item_transfer_packet, &clients);

                events_to_remove.push(index);
            }
            PlayerEvent::ItemTransfer {
                player_id,
                target_id,
                item_id,
            } => {
                if let Some(item) = Item::find_by_id(*item_id, &items) {
                    Item::transfer(*item_id, *target_id, &mut items);

                    let source_items = Item::get_by_owner_packet(item.owner, &items);
                    let target_items = Item::get_by_owner_packet(*target_id, &items);

                    let source_inventory = network::Inventory {
                        id: item.owner,
                        cap: 100,
                        tw: 5,
                        items: source_items,
                    };

                    let target_inventory = network::Inventory {
                        id: *target_id,
                        cap: 100,
                        tw: 5,
                        items: target_items,
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

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn item_split_system(
    mut events: ResMut<Events>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::ItemSplit {
                player_id,
                item_id,
                quantity,
            } => {
                if let Some(item) = Item::find_by_id(*item_id, &items) {
                    // TODO add checks if item_id is owned by player and if quantity is more than item quantity
                    Item::split(
                        *item_id,
                        *quantity,
                        ids.new_item_id(),
                        &mut items,
                        &templates.item_templates,
                    );

                    let item_split_packet: ResponsePacket = ResponsePacket::ItemSplit {
                        result: "success".to_string(),
                        owner: item.owner,
                    };

                    send_to_client(*player_id, item_split_packet, &clients);
                }

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn order_follow_system(
    mut events: ResMut<Events>,
    mut commands: Commands,
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
        With<SubclassHero>,
    >,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::OrderFollow {
                player_id,
                source_id,
            } => {
                for (
                    entity,
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
                    if *player_id != obj_player_id.0 {
                        continue;
                    }

                    for q in &query {
                        if q.id.0 == *source_id {
                            commands
                                .entity(q.entity)
                                .insert(OrderFollow { target: entity });
                        }
                    }
                }

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn structure_list_system(
    mut events: ResMut<Events>,
    clients: Res<Clients>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::StructureList { player_id } => {
                events_to_remove.push(index);
                let structure_list = Structure::available_to_build(&templates.obj_templates);

                let structure_list = ResponsePacket::StructureList {
                    result: structure_list,
                };

                send_to_client(*player_id, structure_list, &clients);

            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn create_foundation_system(
    mut events: ResMut<Events>,
    mut commands: Commands,
    game_tick: ResMut<GameTick>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    templates: Res<Templates>,
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
        With<SubclassHero>,
    >,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::CreateFoundation {
                player_id,
                source_id,
                structure_name,
            } => {
                debug!("CreateFoundation");
                events_to_remove.push(index);

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
                    // Check if player matches
                    if *player_id != obj_player_id.0 {
                        continue;
                    }

                    let structure_id = ids.new_obj_id();

                    if let Some(structure_template) =
                        Structure::get(structure_name.clone(), &templates.obj_templates)
                    {
                        let structure = Obj {
                            id: Id(structure_id),
                            player_id: PlayerId(*player_id),
                            position: Position { x: pos.x, y: pos.y },
                            name: Name(structure_name.clone()),
                            template: Template(structure_template.template.clone()),
                            class: Class(structure_template.class),
                            subclass: Subclass(structure_template.subclass),
                            state: State("founded".into()),
                            viewshed: Viewshed {
                                entities: HashSet::new(),
                                range: 0,
                            },
                            misc: Misc {
                                image: structure_template.template.to_string().to_lowercase(),
                                hsl: Vec::new().into(),
                                groups: Vec::new().into(),
                            },
                        };

                        let structure_attrs = StructureAttrs {
                            start_time: 0,
                            end_time: 0,
                            build_time: structure_template.build_time.unwrap(), // Structure must have build time
                            builder: *source_id,
                            progress: 0,
                        };

                        let structure_entity_id = commands
                            .spawn((structure, structure_attrs, ClassStructure))
                            .id();

                        // Insert new obj event
                        let new_obj_event = MapEventType::NewObjEvent { new_player: false };
                        let map_event_id = ids.new_map_event_id();

                        let map_state_event = MapEvent {
                            event_id: map_event_id,
                            entity_id: structure_entity_id,
                            obj_id: structure_id,
                            player_id: *player_id,
                            pos_x: pos.x,
                            pos_y: pos.y,
                            run_tick: game_tick.0 + 1, // Add one game tick
                            map_event_type: new_obj_event,
                        };

                        map_events.insert(map_event_id, map_state_event);

                        let packet = ResponsePacket::CreateFoundation {
                            result: "success".to_string(),
                        };

                        send_to_client(*player_id, packet, &clients)
                    }
                }

            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn build_system(
    mut events: ResMut<Events>,
    clients: Res<Clients>,
    game_tick: ResMut<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut ids: ResMut<Ids>,
    builder_query: Query<
        (Entity, &Id, &PlayerId, &Position, &State),
        Or<(With<SubclassHero>, With<SubclassVillager>)>,
    >,
    mut structure_query: Query<
        (
            Entity,
            &Id,
            &PlayerId,
            &Position,
            &State,
            &mut StructureAttrs,
        ),
        With<ClassStructure>,
    >,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::Build {
                player_id,
                source_id,
                structure_id,
            } => {
                debug!("Build");
                events_to_remove.push(index);

                let mut builder: Option<TmpObj> = None;
                let mut structure: Option<TmpObj> = None;

                // Get builder
                for (entity, obj_id, player, pos, state) in builder_query.iter() {
                    if obj_id.0 == *source_id {
                        // Get builder
                        builder = Some(TmpObj {
                            entity: entity,
                            obj_id: obj_id.0,
                            player_id: player.0,
                            pos: pos.clone(),
                            state: state.clone(),
                        });
                    }
                }

                // Get structure
                for (entity, obj_id, player_id, pos, state, _structure_attrs) in
                    structure_query.iter()
                {
                    if obj_id.0 == *structure_id {
                        structure = Some(TmpObj {
                            entity: entity,
                            obj_id: obj_id.0,
                            player_id: player_id.0,
                            pos: pos.clone(),
                            state: state.clone(),
                        });
                    }
                }

                if let (Some(builder), Some(structure)) = (builder, structure) {
                    // Check if builder is owned by player
                    if builder.player_id != *player_id {
                        let packet = ResponsePacket::Error {
                            errmsg: "Builder not owned by player."
                                .to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);                        
                        break;
                    }

                    // Check if structure is owned by player
                    if structure.player_id != *player_id {
                        let packet = ResponsePacket::Error {
                            errmsg: "Structure not owned by player."
                                .to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        break;
                    }

                    // Check if builder is on the same pos as structure
                    if builder.pos != structure.pos {
                        let packet = ResponsePacket::Error {
                            errmsg: "Builder must be on the same position as structure."
                                .to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        break;
                    }

                    if let Ok((entity, obj_id, player_id, pos, state, mut structure_attrs)) =
                        structure_query.get_mut(structure.entity)
                    {
                        // Builder State Change Event to Building
                        let state_change_event = MapEventType::StateChangeEvent {
                            new_state: BUILDING.to_string(),
                        };

                        map_events.new(
                            ids.new_map_event_id(),
                            builder.entity,
                            &Id(builder.obj_id),
                            &PlayerId(builder.player_id),
                            &builder.pos,
                            game_tick.0 + 1, // in the future
                            state_change_event,
                        );

                        structure_attrs.start_time = game_tick.0;
                        structure_attrs.end_time = game_tick.0 + structure_attrs.build_time * 2;
                        structure_attrs.builder = *source_id;

                        // Structure State Change Event to Progressing
                        let structure_state_change = MapEventType::StateChangeEvent {
                            new_state: PROGRESSING.to_string(),
                        };

                        map_events.new(
                            ids.new_map_event_id(),
                            entity,
                            obj_id,
                            player_id,
                            pos,
                            game_tick.0 + 1, // in the future
                            structure_state_change,
                        );

                        // Structure State Change Event to None as it completed
                        let structure_state_change = MapEventType::StateChangeEvent {
                            new_state: NONE.to_string(),
                        };

                        map_events.new(
                            ids.new_map_event_id(),
                            entity,
                            obj_id,
                            player_id,
                            pos,
                            structure_attrs.end_time, // in the future
                            structure_state_change,
                        );                        

                        let packet = ResponsePacket::Build {
                            build_time: structure_attrs.build_time / 5 // TODO: Build time in obj_template.yaml should be revisited.
                        };

                        send_to_client(player_id.0, packet, &clients);
                    }
                }
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn explore_system(
    mut events: ResMut<Events>,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
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
        With<SubclassHero>,
    >,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::Explore { player_id } => {
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
                    if *player_id == obj_player_id.0 {
                        // Insert explore event
                        let explore_event = MapEventType::ExploreEvent;
                        let map_event_id = ids.new_map_event_id();

                        let map_state_event = MapEvent {
                            event_id: map_event_id,
                            entity_id: entity_id,
                            obj_id: obj_id.0,
                            player_id: *player_id,
                            pos_x: pos.x,
                            pos_y: pos.y,
                            run_tick: game_tick.0 + 1, // Add one game tick
                            map_event_type: explore_event,
                        };

                        map_events.insert(map_event_id, map_state_event);
                    }
                }

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn new_obj_event_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut processed_map_events: ResMut<ProcessedMapEvents>,
    mut perception_updates: ResMut<PerceptionUpdates>,
) {
    let mut events_to_remove = Vec::new();

    for (map_event_id, map_event) in map_events.iter_mut() {
        println!("Processing {:?}", map_event);

        if map_event.run_tick < game_tick.0 {
            // Execute event
            match &map_event.map_event_type {
                MapEventType::NewObjEvent { new_player } => {
                    println!("Processing NewObjEvent");

                    if *new_player {
                        perception_updates.insert(map_event.player_id);
                    }

                    processed_map_events.push(map_event.clone());
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
    mut processed_map_events: ResMut<ProcessedMapEvents>,
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
                MapEventType::MoveEvent { dst_x, dst_y } => {
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
    mut processed_map_events: ResMut<ProcessedMapEvents>,
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
                MapEventType::StateChangeEvent { new_state } => {
                    println!("Processing StateChangeEvent: {:?}", new_state);

                    // Get entity and update state
                    if let Ok((_entity, id, playerId, mut pos, mut state, _viewshed, ai)) =
                        set.p0().get_mut(map_event.entity_id)
                    {
                        state.0 = new_state.to_string();

                        println!("Adding processed map event");
                        processed_map_events.push(map_event.clone());
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
                MapEventType::GatherEvent { res_type } => {
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
                MapEventType::ExploreEvent => {
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
                    MapEventType::NewObjEvent { new_player } => {
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

fn new_player(
    player_id: i32,
    mut commands: &mut Commands,
    mut ids: &mut ResMut<Ids>,
    mut map_events: &mut ResMut<MapEvents>,
    mut items: &mut ResMut<Items>,
    templates: &Res<Templates>,
    game_tick: &ResMut<GameTick>,
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
    let berries = Item::new(
        ids.new_item_id(),
        hero_id,
        "Honeybell Berries".to_string(),
        25,
        &templates.item_templates,
    );
    let water = Item::new(
        ids.new_item_id(),
        hero_id,
        "Spring Water".to_string(),
        25,
        &templates.item_templates,
    );

    items.push(berries);
    items.push(water);

    // Spawn hero
    let hero_entity_id = commands
        .spawn((
            hero,
            SubclassHero, // Hero component tag
        ))
        .id();

    // Insert new obj event
    let new_obj_event = MapEventType::NewObjEvent { new_player: true };
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

    let water_villager = Item::new(
        ids.new_item_id(),
        villager_id,
        "Spring Water".to_string(),
        50,
        &templates.item_templates,
    );

    items.push(water_villager);

    let villager_entity_id = commands
        .spawn((
            villager,
            SubclassVillager,
            Morale::new(100.0, 1.0),
            Thirst::new(0.0, 0.1),
            Thinker::build()
                .label("My Thinker")
                .picker(FirstToScore { threshold: 0.8 })
                .when(
                    Thirsty,
                    Drink {
                        until: 70.0,
                        per_tick: 10.0,
                    },
                )
                .when(HighMorale, ProcessOrder),
        ))
        .id();

    // Insert state change event
    let new_obj_event = MapEventType::NewObjEvent { new_player: false };
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

fn is_blocking_state(state_str: &str) -> bool {
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
