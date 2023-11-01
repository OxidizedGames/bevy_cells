use aery::Aery;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::Vec2Swizzles,
    prelude::*,
    render::camera::ScalingMode,
    sprite::SpriteBundle,
    window::PrimaryWindow,
    DefaultPlugins,
};
use bevy_cells::{cells::coords::world_to_cell, prelude::*};
use std::ops::{Deref, DerefMut};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Aery)
        .add_systems(Startup, spawn)
        .add_systems(Update, (add_damage, check_damage).chain())
        .add_systems(PostUpdate, sync_cell_transforms)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .run();
}

#[derive(Component)]
struct Block;

#[derive(Component)]
struct Damage(usize);

impl Deref for Damage {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Damage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

struct GameLayer;

impl CellMapLabel for GameLayer {
    const CHUNK_SIZE: usize = 16;
}

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    let block = asset_server.load("block.png");

    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scale: 1.0,
            ..Camera2dBundle::default().projection
        },
        ..Default::default()
    });
    let mut cell_commands = commands.cells::<GameLayer, 2>();

    let sprite_bundle = SpriteBundle {
        texture: block,
        ..Default::default()
    };

    // spawn a 10 * 10 group
    for x in -500..=500 {
        for y in -500..=500 {
            cell_commands.spawn_cell([x, y], (Block, sprite_bundle.clone()));
            cell_commands.spawn_cell([x, -y], (Block, sprite_bundle.clone()));
        }
    }
}

fn add_damage(
    mut commands: Commands,
    mut blocks: CellQuery<GameLayer, (Entity, Option<&mut Damage>), With<Block>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    buttons: Res<Input<MouseButton>>,
) {
    let (cam, cam_t) = camera.single();
    if let Some(damage_pos) = buttons
        .just_pressed(MouseButton::Left)
        .then(|| windows.single().cursor_position())
        .flatten()
        .and_then(|cursor| cam.viewport_to_world(cam_t, cursor.xy()))
        .map(|ray| ray.origin.truncate())
        .map(|pos| world_to_cell(pos.into(), 16.0))
    {
        let start = [damage_pos[0] - 2, damage_pos[1] - 2];
        let end = [damage_pos[0] + 2, damage_pos[1] + 2];
        for (block_id, damage) in blocks.iter_in_mut(start, end) {
            if let Some(mut damage) = damage {
                **damage += 1;
            } else {
                commands.entity(block_id).insert(Damage(1));
            }
        }
    }
}

fn check_damage(
    mut commands: Commands,
    mut damaged: Query<(Entity, &Damage, &mut Sprite), Changed<Damage>>,
) {
    for (id, damage, mut sprite) in damaged.iter_mut() {
        match **damage {
            x if x > 3 => commands.entity(id).despawn(),
            x => {
                let tint = 1.0 - x as f32 / 3.0;
                sprite.color = Color::Rgba {
                    red: 1.0,
                    green: tint,
                    blue: tint,
                    alpha: 1.0,
                }
            }
        }
    }
}

fn sync_cell_transforms(mut cells: CellQuery<GameLayer, (CellCoord, &mut Transform)>) {
    for (cell_c, mut transform) in cells.iter_mut() {
        transform.translation.x = cell_c[0] as f32 * 16.0;
        transform.translation.y = cell_c[1] as f32 * 16.0;
    }
}
