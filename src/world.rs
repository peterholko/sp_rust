use crate::{
    constants::{DAWN, DUSK, EVENING, GAME_TICKS_PER_DAY, MORNING, NIGHT}, event::{MapEvents, VisibleEvent}, game::{GameTick, Id, Position, Viewshed}, map::Map, network::MapWeather, obj
};
use bevy::prelude::*;
use rand::Rng;

#[derive(Debug, Clone)]
pub enum Weather {
    ClearSunny,
    HeavyRain,
    Thunderstorm,
    Moonsoon,
    Hurricane,
    Fog,
    ColdSnap,
    Snow,
    Blizzard,
    PolarVortex,
    Hail,
    Heatwave,
    Drought,
    Duststorm,
    SuperTyphoon,
    FlashFlood,
    IceStorm,
    FireStorm,
    Tornado,
    LightningSuperstorm,
}

impl Weather {
    pub fn to_string(&self) -> String {
        let str = match self {
            Weather::ClearSunny => "Clear and Sunny",
            Weather::HeavyRain => "Heavy Rain",
            Weather::Thunderstorm => "Thunderstorm",
            Weather::Moonsoon => "Monsoon",
            Weather::Hurricane => "Hurricane",
            Weather::Fog => "Fog",
            Weather::ColdSnap => "Cold Snap",
            Weather::Snow => "Snow",
            Weather::Blizzard => "Blizzard",
            Weather::PolarVortex => "Polar Vortex",
            Weather::Hail => "Hail",
            Weather::Heatwave => "Heatwave",
            Weather::Drought => "Drought",
            Weather::Duststorm => "Duststorm",
            Weather::SuperTyphoon => "Super Typhoon",
            Weather::FlashFlood => "Flash Flood",
            Weather::IceStorm => "Ice Storm",
            Weather::FireStorm => "Fire Storm",
            Weather::Tornado => "Tornado",
            Weather::LightningSuperstorm => "Lightning Superstorm",
        };

        return str.to_string();
    }

}

#[derive(Debug, Clone)]
pub struct WeatherArea {
    pub center: (i32, i32),
    pub weather: Weather,
    pub area: Vec<(i32, i32)>,
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct WeatherAreas(Vec<WeatherArea>);

impl WeatherAreas {
    pub fn get_visible_weather_tiles(&self, visible_pos: &Vec<(i32, i32)>) -> Vec<MapWeather> {
        // Get visible weather tiles from weather areas
        let mut visible_weather_tiles = Vec::new();

        for weather_area in self.iter() {
            for pos in visible_pos {
                if weather_area.area.contains(&pos) {

                    let map_weather = MapWeather {
                        x: pos.0,
                        y: pos.1,
                        weather: weather_area.weather.to_string(),
                    };

                    visible_weather_tiles.push(map_weather);
                }
            }
        }

        return visible_weather_tiles;
    }
}

pub fn create_weather_area(center_x: i32, center_y: i32, weather: Weather) -> WeatherArea {
    let radius = rand::thread_rng().gen_range(3..5);
    let area = Map::range((center_x, center_y), radius as u32);

    let weather_area = WeatherArea {
        center: (center_x, center_y),
        weather: weather,
        area: area,
    };

    return weather_area;
}

fn day_system(
    game_tick: ResMut<GameTick>,
    mut viewshed_query: Query<(&Id, &mut Viewshed)>,
    mut map_events: ResMut<MapEvents>,
) {
    let remainder = game_tick.0 % GAME_TICKS_PER_DAY;

    if remainder == DAWN
        || remainder == MORNING
        || remainder == DUSK
        || remainder == EVENING
        || remainder == NIGHT
    {
        for (id, mut viewshed) in viewshed_query.iter_mut() {
            if viewshed.range > 0 {
                let new_range = match game_tick.0 {
                    DAWN => 3,
                    MORNING => 4,
                    EVENING => 3,
                    DUSK => 2,
                    NIGHT => 1,
                    _ => 1,
                };

                info!("Updating viewshed range to: {:?}", new_range);

                viewshed.range = new_range;

                //Add obj update event
                let obj_update_event = VisibleEvent::UpdateObjEvent {
                    attr: obj::VISION.to_string(),
                    value: new_range.to_string(),
                };

                map_events.new(id.0, game_tick.0, obj_update_event);
            }
        }
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        //let weather_area = create_weather_area(16, 36, Weather::HeavyRain);
        //let weather_areas = WeatherAreas(vec![weather_area]);
        let weather_areas = WeatherAreas(Vec::new());

        app.insert_resource(weather_areas);

        app.add_systems(Update, day_system);
    }
}
