use std::cmp::max;
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

#[allow(unused_imports)]
use rand::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::messagebox::{self, ButtonData, ClickedButton, MessageBoxButtonFlag, MessageBoxFlag};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::Sdl;

const WIDTH: usize = 480;
const HEIGHT: usize = 480;

#[derive(Debug, Default)]
struct Obstacle {}
#[derive(Debug, Default)]
struct Player {}

#[derive(Debug, Clone, Copy)]
struct Position {
    pub x: isize,
    pub y: isize,
}

impl Position {
    fn new(x: isize, y: isize) -> Self {
        Position { x, y }
    }

    fn unsafe_left(&mut self) {
        self.x -= 1;
    }

    fn left(&mut self) {
        if self.x != 0 {
            self.x -= 1;
        }
    }

    fn right(&mut self) {
        if self.x != WIDTH as isize {
            self.x += 1;
        }
    }

    fn down(&mut self) {
        if self.y != HEIGHT as isize {
            self.y += 1;
        }
    }

    fn up(&mut self) {
        if self.y != 0 {
            self.y -= 1;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Coverage {
    width: usize,
    height: usize,
}

impl Coverage {
    fn new(width: usize, height: usize) -> Self {
        Coverage { width, height }
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

enum Action {
    Left,
    Right,
    Up,
    Down,
}

const MAX_OBST: usize = 512;

struct World {
    obstacles: VecDeque<(Obstacle, Position, Coverage)>,
    player: (Player, Position, Coverage),
    last_action: Option<Action>,
    timer: usize,
    rng: rand::rngs::ThreadRng,
}

fn obj_to_rect(p: Position, c: Coverage) -> Rect {
    Rect::new(
        max(p.x, 0) as i32,
        max(p.y, 0) as i32,
        c.width() as u32,
        c.height() as u32,
    )
}

fn is_collided((f_pos, f_cov): (Position, Coverage), (s_pos, _): (Position, Coverage)) -> bool {
    let in_range = |min: isize, delta: usize, x: isize| min <= x && x <= min + (delta as isize);
    in_range(f_pos.x, f_cov.width, s_pos.x) && in_range(f_pos.y, f_cov.height, s_pos.y)
}

impl World {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0, WIDTH / 2);
        let y = rng.gen_range(0, HEIGHT / 2);
        World {
            obstacles: VecDeque::with_capacity(MAX_OBST),
            player: (
                Default::default(),
                Position::new(x as isize, y as isize),
                Coverage::new(5, 5),
            ),
            last_action: None,
            timer: 0,
            rng,
        }
    }

    const SPAWN_DELAY: usize = 150;

    fn check_collisions(&self) -> bool {
        let (pos, cov) = (self.player.1, self.player.2);
        self.obstacles
            .iter()
            .any(|(_, p, c)| is_collided((*p, *c), (pos, cov)))
    }

    fn cleanup(&mut self) {
        while let Some(obs) = self.obstacles.pop_front() {
            if obs.1.x + (obs.2.width as isize) > 0 {
                self.obstacles.push_front(obs);
                return;
            }
        }
    }

    /// Makes a world tick
    /// Returns true, if player still alive
    fn tick(&mut self) -> bool {
        self.timer += 1;
        match self.last_action.take() {
            Some(Action::Left) => self.player.1.left(),
            Some(Action::Right) => self.player.1.right(),
            Some(Action::Up) => self.player.1.up(),
            Some(Action::Down) => self.player.1.down(),
            None => {}
        }

        if self.check_collisions() {
            return false;
        }

        // moving obstacles
        for (_, p, _) in self.obstacles.iter_mut() {
            p.unsafe_left();
        }

        self.cleanup();

        // generating newer ones
        if self.timer % Self::SPAWN_DELAY == 0 {
            let num = self.rng.gen_range(2, 10);
            eprintln!("spawned {}", num);
            for _ in 0..num {
                let var = (
                    Obstacle {},
                    Position {
                        x: WIDTH as isize,
                        y: self.rng.gen_range(0, HEIGHT) as isize,
                    },
                    Coverage::new(self.rng.gen_range(5, 32), self.rng.gen_range(5, 32)),
                );
                self.obstacles.push_back(var);
            }
            eprintln!("{:?}", self.obstacles);
        }

        if self.check_collisions() {
            return false;
        }

        true
    }

    fn draw_obstacles<O: Default, E>(
        &self,
        mut drawer: impl FnMut(Rect) -> Result<O, E>,
    ) -> Result<O, E> {
        let o: O = Default::default();
        self.obstacles
            .iter()
            .map(|(_, p, c)| (p.to_owned(), c.to_owned()))
            .map(|(p, c)| obj_to_rect(p, c))
            .fold(Ok(o), |res, val| match res {
                Ok(_) => drawer(val),
                err => err,
            })
    }

    fn draw_player<T>(&self, mut drawer: impl FnMut(Rect) -> T) -> T {
        let pos = self.player.1;
        let cov = self.player.2;
        drawer(obj_to_rect(pos, cov))
    }
}

pub enum Finished {
    Exit,
    Restart,
    Error,
}

pub fn run(canvas: &mut WindowCanvas, sdl_context: &mut Sdl) -> Finished {
    canvas
        .window_mut()
        .set_title("Simple platformer")
        .expect("rename window failed");
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut world = World::new();
    loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Finished::Exit,
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => world.last_action = Some(Action::Down),
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => world.last_action = Some(Action::Up),
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => world.last_action = Some(Action::Left),
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => world.last_action = Some(Action::Right),
                _ => {}
            }
        }
        if !world.tick() {
            let restart_id = 1;
            let exit_id = 2;
            let buttons = [
                ButtonData {
                    flags: MessageBoxButtonFlag::RETURNKEY_DEFAULT,
                    button_id: restart_id,
                    text: "Restart",
                },
                ButtonData {
                    flags: MessageBoxButtonFlag::ESCAPEKEY_DEFAULT,
                    button_id: exit_id,
                    text: "Exit",
                },
            ];
            let points = world.timer as f64 / World::SPAWN_DELAY as f64;
            let clicked = messagebox::show_message_box(
                MessageBoxFlag::INFORMATION,
                &buttons,
                "Game over!",
                &format!("Your points: {}", points),
                canvas.window(),
                None,
            );
            return match clicked {
                Ok(ClickedButton::CloseButton) => Finished::Exit,
                Ok(ClickedButton::CustomButton(ButtonData { button_id, .. })) => match button_id {
                    id if id == &exit_id => Finished::Exit,
                    id if id == &restart_id => Finished::Restart,
                    _ => Finished::Error,
                },
                Err(_) => Finished::Error,
            };
        }

        canvas.set_draw_color(Color::RGB(255, 0, 0));
        if let Err(e) = world.draw_obstacles(|x| canvas.draw_rect(x)) {
            eprintln!("{:?}", e);
            return Finished::Error;
        }
        canvas.present();
        canvas.set_draw_color(Color::RGB(0, 255, 255));
        if let Err(e) = world.draw_player(|x| canvas.draw_rect(x)) {
            eprintln!("{:?}", e);
            return Finished::Error;
        }
        canvas.present();
        thread::sleep(Duration::from_millis(10));
    }
}

pub fn main() {
    let mut sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rust-sdl2 demo", WIDTH as u32, HEIGHT as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    while let Finished::Restart = run(&mut canvas, &mut sdl_context) {}
}
