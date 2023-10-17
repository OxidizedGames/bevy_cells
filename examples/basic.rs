use aery::Aery;
use bevy::{
    prelude::{
        info, App, AssetServer, Camera2dBundle, Commands, Component, Handle, Image, Input, KeyCode,
        PostUpdate, Res, Resource, Startup, Transform, Update, Vec3, With,
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
        .add_systems(PostUpdate, sync_cell_transforms)
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
        commands.spawn_cell::<GameLayer, _>([x, 5], (Block, sprite_bundle.clone()));
        commands.spawn_cell::<GameLayer, _>([x, -5], (Block, sprite_bundle.clone()));
    }

    for y in -4..=4 {
        commands.spawn_cell::<GameLayer, _>([5, y], (Block, sprite_bundle.clone()));
        commands.spawn_cell::<GameLayer, _>([-5, y], (Block, sprite_bundle.clone()));
    }

    // spawn a player
    commands.spawn_cell::<GameLayer, _>(
        [0, 0],
        (
            Character,
            SpriteBundle {
                texture: assets.character,
                ..Default::default()
            },
        ),
    );
}

fn move_character(
    keyboard_input: Res<Input<KeyCode>>,
    mut commands: Commands,
    character: CellQuery<GameLayer, (), With<Character>>,
    walls: CellQuery<GameLayer, (), With<Block>>,
) {
    let (char_c, _) = character.get_single_with_coord().unwrap();

    let mut x = if keyboard_input.just_pressed(KeyCode::A) {
        -1
    } else {
        0
    };
    x += if keyboard_input.just_pressed(KeyCode::D) {
        1
    } else {
        0
    };

    let mut y = if keyboard_input.just_pressed(KeyCode::W) {
        1
    } else {
        0
    };
    y -= if keyboard_input.just_pressed(KeyCode::S) {
        1
    } else {
        0
    };

    let new_coord = [char_c[0] + x, char_c[1] + y];

    if walls.get([char_c[0] + x, char_c[1] + y]).is_none() {
        commands.move_cell::<GameLayer>(char_c, new_coord);
    }
}

fn sync_cell_transforms(mut cells: CellQuery<GameLayer, &mut Transform>) {
    for (cell_c, mut transform) in cells.iter_mut_with_coord() {
        transform.translation.x = cell_c[0] as f32 * 16.0;
        transform.translation.y = cell_c[1] as f32 * 16.0;
    }
}
