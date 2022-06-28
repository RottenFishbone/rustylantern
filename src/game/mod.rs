#![allow(unused_variables, unused_imports, dead_code)]
mod player;

use std::rc::Rc;
use std::collections::HashMap;
use glutin::event::ElementState;
use nalgebra_glm::{Vec2, Vec3, Vec4};

use specs::{Builder, World, WorldExt, Entity, RunNow, DispatcherBuilder, Dispatcher};
use stoneng::ecs::component::Scale;
use stoneng::ecs::{
    resource,
    system,
    component,
};
use stoneng::event::{KeyEvent, KeyCode};
use stoneng::{
    self, 
    model::spritesheet::SpriteSheet,
    event,
};

use crate::game::player::*;

pub struct RustyLantern<'a> {
    spritesheet:        SpriteSheet,
    world:              Option<World>,
    dispatcher:         Option<Dispatcher<'a, 'a>>,
    time:               std::time::Instant,

    cursor:             Option<Entity>,
    player:             Option<PlayerController>,
}

impl<'a> RustyLantern<'a> {
    pub fn new() -> Self {
        Self {
            spritesheet: SpriteSheet::from_layout("assets/textures/sprites.ron".into()).unwrap(),
            world: None,
            dispatcher: None,
            time: std::time::Instant::now(),

            cursor: None,
            player: None,
        }
    }
}


impl<'a> stoneng::EngineCore for RustyLantern<'a> {
    fn init(&mut self){
        // Setup ECS
        let mut world = World::new();
        world.insert(resource::DeltaTime(0.0));
        world.insert(resource::WindowSize(800.0, 600.0));

        let mut dispatcher = DispatcherBuilder::new()
            .with(system::movement::MovementSys, "move_sys", &[])
            .with(system::sprite::StaticSpriteSys, "static_sprite", &[])
            .with(system::sprite::AnimSpriteSys, "anim_sprite", &["static_sprite"])
            .with_thread_local(system::RenderSys::default())
            .with_thread_local(system::sprite::SpriteRenderSys::default())
            .with_thread_local(system::light::LightRenderSys::default())
            .with_thread_local(system::text::TextRenderSys::default())
            .build();
 
        dispatcher.setup(&mut world);
        let tile = self.spritesheet.sprites.get("human-unarmed").unwrap().clone();
        let mut xform = component::Transform::default(); 
        xform.translation.x = 32.0;
        xform.translation.y = 32.0;
        xform.scale = Scale { x: 6.0, y: 6.0 };
        world.create_entity()
            .with(xform.clone())
            .with(component::Color::default())
            .with(component::Sprite::from(tile.clone())) 
            .build();

        xform.translation.x = 100.0;
        xform.translation.y = 100.0;
        let player_anim = tile.animations.get("walk-side"); 
        let player_entity = world.create_entity()
                .with(xform)
                .with(component::Color::default())
                .with(component::Sprite::from(tile.clone()))
                .with(component::Animation::from(player_anim))
                .with(component::PointLight { intensity: 100.0 })
                .with(component::Velocity { x: 0.0, y: 0.0 })
                .build();

        self.player = Some(PlayerController::from(player_entity));


        self.cursor = Some(
            world.create_entity()
                .with(component::Transform::default())
                .with(component::PointLight { intensity: 300.0 })
                .with(component::Text{ 
                    content: String::from(">9000"), size: 2.0, offset: (0.0, 0.0) 
                })
                .with(component::Color::default())
                .build()
        );
        
        world.maintain();

        self.world = Some(world);
        self.dispatcher = Some(dispatcher);
    }

    fn tick(&mut self, dt: f64){
        if let Some(world) = &mut self.world {
            let mut dt_res = world.write_resource::<resource::DeltaTime>();
            *dt_res = resource::DeltaTime(dt);
        }
    }

    fn render(&mut self) {
        if let Some(dispatcher) = &mut self.dispatcher {
            if let Some(world) = &mut self.world {
                dispatcher.dispatch(world);
            }
        }              
    }
    fn post_render(&mut self) {
        if let Some(world) = &mut self.world {
            world.maintain();
        }
    }

    fn key_input(&mut self, event: event::KeyEvent){
        let world = match &self.world {
            Some(world) => world,
            None => return,
        };
        let player = match &mut self.player {
            Some(player) => player,
            None => return,
        };


        let state = match event.state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };

        if let Some(key) = event.key {
            let dir = match key {
                KeyCode::D => Some(MoveDir::Right),
                KeyCode::A => Some(MoveDir::Left),
                KeyCode::W => Some(MoveDir::Up),
                KeyCode::S => Some(MoveDir::Down),
                _ => None,
            };

            if let Some(dir) = dir {
                player.set_move(dir, state, world);
            }
        }
    }

    fn mouse_btn(&mut self, event: event::MouseBtnEvent){}

    fn cursor_moved(&mut self, x: f64, y: f64) {
        if let Some(world) = &self.world {
            if let Some(cursor) = &self.cursor {
                let win = world.read_resource::<resource::WindowSize>();

                let mut xforms = world.write_component::<component::Transform>();
                match xforms.get_mut(*cursor) {
                    Some(xform) => {
                        xform.translation.x = x as f32;
                        xform.translation.y = win.1 as f32 - y as f32;
                    },
                    None => return,
                }
            }
        }
    }

    fn resized(&mut self, x: u32, y: u32) {
        if let Some(world) = &self.world {
            let mut win = world.write_resource::<resource::WindowSize>();
            *win = resource::WindowSize(x as f32, y as f32);
        }
    }
}
