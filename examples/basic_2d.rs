use aery::Aery;
use bevy::{prelude::*, sprite::SpriteBundle, DefaultPlugins};
use bevy_cells::{cells::CellCoord, prelude::*};

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

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    let block = asset_server.load("block.png");
    let character = asset_server.load("character.png");

    commands.spawn(Camera2dBundle::default());
    let mut cell_commands = commands.cells::<GameLayer, 2>();

    let sprite_bundle = SpriteBundle {
        texture: block,
        ..Default::default()
    };

    // spawn a 10 * 10 room
    for x in -5..=5 {
        cell_commands.spawn_cell([x, 5], (Block, sprite_bundle.clone()));
        cell_commands.spawn_cell([x, -5], (Block, sprite_bundle.clone()));
    }

    for y in -4..=4 {
        cell_commands.spawn_cell([5, y], (Block, sprite_bundle.clone()));
        cell_commands.spawn_cell([-5, y], (Block, sprite_bundle.clone()));
    }

    // spawn a player
    cell_commands.spawn_cell(
        [0, 0],
        (
            Character,
            SpriteBundle {
                texture: character,
                ..Default::default()
            },
        ),
    );
}

fn move_character(
    keyboard_input: Res<Input<KeyCode>>,
    mut commands: Commands,
    character: CellQuery<GameLayer, &CellCoord, With<Character>>,
    walls: CellQuery<GameLayer, (), With<Block>>,
) {
    let mut cell_commands = commands.cells::<GameLayer, 2>();

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

    let char_c = character.get_single().unwrap();
    let new_coord = [char_c[0] + x, char_c[1] + y];

    if walls.get_at(new_coord).is_none() {
        cell_commands.move_cell(**char_c, new_coord);
    }
}

fn sync_cell_transforms(
    mut cells: CellQuery<GameLayer, (&CellCoord, &mut Transform), Changed<CellCoord>>,
) {
    for (cell_c, mut transform) in cells.iter_mut() {
        transform.translation.x = cell_c[0] as f32 * 16.0;
        transform.translation.y = cell_c[1] as f32 * 16.0;
    }
}
