use bevy::prelude::{Res, Resource};
use crossbeam_channel::Sender as CBSender;
use serde_with::skip_serializing_none;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use lazy_static::lazy_static;

use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::{Message, Result};
use tokio_tungstenite::{accept_async, tungstenite::Error};

use serde::{Deserialize, Serialize};

use crate::{
    account::{Account, Accounts}, game::{MapObjQueryItem, ObjQueryMutReadOnlyItem}, item, obj::Obj, resource::Property, templates::ResReq
};
use crate::{
    game::{Client, Clients, HeroClassList},
    player::PlayerEvent,
};
use crate::{map::MapTile, recipe, templates::RecipeTemplate};

use std::path::Path;

use glob::glob;

//pub struct Network; // Is this needed?

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "cmd")]
enum NetworkPacket {
    #[serde(rename = "login")]
    Login { username: String, password: String },
    #[serde(rename = "select_class")]
    SelectedClass { classname: String },
    #[serde(rename = "get_stats")]
    GetStats { id: i32 },
    #[serde(rename = "image_def")]
    ImageDef { name: String },
    #[serde(rename = "move_unit")]
    Move { x: i32, y: i32 },
    #[serde(rename = "attack")]
    Attack {
        attacktype: String,
        sourceid: i32,
        targetid: i32,
    },
    #[serde(rename = "combo")]
    Combo { sourceid: i32, targetid: i32, combotype: String },
    #[serde(rename = "info_obj")]
    InfoObj { id: i32 },
    #[serde(rename = "info_skills")]
    InfoSkills { id: i32 },
    #[serde(rename = "info_attrs")]
    InfoAttrs { id: i32 },
    #[serde(rename = "info_advance")]
    InfoAdvance { sourceid: i32 },
    #[serde(rename = "info_upgrade")]
    InfoUpgrade { structureid: i32 },    
    #[serde(rename = "info_tile")]
    InfoTile { x: i32, y: i32 },
    #[serde(rename = "info_tile_resources")]
    InfoTileResources { x: i32, y: i32 },
    #[serde(rename = "info_inventory")]
    InfoInventory { id: i32 },
    #[serde(rename = "info_item")]
    InfoItem { id: i32, merchantid: i32, merchantaction: String},
    #[serde(rename = "info_item_by_name")]
    InfoItemByName { name: String },
    #[serde(rename = "info_item_transfer")]
    InfoItemTransfer { sourceid: i32, targetid: i32 },
    #[serde(rename = "info_exit")]
    InfoExit { id: i32, paneltype: String },
    #[serde(rename = "info_hire")]
    InfoHire { sourceid: i32 },
    #[serde(rename = "item_transfer")]
    ItemTransfer { targetid: i32, item: i32 },
    #[serde(rename = "item_split")]
    ItemSplit { item: i32, quantity: i32 },
    #[serde(rename = "gather")]
    Gather { sourceid: i32, restype: String },
    #[serde(rename = "refine")]
    Refine {},
    #[serde(rename = "craft")]
    Craft {recipe: String},   
    #[serde(rename = "order_follow")]
    OrderFollow { sourceid: i32 },
    #[serde(rename = "order_gather")]
    OrderGather { sourceid: i32, restype: String },
    #[serde(rename = "order_refine")]
    OrderRefine { structureid: i32 },
    #[serde(rename = "order_craft")]
    OrderCraft { sourceid: i32, recipe: String },
    #[serde(rename = "order_explore")]
    OrderExplore { sourceid: i32 },
    #[serde(rename = "order_experiment")]
    OrderExperiment { structureid: i32 },
    #[serde(rename = "structure_list")]
    StructureList {},
    #[serde(rename = "create_foundation")]
    CreateFoundation { sourceid: i32, structure: String },
    #[serde(rename = "build")]
    Build { sourceid: i32, structureid: i32 },
    #[serde(rename = "upgrade")]
    Upgrade { sourceid: i32, structureid: i32, selected_upgrade: String},
    #[serde(rename = "survey")]
    Survey { sourceid: i32 },
    #[serde(rename = "explore")]
    Explore {},
    #[serde(rename = "nearby_resources")]
    NearbyResources {},
    #[serde(rename = "assign_list")]
    AssignList {},
    #[serde(rename = "assign")]
    Assign { sourceid: i32, targetid: i32 },
    #[serde(rename = "equip")]
    Equip { item: i32, status: bool },
    #[serde(rename = "recipe_list")]
    RecipeList { structureid: i32 },
    #[serde(rename = "use")]
    Use { item: i32 },
    #[serde(rename = "delete")]
    Remove { sourceid: i32 },
    #[serde(rename = "advance")]
    Advance { sourceid: i32 },
    #[serde(rename = "info_experiment")]
    InfoExperiment { structureid: i32 },
    #[serde(rename = "set_exp_item")]
    SetExperimentItem { itemid: i32 },
    #[serde(rename = "set_exp_resource")]
    SetExperimentResource { itemid: i32 },
    #[serde(rename = "reset_experiment")]
    ResetExperiment { structureid: i32 },
    #[serde(rename = "hire")]
    Hire {sourceid: i32, targetid: i32},
    #[serde(rename = "buy_item")]
    BuyItem {itemid: i32, quantity: i32},
    #[serde(rename = "sell_item")]
    SellItem {itemid: i32, targetid: i32, quantity: i32}
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct StructureList {
    pub result: Vec<Structure>,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "packet")]
pub enum ResponsePacket {
    #[serde(rename = "select_class")]
    SelectClass {
        player: u32,
    },
    #[serde(rename = "info_select_class")]
    InfoSelectClass {
        result: String,
    },
    #[serde(rename = "login")]
    Login {
        player: u32,
    },
    #[serde(rename = "obj_perception")]
    ObjPerception {
        new_objs: Vec<MapObj>,
        new_tiles: Vec<MapTile>,
    },
    #[serde(rename = "stats")]
    Stats {
        data: StatsData,
    },
    #[serde(rename = "info_unit")]
    InfoObj {
        id: i32,
        name: String,
        class: String,
        subclass: String,
        template: String,
        state: String,
        image: String,
        hsl: Vec<i32>,
        items: Option<Vec<Item>>,
        skills: Option<HashMap<String, i32>>,
        attributes: Option<HashMap<String, i32>>,
        hp: Option<i32>,
        base_hp: Option<i32>,
        base_def: Option<i32>,
        base_vision: Option<i32>,
        base_speed: Option<i32>,
        base_stamina: Option<i32>,
        base_dmg: Option<i32>,
        dmg_range: Option<i32>,
        stamina: Option<i32>,
        structure: Option<String>,
        action: Option<String>,
        shelter: Option<String>,
        morale: Option<String>,
        order: Option<String>,
        capacity: Option<i32>,
        total_weight: Option<i32>,
        effects: Option<Vec<String>>,
        build_time: Option<i32>,
    },
    #[serde(rename = "info_hero")]
    InfoHero {
        id: i32,
        name: String,
        class: String,
        subclass: String,
        template: String,
        state: String,
        image: String,
        hsl: Vec<i32>,
        items: Option<Vec<Item>>,
        skills: Option<HashMap<String, i32>>,
        attributes: Option<HashMap<String, i32>>,
        effects: Option<Vec<String>>,
        hp: Option<i32>,
        stamina: Option<i32>,
        base_hp: Option<i32>,
        base_stamina: Option<i32>,
        base_def: Option<i32>,
        base_vision: Option<u32>,
        base_speed: Option<i32>,
        base_dmg: Option<i32>,
        dmg_range: Option<i32>,
    },
    #[serde(rename = "info_villager")]
    InfoVillager {
        id: i32,
        name: String,
        class: String,
        subclass: String,
        template: String,
        state: String,
        image: String,
        hsl: Vec<i32>,
        items: Option<Vec<Item>>,
        skills: Option<HashMap<String, i32>>,
        attributes: Option<HashMap<String, i32>>,
        effects: Option<Vec<String>>,
        hp: Option<i32>,
        stamina: Option<i32>,
        base_hp: Option<i32>,
        base_stamina: Option<i32>,
        base_def: Option<i32>,
        base_vision: Option<u32>,
        base_speed: Option<i32>,
        base_dmg: Option<i32>,
        dmg_range: Option<i32>,
        structure: Option<String>,
        action: Option<String>,
        shelter: Option<String>,
        morale: Option<String>,
        order: Option<String>,
        capacity: Option<i32>,
        total_weight: Option<i32>,
    },
    #[serde(rename = "info_structure")]
    InfoStructure {
        id: i32,
        name: String,
        class: String,
        subclass: String,
        template: String,
        state: String,
        image: String,
        hsl: Vec<i32>,
        items: Option<Vec<Item>>,
        hp: Option<i32>,
        base_hp: Option<i32>,
        base_def: Option<i32>,
        capacity: Option<i32>,
        total_weight: Option<i32>,
        effects: Option<Vec<String>>,
        build_time: Option<i32>,
        progress: Option<i32>,
        upgrade_req: Option<Vec<ResReq>>
    },
    #[serde(rename = "info_npc")]
    InfoNPC {
        id: i32,
        name: String,
        class: String,
        subclass: String,
        template: String,
        state: String,
        image: String,
        hsl: Vec<i32>,
        items: Option<Vec<Item>>,
        effects: Vec<String>,
    },
    #[serde(rename = "info_skills")]
    InfoSkills {
        id: i32,
        skills: HashMap<String, Skill>,
    },
    #[serde(rename = "info_attrs")]
    InfoAttrs {
        id: i32,
        attrs: HashMap<String, i32>,
    },
    #[serde(rename = "info_advance")]
    InfoAdvance {
        id: i32,
        rank: String,
        next_rank: String,
        total_xp: i32,
        req_xp: i32,
    },
    #[serde(rename = "info_upgrade")]
    InfoUpgrade {
        id: i32,        
        upgrade_list: Vec<UpgradeTemplate>,
        req: Vec<ResReq>
    },
    #[serde(rename = "info_tile")]
    InfoTile {
        x: i32,
        y: i32,
        name: String,
        mc: i32,
        def: f32,
        unrevealed: i32,
        sanctuary: String,
        passable: bool,
        wildness: String,
        resources: Vec<TileResource>,
        terrain_features: Vec<TileTerrainFeature>
    },
    #[serde(rename = "info_tile_resources")]
    InfoTileResources {
        x: i32,
        y: i32,
        name: String,
        resources: Vec<TileResource>,
    },    
    #[serde(rename = "info_inventory")]
    InfoInventory {
        id: i32,
        cap: i32,
        tw: i32,
        items: Vec<Item>,
    },
    #[serde(rename = "info_item")]
    InfoItem {
        id: i32,
        owner: i32,
        name: String,
        quantity: i32,
        class: String,
        subclass: String,
        image: String,
        weight: f32,
        equipped: bool,
        price: Option<i32>,
        attrs: Option<HashMap<item::AttrKey, item::AttrVal>>
    },
    #[serde(rename = "info_item_transfer")]
    InfoItemTransfer {
        sourceid: i32,
        sourceitems: Inventory,
        targetid: i32,
        targetitems: Inventory,
        reqitems: Vec<ResReq>,
    },
    #[serde(rename = "info_items_update")]
    InfoItemsUpdate {
        id: i32,
        items_updated: Vec<Item>,
        items_removed: Vec<i32>,
    },
    #[serde(rename = "info_hire")]
    InfoHire {
        data: Vec<HireData>
    },
    #[serde(rename = "item_transfer")]
    ItemTransfer {
        result: String,
        sourceid: i32,
        sourceitems: Inventory,
        targetid: i32,
        targetitems: Inventory,
        reqitems: Vec<ResReq>,
    },
    #[serde(rename = "item_split")]
    ItemSplit {
        result: String,
        owner: i32,
    },
    #[serde(rename = "info_experiment")]
    InfoExperiment {
        id: i32,
        expitem: Vec<Item>,
        expresources: Vec<Item>,
        validresources: Vec<Item>,
        expstate: String,
        recipe: Option<Recipe>,
    },
    #[serde(rename = "info_experiment_state")]
    InfoExperimentState {
        id: i32,
        expstate: String,
    },
    #[serde(rename = "nearby_resources")]
    NearbyResources {
        data: Vec<TileResourceWithPos>,
    },
    #[serde(rename = "structure_list")]
    StructureList(StructureList),
    #[serde(rename = "image_def")]
    ImageDef {
        name: String,
        data: serde_json::Value,
    },
    PlayerMoved {
        player_id: i32,
        x: i32,
        y: i32,
    },
    #[serde(rename = "perception")]
    Perception {
        data: PerceptionData,
    },
    #[serde(rename = "changes")]
    Changes {
        events: Vec<ChangeEvents>,
    },
    #[serde(rename = "create_foundation")]
    CreateFoundation {
        result: String,
    },
    #[serde(rename = "build")]
    Build {
        build_time: i32,
    },
    #[serde(rename = "upgrade")]
    Upgrade {
        upgrade_time: i32,
    },
    #[serde(rename = "explore")]
    Explore {
        explore_time: i32,
    },    
    #[serde(rename = "gather")]
    Gather {
        gather_time: i32,
    },      
    #[serde(rename = "attack")]
    Attack {
        sourceid: i32,
        attacktype: String,
        cooldown: i32,
        stamina_cost: i32,
    },
    #[serde(rename = "assign_list")]
    AssignList {
        result: Vec<Assignment>,
    },
    #[serde(rename = "assign")]
    Assign {
        result: String,
    },
    #[serde(rename = "equip")]
    Equip {
        result: String,
    },
    #[serde(rename = "recipe_list")]
    RecipeList {
        result: Vec<Recipe>,
    },
    #[serde(rename = "xp")]
    Xp {
        id: i32,
        xp_type: String,
        xp: i32,
    },
    #[serde(rename = "new_items")]
    NewItems {
        action: String,
        sourceid: i32,
        item_name: String,
    },
    #[serde(rename = "buy_item")]
    BuyItem {
        sourceid: i32,
        sourceitems: Inventory,
        targetid: i32,
        targetitems: Inventory,
    },    
    #[serde(rename = "sell_item")]
    SellItem {
        sourceid: i32,
        sourceitems: Inventory,
        targetid: i32,
        targetitems: Inventory,
    },
    Ok,
    None,
    Pong,
    Error {
        errmsg: String,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct PerceptionData {
    pub map: Vec<MapTile>,
    pub objs: Vec<MapObj>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChangeEvents {
    ObjCreate {
        event: String,
        obj: MapObj,
    },
    ObjUpdate {
        event: String,
        obj_id: i32,
        attr: String,
        value: String,
    },
    ObjMove {
        event: String,
        obj: MapObj,
        src_x: i32,
        src_y: i32,
    },
    ObjDelete {
        event: String,
        obj_id: i32,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct StatsData {
    pub id: i32,
    pub hp: i32,
    pub base_hp: i32,
    pub stamina: i32,
    pub base_stamina: i32,
    pub effects: Vec<i32>,
}

#[skip_serializing_none]
#[derive(Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(tag = "packet")]
pub enum BroadcastEvents {
    #[serde(rename = "dmg")]
    Damage {
        sourceid: i32,
        targetid: i32,
        attacktype: String,
        dmg: i32,
        state: String,
        combo: Option<String>,
        countered: Option<String>,
    },
    #[serde(rename = "speech")] // TODO consider renaming
    SoundObjEvent { source: i32, text: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, Hash, PartialEq)]
pub struct MapObj {
    pub id: i32,
    pub player: i32,
    pub name: String,
    pub class: String,
    pub subclass: String,
    pub template: String,
    pub image: String,
    pub x: i32,
    pub y: i32,
    pub state: String,
    pub vision: u32,
    pub hsl: Vec<i32>,
    pub groups: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Inventory {
    pub id: i32,
    pub cap: i32,
    pub tw: i32,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub quantity: i32,
    pub owner: i32,
    pub class: String,
    pub subclass: String,
    pub slot: Option<String>,
    pub image: String,
    pub weight: f32,
    pub equipped: bool,
    pub attrs: Option<HashMap<item::AttrKey, item::AttrVal>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Structure {
    pub name: String,
    pub image: String,
    pub class: String,
    pub subclass: String,
    pub template: String,
    pub base_hp: i32,
    pub base_def: i32,
    pub build_time: i32,
    pub req: Vec<ResReq>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Assignment {
    pub id: i32,
    pub name: String,
    pub image: String,
    pub order: String,
    pub structure: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Recipe {
    pub name: String,
    pub image: String,
    pub structure: String,
    pub class: String,
    pub subclass: String,
    pub tier: Option<i32>,
    pub slot: Option<String>,
    pub damage: Option<i32>,
    pub speed: Option<f32>,
    pub armor: Option<i32>,
    pub stamina_req: Option<i32>,
    pub skill_req: Option<i32>,
    pub weight: i32,
    pub req: Vec<ResReq>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Skill {
    pub level: i32,
    pub xp: i32,
    pub next: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TileResource {
    pub name: String,
    pub color: i32,
    pub yield_label: String,
    pub quantity_label: String,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TileTerrainFeature {
    pub name: String,
    pub image: String,
    pub bonus: String
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TileResourceWithPos {
    pub name: String,
    pub color: i32,
    pub yield_label: String,
    pub quantity_label: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct HireData {
    pub id: i32,
    pub name: String,
    pub image: String,
    pub wage: i32,
    pub creativity: i32,
    pub dexterity: i32,
    pub endurance: i32,
    pub focus: i32,
    pub intellect: i32,
    pub spirit: i32,
    pub strength: i32,
    pub toughness: i32,
    pub skills: HashMap<String, i32>
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct UpgradeTemplate {
    pub name: String,
    pub template: String
}

pub fn send_to_client(player_id: i32, packet: ResponsePacket, clients: &Res<Clients>) {
    for (_client_id, client) in clients.lock().unwrap().iter() {
        if client.player_id == player_id {
            client
                .sender
                .try_send(serde_json::to_string(&packet).unwrap())
                .expect("Could not send message");
        }
    }
}

pub fn create_network_obj(
    obj: MapObjQueryItem<'_>
) -> MapObj {
    let network_obj = MapObj {
        id: obj.id.0,
        player: obj.player_id.0,
        x: obj.pos.x,
        y: obj.pos.y,
        name: obj.name.0.clone(),
        template: obj.template.0.clone(),
        class: obj.class.0.clone(),
        subclass: obj.subclass.0.clone(),
        state: Obj::state_to_str(obj.state.clone()),
        vision: obj.viewshed.range,
        image: obj.misc.image.clone(),
        hsl: obj.misc.hsl.clone(),
        groups: obj.misc.groups.clone(),
    };

    network_obj
}

pub fn network_obj(
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
) -> MapObj {
    let network_obj = MapObj {
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

pub fn map_obj(
    obj: ObjQueryMutReadOnlyItem<'_>
) -> MapObj {
    let network_obj = MapObj {
        id: obj.id.0,
        player: obj.player_id.0,
        x: obj.pos.x,
        y: obj.pos.y,
        name: obj.name.0.clone(),
        template: obj.template.0.clone(),
        class: obj.class.0.clone(),
        subclass: obj.subclass.0.clone(),
        state: Obj::state_to_str(obj.state.clone()),
        vision: obj.viewshed.range,
        image: obj.misc.image.clone(),
        hsl: obj.misc.hsl.clone(),
        groups: obj.misc.groups.clone(),
    };

    network_obj
}

lazy_static! {
    static ref TILESET: HashMap<String, serde_json::Value> = {
        println!("Loading tilesets");
        let mut tileset = HashMap::new();

        // Load tilesets
        for entry in glob("./tileset/*.json").expect("Failed to read glob pattern") {
          match entry {
              Ok(path) => {
                let path = Path::new(&path);
                println!("path: {:?}", path);
                let file_stem = path.file_stem();
                let data = std::fs::read_to_string(&path).expect("Unable to read file");
                let json: serde_json::Value = serde_json::from_str(&data).expect("JSON does not have correct format.");
                //tileset.insert(file_stem.unwrap().to_str().unwrap().to_string(), serde_json::to_string(&json).unwrap());
                let file_stem = file_stem.unwrap().to_str().unwrap().to_string();
                println!("File stem: {:?}", file_stem);
                tileset.insert(file_stem, json);
              },
              Err(e) => println!("{:?}", e),
          }
        }

        tileset
    };
}

pub async fn tokio_setup(
    client_to_game_sender: CBSender<PlayerEvent>,
    clients: Clients,
    accounts: Accounts,
) {
    // env_logger::init();

    let addr = "127.0.0.1:9002";
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        println!("Peer address: {}", peer);

        //Spawn a connection handler per client
        tokio::spawn(accept_connection(
            peer,
            stream,
            client_to_game_sender.clone(),
            clients.clone(),
            accounts.clone(),
        ));
    }

    println!("Finished");
}

async fn accept_connection(
    peer: SocketAddr,
    stream: TcpStream,
    client_to_game_sender: CBSender<PlayerEvent>,
    clients: Clients,
    accounts: Accounts,
) {
    if let Err(e) = handle_connection(peer, stream, client_to_game_sender, clients, accounts).await
    {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => {
                println!("Connection closed")
            }
            err => println!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(
    peer: SocketAddr,
    stream: TcpStream,
    client_to_game_sender: CBSender<PlayerEvent>,
    clients: Clients,
    accounts: Accounts,
) -> Result<()> {
    println!("New WebSocket connection: {}", peer);
    let ws_stream = accept_async(stream).await.expect("Failed to accept");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    //Create a tokio sync channel to for messages from the game to each client
    let (game_to_client_sender, mut game_to_client_receiver) = tokio::sync::mpsc::channel(100);

    //Get the number of clients for a client id
    let num_clients = clients.lock().unwrap().keys().len() as i32;

    //Client ID
    let client_id = num_clients + 1;

    let mut player_id = -1;

    //Store the incremented client id and the game to client sender in the clients hashmap
    clients.lock().unwrap().insert(
        client_id,
        Client {
            id: client_id,
            player_id: player_id,
            sender: game_to_client_sender,
        },
    );

    //This loop uses the tokio select! macro to receive messages from either the websocket receiver
    //or the game to client receiver
    loop {
        tokio::select! {
            //Receive messages from the websocket
            msg = ws_receiver.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        if msg.is_text() || msg.is_binary() {

                            println!("player_id: {:?}", player_id);

                            //Check if the player is authenticated
                            if player_id == -1 {
                                //Attempt to login
                                let res_packet: ResponsePacket = match serde_json::from_str(msg.to_text().unwrap()) {
                                    Ok(packet) => {
                                        match packet {
                                            NetworkPacket::Login{username, password} => {
                                                println!("{:?}", username);
                                                //Retrieve player id, note will be set if authenticated
                                                let (pid, res) = handle_login(username, password, accounts.clone(), client_to_game_sender.clone());

                                                //Set player_id
                                                player_id = pid;

                                                if let Some(client) = clients.lock().unwrap().get_mut(&client_id) {
                                                    (*client).player_id = player_id;
                                                }

                                                //Return packet
                                                res
                                            }
                                            _ => ResponsePacket::Error{errmsg: "Unknown packet".to_owned()}
                                        }
                                    },

                                    Err(_) => ResponsePacket::Error{errmsg: "Unknown packet".to_owned()}
                                };
                                println!("{:?}", res_packet);
                                //TODO send event to game
                                //client_to_game_sender.send(Message::text(res)).expect("Could not send message");

                                //Send response to client
                                let res = serde_json::to_string(&res_packet).unwrap();
                                ws_sender.send(Message::Text(res)).await?;
                            } else {
                                println!("Authenticated packet: {:?}", msg.to_text().unwrap());

                                let res_packet: ResponsePacket = match serde_json::from_str(msg.to_text().unwrap()) {
                                    Ok(packet) => {
                                        match packet {
                                            NetworkPacket::SelectedClass{classname} => {
                                                handle_selected_class(player_id, classname, accounts.clone(), client_to_game_sender.clone())
                                            }
                                            NetworkPacket::GetStats{id} => {
                                                handle_get_stats(player_id, id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::ImageDef{name} => {
                                                println!("ImageDef name: {:?}", name);
                                                let mut name_stripped = name.clone();
                                                let raw_name = name;

                                                if name_stripped.chars().last().unwrap().is_numeric() {
                                                    name_stripped.pop();
                                                }

                                                ResponsePacket::ImageDef{
                                                    name: raw_name,
                                                    data: TILESET.get(&name_stripped).unwrap().clone()
                                                }
                                            }
                                            NetworkPacket::Move{x, y} => {
                                                handle_move(player_id, x, y, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Attack{attacktype, sourceid, targetid} => {
                                                handle_attack(player_id, attacktype, sourceid, targetid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Combo{sourceid, targetid, combotype} => {
                                                handle_combo(player_id, sourceid, targetid, combotype, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoObj{id} => {
                                                handle_info_obj(player_id, id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoSkills{id} => {
                                                handle_info_skills(player_id, id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoAttrs{id} => {
                                                handle_info_attrs(player_id, id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoAdvance{sourceid} => {
                                                handle_info_advance(player_id, sourceid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoUpgrade{structureid} => {
                                                handle_info_upgrade(player_id, structureid, client_to_game_sender.clone())
                                            }                                            
                                            NetworkPacket::InfoTile{x, y} => {
                                                handle_info_tile(player_id, x, y, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoTileResources{x, y} => {
                                                handle_info_tile_resources(player_id, x, y, client_to_game_sender.clone())
                                            }                                            
                                            NetworkPacket::InfoInventory{id} => {
                                                handle_info_inventory(player_id, id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoItem{id, merchantid, merchantaction} => {
                                                handle_info_item(player_id, id, merchantid, merchantaction, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoItemByName{name} => {
                                                handle_info_item_by_name(player_id, name, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoItemTransfer{sourceid, targetid} => {
                                                handle_info_item_transfer(player_id, sourceid, targetid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoExit{id, paneltype} => {
                                                handle_info_exit(player_id, id, paneltype, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoHire{sourceid} => {
                                                handle_info_hire(player_id, sourceid, client_to_game_sender.clone())
                                            }                                            
                                            NetworkPacket::ItemTransfer{targetid, item} => {
                                                handle_item_transfer(player_id, targetid, item, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::ItemSplit{item, quantity} => {
                                                handle_item_split(player_id, item, quantity, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Gather{sourceid, restype} => {
                                                handle_gather(player_id, sourceid, restype, client_to_game_sender.clone())
                                            }                                            
                                            NetworkPacket::Refine{} => {
                                                handle_refine(player_id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Craft{recipe} => {
                                                handle_craft(player_id, recipe, client_to_game_sender.clone())
                                            }                                            
                                            NetworkPacket::OrderFollow{sourceid} => {
                                                handle_order_follow(player_id, sourceid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::OrderGather{sourceid, restype} => {
                                                handle_order_gather(player_id, sourceid, restype, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::StructureList{} => {
                                                handle_structure_list(player_id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::CreateFoundation{sourceid, structure} => {
                                                handle_create_foundation(player_id, sourceid, structure, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Build{sourceid, structureid} => {
                                                handle_build(player_id, sourceid, structureid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Upgrade{sourceid, structureid, selected_upgrade} => {
                                                handle_upgrade(player_id, sourceid, structureid, selected_upgrade, client_to_game_sender.clone())
                                            }                                            
                                            NetworkPacket::Survey{sourceid} => {
                                                handle_survey(player_id, sourceid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::NearbyResources{} => {
                                                handle_nearby_resources(player_id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Explore{} => {
                                                handle_explore(player_id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::AssignList{} => {
                                                handle_assign_list(player_id, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Assign{sourceid, targetid} => {
                                                handle_assign(player_id, sourceid, targetid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Equip{item, status} => {
                                                handle_equip(player_id, item, status, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::RecipeList{structureid} => {
                                                handle_recipe_list(player_id, structureid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::OrderRefine{structureid} => {
                                                handle_order_refine(player_id, structureid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::OrderCraft{sourceid, recipe} => {
                                                handle_order_craft(player_id, sourceid, recipe, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::OrderExplore{sourceid} => {
                                                handle_order_explore(player_id, sourceid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::OrderExperiment{structureid} => {
                                                handle_order_experiment(player_id, structureid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Use{item} => {
                                                handle_use(player_id, item, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Remove{sourceid} => {
                                                handle_remove(player_id, sourceid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Advance{sourceid} => {
                                                handle_advance(player_id, sourceid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::InfoExperiment{structureid} => {
                                                handle_info_experiment(player_id, structureid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::SetExperimentItem{itemid} => {
                                                //Setting experiment source item, is_resource = false
                                                handle_set_experiment_item(player_id, itemid, false, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::SetExperimentResource{itemid} => {
                                                //Setting experiment resource item, is_resource = true
                                                handle_set_experiment_item(player_id, itemid, true, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::ResetExperiment{structureid} => {
                                                handle_reset_experiment(player_id, structureid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::Hire{sourceid, targetid} => {
                                                handle_hire(player_id, sourceid, targetid, client_to_game_sender.clone())
                                            }
                                            NetworkPacket::BuyItem{itemid, quantity} => {
                                                handle_buy_item(player_id, itemid, quantity, client_to_game_sender.clone())
                                            }                                                 
                                            NetworkPacket::SellItem{itemid, targetid, quantity} => {
                                                handle_sell_item(player_id, itemid, targetid, quantity, client_to_game_sender.clone())
                                            }                                            
                                            _ => ResponsePacket::Ok
                                        }
                                    },
                                    Err(packet) => {
                                        let ping = r#"0"#;

                                        if msg.to_text().unwrap() == ping {
                                            ResponsePacket::Pong
                                        } else {
                                            println!("Error packet: {:?}", packet);
                                            ResponsePacket::Error{errmsg: "Unknown packet".to_owned()}
                                        }
                                    }
                                };
                                if res_packet == ResponsePacket::Pong {
                                    ws_sender.send(Message::Text("1".to_string())).await?;
                                }
                                else if res_packet != ResponsePacket::None {
                                    let res = serde_json::to_string(&res_packet).unwrap();
                                    ws_sender.send(Message::Text(res)).await?;
                                }
                            }
                        } else if msg.is_close() {
                            println!("Message is closed for player: {:?}", player_id);
                            handle_disconnect(client_id, clients.clone());
                            break;
                        }
                    }
                    None => break,
                }
            }
            //Receive messages from the game
            game_msg = game_to_client_receiver.recv() => {
                let game_msg = game_msg.unwrap();
                ws_sender.send(Message::Text(game_msg)).await?;
            }
        }
    }
    Ok(())
}

fn handle_login(
    username: String,
    password: String,
    accounts: Accounts,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> (i32, ResponsePacket) {
    println!("handle_login: {:?}", accounts);

    let mut found_account = false;
    let mut password_match = false;
    let mut player_id: i32 = -1;
    let mut account_class = HeroClassList::None;

    let mut accounts = accounts.lock().unwrap();

    for (_, account) in accounts.iter() {
        if account.username == username {
            found_account = true;

            let verify_password =
                Account::verify_password(password.clone(), account.password.clone());

            match verify_password {
                Ok(_) => {
                    password_match = true;
                    player_id = account.player_id;
                    account_class = account.class;
                }
                Err(_) => {
                    break;
                }
            }
        }
    }

    println!("found_account: {:?}", found_account);

    let ret = if found_account && password_match {
        println!("Found account and password matched: {:?}", account_class);
        if account_class == HeroClassList::None {
            (
                player_id,
                ResponsePacket::SelectClass {
                    player: player_id as u32,
                },
            )
        } else {
            //Send login to player
            client_to_game_sender
                .send(PlayerEvent::Login {
                    player_id: player_id,
                })
                .expect("Could not send message");

            (
                player_id,
                ResponsePacket::Login {
                    player: player_id as u32,
                },
            )
        }
    } else if found_account && !password_match {
        println!("Found account and password incorrect.");
        (
            player_id,
            ResponsePacket::Error {
                errmsg: "Incorrect password".to_owned(),
            },
        )
    } else {
        println!("Account not found, creating new account...");
        let player_id = (accounts.len() + 1) as i32;
        let account = Account::new(player_id, username, password);

        accounts.insert(player_id, account);

        (
            player_id,
            ResponsePacket::SelectClass {
                player: player_id as u32,
            },
        )
    };

    ret
}

fn handle_disconnect(client_id: i32, clients: Clients) {
    let mut clients = clients.lock().unwrap();

    clients.remove(&client_id);
}

fn handle_selected_class(
    player_id: i32,
    classname: String,
    accounts: Accounts,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    println!("handle_selected_class: {:?}", player_id);
    let mut accounts = accounts.lock().unwrap();
    println!("{:?}", accounts);
    let mut account = accounts.get_mut(&player_id).unwrap();

    if account.class == HeroClassList::None {
        println!("classname: {:?}", classname.as_str());
        let selected_class = match classname.as_str() {
            "Warrior" => HeroClassList::Warrior,
            "Ranger" => HeroClassList::Ranger,
            "Mage" => HeroClassList::Mage,
            _ => HeroClassList::None,
        };

        account.class = selected_class;
        println!("Selected Class - account_class: {:?}", account.class);

        //Send new player event to game
        client_to_game_sender
            .send(PlayerEvent::NewPlayer {
                player_id: player_id,
            })
            .expect("Could not send message");

        ResponsePacket::InfoSelectClass {
            result: "success".to_owned(),
        }
    } else {
        ResponsePacket::Error {
            errmsg: "Hero class already selected.".to_owned(),
        }
    }
}

fn handle_get_stats(
    player_id: i32,
    id: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {    
    client_to_game_sender
        .send(PlayerEvent::GetStats {
            player_id: player_id,
            id: id
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_move(
    player_id: i32,
    x: i32,
    y: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Move {
            player_id: player_id,
            x: x,
            y: y,
        })
        .expect("Could not send message");

    ResponsePacket::Ok
}

fn handle_attack(
    player_id: i32,
    attacktype: String,
    sourceid: i32,
    targetid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Attack {
            player_id: player_id,
            attack_type: attacktype,
            source_id: sourceid,
            target_id: targetid,
        })
        .expect("Could not send message");

    ResponsePacket::None
}
fn handle_combo(
    player_id: i32,
    sourceid: i32,
    targetid: i32,
    combotype: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Combo {
            player_id: player_id,
            source_id: sourceid,
            target_id: targetid,
            combo_type: combotype,
        })
        .expect("Could not send message");

    ResponsePacket::Ok
}

fn handle_info_obj(
    player_id: i32,
    id: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoObj {
            player_id: player_id,
            id: id,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_skills(
    player_id: i32,
    id: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoSkills {
            player_id: player_id,
            id: id,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_attrs(
    player_id: i32,
    id: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoAttrs {
            player_id: player_id,
            id: id,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_advance(
    player_id: i32,
    sourceid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoAdvance {
            player_id: player_id,
            id: sourceid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_upgrade(
    player_id: i32,
    structureid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoUpgrade {
            player_id: player_id,
            structure_id: structureid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_tile(
    player_id: i32,
    x: i32,
    y: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoTile {
            player_id: player_id,
            x: x,
            y: y,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_tile_resources(
    player_id: i32,
    x: i32,
    y: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoTileResources {
            player_id: player_id,
            x: x,
            y: y,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_inventory(
    player_id: i32,
    id: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoInventory {
            player_id: player_id,
            id: id,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_item(
    player_id: i32,
    id: i32,
    merchantid: i32,
    merchantaction: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoItem {
            player_id: player_id,
            id: id,
            merchant_id: merchantid,
            merchant_action: merchantaction
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_item_by_name(
    player_id: i32,
    name: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoItemByName {
            player_id: player_id,
            name: name,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_item_transfer(
    player_id: i32,
    sourceid: i32,
    targetid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoItemTransfer {
            player_id: player_id,
            source_id: sourceid,
            target_id: targetid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_exit(
    player_id: i32,
    id: i32,
    paneltype: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoExit {
            player_id: player_id,
            id: id,
            panel_type: paneltype,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_info_hire(
    player_id: i32,
    sourceid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoHire {
            player_id: player_id,
            source_id: sourceid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_item_transfer(
    player_id: i32,
    targetid: i32,
    item: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::ItemTransfer {
            player_id: player_id,
            target_id: targetid,
            item_id: item,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_item_split(
    player_id: i32,
    item: i32,
    quantity: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::ItemSplit {
            player_id: player_id,
            item_id: item,
            quantity: quantity,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_gather(
    player_id: i32,
    sourceid: i32,
    restype: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Gather {
            player_id: player_id,
            source_id: sourceid,
            res_type: restype,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_refine(
    player_id: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Refine {
            player_id: player_id,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_craft(
    player_id: i32,
    recipe: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Craft {
            player_id: player_id,
            recipe_name: recipe
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_order_follow(
    player_id: i32,
    sourceid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::OrderFollow {
            player_id: player_id,
            source_id: sourceid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_order_gather(
    player_id: i32,
    sourceid: i32,
    restype: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::OrderGather {
            player_id: player_id,
            source_id: sourceid,
            res_type: restype,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_structure_list(
    player_id: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::StructureList {
            player_id: player_id,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_create_foundation(
    player_id: i32,
    sourceid: i32,
    structure: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::CreateFoundation {
            player_id: player_id,
            source_id: sourceid,
            structure_name: structure,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::None
}

fn handle_build(
    player_id: i32,
    sourceid: i32,
    structureid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Build {
            player_id: player_id,
            source_id: sourceid,
            structure_id: structureid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_upgrade(
    player_id: i32,
    sourceid: i32,
    structureid: i32,
    selected_upgrade: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Upgrade {
            player_id: player_id,
            source_id: sourceid,
            structure_id: structureid,
            selected_upgrade: selected_upgrade
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_survey(
    player_id: i32,
    sourceid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Survey {
            player_id: player_id,
            source_id: sourceid,
        })
        .expect("Could not send message");

    ResponsePacket::Ok
}

fn handle_explore(player_id: i32, client_to_game_sender: CBSender<PlayerEvent>) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Explore {
            player_id: player_id,
        })
        .expect("Could not send message");

    ResponsePacket::Ok
}

fn handle_nearby_resources(
    player_id: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::NearbyResources {
            player_id: player_id,
        })
        .expect("Could not send message");

    ResponsePacket::Ok
}

fn handle_assign_list(
    player_id: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::AssignList {
            player_id: player_id,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_assign(
    player_id: i32,
    sourceid: i32,
    targetid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Assign {
            player_id: player_id,
            source_id: sourceid,
            target_id: targetid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_equip(
    player_id: i32,
    item: i32,
    status: bool,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Equip {
            player_id: player_id,
            item_id: item,
            status: status,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_recipe_list(
    player_id: i32,
    structureid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::RecipeList {
            player_id: player_id,
            structure_id: structureid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_order_refine(
    player_id: i32,
    structureid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::OrderRefine {
            player_id: player_id,
            structure_id: structureid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_order_craft(
    player_id: i32,
    sourceid: i32,
    recipe: String,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::OrderCraft {
            player_id: player_id,
            structure_id: sourceid, // sourceid should really be renamed to structure_id in the client
            recipe_name: recipe,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_order_explore(
    player_id: i32,
    sourceid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::OrderExplore {
            player_id: player_id,
            villager_id: sourceid, // sourceid should really be renamed to structure_id in the client
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_order_experiment(
    player_id: i32,
    structureid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::OrderExperiment {
            player_id: player_id,
            structure_id: structureid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_use(
    player_id: i32,
    item: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Use {
            player_id: player_id,
            item_id: item, // sourceid should really be renamed to structure_id in the client
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_remove(
    player_id: i32,
    sourceid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Remove {
            player_id: player_id,
            structure_id: sourceid, // sourceid should really be renamed to structure_id in the client
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_advance(
    player_id: i32,
    sourceid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Advance {
            player_id: player_id,
            id: sourceid, // sourceid should really be renamed to structure_id in the client
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_info_experiment(
    player_id: i32,
    structureid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::InfoExperinment {
            player_id: player_id,
            structure_id: structureid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_set_experiment_item(
    player_id: i32,
    itemid: i32,
    is_resource: bool,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::SetExperimentItem {
            player_id: player_id,
            item_id: itemid,
            is_resource: is_resource,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_reset_experiment(
    player_id: i32,
    structureid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::ResetExperiment {
            player_id: player_id,
            structure_id: structureid,
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_hire(
    player_id: i32,
    sourceid: i32,
    targetid: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::Hire {
            player_id: player_id,
            merchant_id: sourceid,
            target_id: targetid
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_buy_item(
    player_id: i32,
    itemid: i32,
    quantity: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::BuyItem {
            player_id: player_id,
            item_id: itemid,
            quantity: quantity
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}

fn handle_sell_item(
    player_id: i32,
    itemid: i32,
    targetid: i32,
    quantity: i32,
    client_to_game_sender: CBSender<PlayerEvent>,
) -> ResponsePacket {
    client_to_game_sender
        .send(PlayerEvent::SellItem {
            player_id: player_id,
            item_id: itemid,
            target_id: targetid,
            quantity: quantity
        })
        .expect("Could not send message");

    // Response will come from game.rs
    ResponsePacket::Ok
}
