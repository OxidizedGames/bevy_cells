use bevy::{prelude::*, sprite::SpriteBundle, DefaultPlugins};
use bevy_cells::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CellsPlugin)
        .add_systems(Startup, spawn)
        .add_systems(Update, sync_cell_transforms)
        .run();
}

#[derive(Component)]
struct Block;

struct GameLayer;

impl CellMapLabel for GameLayer {
    const CHUNK_SIZE: usize = 16;
}

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    let block = asset_server.load("block.png");

    commands.spawn(Camera2dBundle {
        transform: Transform::from_translation(Vec3::new(480.0, 32.0, 0.0)),
        ..Default::default()
    });
    let mut cell_commands = commands.cells::<GameLayer, 2>();

    let sprite_bundle = SpriteBundle {
        texture: block,
        ..Default::default()
    };

    let logo = r#"
eeeee  eeee e    e e    e       eeee eeee e     e     eeeee 
8   8  8    8    8 8    8       8    8    8     8     8     
8eee8e 8eee 88  e8 8eeee8       8    8eee 8     8     8eeee 
8    8 8     8  8    88         8    8    8     8         8 
88eee8 88ee  8ee8    88   eeeee 88e8 88ee 88eee 88eee 8ee88 "#;

    let logo = logo.split('\n').enumerate().flat_map(|(y, line)| {
        line.bytes().enumerate().filter_map(move |(x, byte)| {
            if byte == 56 || byte == 101 {
                Some([x as isize, 6 - y as isize])
            } else {
                None
            }
        })
    });

    // spawn a 10 * 10 room
    cell_commands.spawn_cell_batch(logo.collect::<Vec<[isize; 2]>>(), move |_| {
        (Block, sprite_bundle.clone())
    });
}

fn sync_cell_transforms(
    mut cells: CellQuery<GameLayer, (&CellCoord, &mut Transform), Changed<CellCoord>>,
) {
    for (cell_c, mut transform) in cells.iter_mut() {
        transform.translation.x = cell_c[0] as f32 * 16.0;
        transform.translation.y = cell_c[1] as f32 * 16.0;
    }
}
