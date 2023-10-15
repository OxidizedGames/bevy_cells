use aery::Aery;
use bevy::{
    prelude::{
        App, AssetServer, Camera2dBundle, Commands, Component, Handle, Image, Res, Resource,
        Startup, Transform, Update, Vec3,
    },
    sprite::SpriteBundle,
    DefaultPlugins,
};
use bevy_cells::{
    tiles::{CellMapLabel, CellQuery},
    CellCommandExt,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Aery)
        .add_systems(Startup, spawn)
        .add_systems(Update, move_character)
        .run();
}

#[derive(Component)]
struct Block;

#[derive(Component)]
struct Character;

struct GameLayer;

impl CellMapLabel for GameLayer {
    const CHUNK_SIZE: usize = 16;
}

#[derive(Resource)]
struct ExampleAssets {
    block: Handle<Image>,
    character: Handle<Image>,
}

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    let assets = ExampleAssets {
        block: asset_server.load("block.png"),
        character: asset_server.load("character.png"),
    };

    commands.spawn(Camera2dBundle::default());

    let sprite_bundle = SpriteBundle {
        texture: assets.block,
        ..Default::default()
    };

    // spawn a 10 * 10 room
    for x in -5..=5 {
        commands.spawn_cell::<GameLayer, _>(
            [x, 5],
            (
                Block,
                SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(16.0 * x as f32, 80.0, 0.0)),
                    ..sprite_bundle.clone()
                },
            ),
        );
        commands.spawn_cell::<GameLayer, _>(
            [x, -5],
            (
                Block,
                SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(16.0 * x as f32, -80.0, 0.0)),
                    ..sprite_bundle.clone()
                },
            ),
        );
    }

    for y in -4..=4 {
        commands.spawn_cell::<GameLayer, _>(
            [5, y],
            (
                Block,
                SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(80.0, 16.0 * y as f32, 0.0)),
                    ..sprite_bundle.clone()
                },
            ),
        );
        commands.spawn_cell::<GameLayer, _>(
            [-5, y],
            (
                Block,
                SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(-80.0, 16.0 * y as f32, 0.0)),
                    ..sprite_bundle.clone()
                },
            ),
        );
    }

    // spawn a player
    commands.spawn_cell::<GameLayer, _>(
        [0, 0],
        (
            Character,
            SpriteBundle {
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                texture: assets.character,
                ..Default::default()
            },
        ),
    );
}

fn move_character(
    character: CellQuery<GameLayer, &Character>,
    wall_query: CellQuery<GameLayer, &Block>,
) {
    character.get([0, 0]).unwrap();
}
