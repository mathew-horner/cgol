use std::time::Duration;

use clap::{Parser, ValueEnum};
use pixels::{Pixels, SurfaceTexture};
use rand::Rng;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

/// A "cell" in the grid has this number of pixels along its height and width,
/// and each cell is offset by a multiple of this number.
const PIXELS_PER_CELL: usize = 16;

const WINDOW_WIDTH: usize = 800;
const WINDOW_HEIGHT: usize = 640;

/// A color with red, green, and blue components.
#[derive(Clone, Copy)]
struct Rgb(u8, u8, u8);

impl Rgb {
    /// Plain black.
    const BLACK: Rgb = Rgb(0, 0, 0);
    /// Plain white.
    const WHITE: Rgb = Rgb(255, 255, 255);

    /// Generate a random color.
    fn random() -> Self {
        let mut rng = rand::thread_rng();
        let r = rng.gen_range(0..=255);
        let g = rng.gen_range(0..=255);
        let b = rng.gen_range(0..=255);
        Self(r, g, b)
    }
}

/// A location in grid space.
struct GridCoords {
    x: usize,
    y: usize,
}

/// A location in pixel space.
struct PixelCoords {
    x: usize,
    y: usize,
}

impl PixelCoords {
    /// Returns pixel coordinates at the origin of the screen (0, 0).
    fn origin() -> Self {
        Self { x: 0, y: 0 }
    }
}

/// Draw a filled cell in the pixel buffer.
fn fill_cell(frame: &mut [u8], buffer_width: usize, coords: GridCoords, rgb: Rgb) {
    let pixel_coords = PixelCoords { x: coords.x * PIXELS_PER_CELL, y: coords.y * PIXELS_PER_CELL };
    fill_rect(frame, buffer_width, pixel_coords, PIXELS_PER_CELL, PIXELS_PER_CELL, rgb);
}

/// Draw a filled rectangle in the pixel buffer.
fn fill_rect(frame: &mut [u8], buffer_width: usize, coords: PixelCoords, w: usize, h: usize, rgb: Rgb) {
    for y in coords.y..coords.y + h {
        for x in coords.x..coords.x + w {
            let idx = (y * buffer_width + x) * 4;
            frame[idx + 0] = rgb.0;
            frame[idx + 1] = rgb.1;
            frame[idx + 2] = rgb.2;
            frame[idx + 3] = 0xFF;
        }
    }
}

/// Return the number of alive cells out of a given cell's up-to eight
/// neighbors.
fn alive_neighbors(grid: &Vec<Vec<bool>>, x: i32, y: i32) -> u8 {
    const OFFSETS: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)];

    let grid_width = grid[0].len();
    let grid_height = grid.len();
    let mut alive = 0;
    for offset in OFFSETS {
        // Ensure that x and y for this offset are in range, otherwise skip it.
        let Some(x) = usize::try_from(x + offset.0).ok() else { continue };
        let Some(y) = usize::try_from(y + offset.1).ok() else { continue };
        if x >= grid_width || y >= grid_height {
            continue;
        }

        if grid[y][x] {
            alive += 1;
        }
    }
    alive
}

/// Generate and fill a random configuration of the grid.
fn random_configuration(grid: &mut Vec<Vec<bool>>, chance: f64) {
    let mut rng = rand::thread_rng();
    for r in grid {
        for c in r {
            if rng.gen_bool(chance) {
                *c = true;
            }
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long, default_value_t = ColorMode::Monochrome)]
    color_mode: ColorMode,

    #[arg(long, default_value_t = 0.25)]
    alive_random_chance: f64,
}

#[derive(ValueEnum, strum::Display, Clone)]
#[strum(serialize_all = "lowercase")]
enum ColorMode {
    /// Cells will be rendered as plain white.
    Monochrome,
    /// Cells will be rendered with a random color.
    Random,
}

fn main() {
    env_logger::init();
    let args = Cli::parse();
    let event_loop = EventLoop::new();
    let size = PhysicalSize::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64);
    let window = WindowBuilder::new().with_inner_size(size).build(&event_loop).unwrap();

    let pixel_buffer_width = WINDOW_WIDTH;
    let pixel_buffer_height = WINDOW_HEIGHT;

    // In order to properly render the game, we *need* the screen size in each
    // direction to be a multiple of the PIXELS_PER_CELL value.
    assert!(size.width as usize % PIXELS_PER_CELL == 0, "screen width must be a multiple of {PIXELS_PER_CELL}",);
    assert!(size.height as usize % PIXELS_PER_CELL == 0, "screen height must be a multiple of {PIXELS_PER_CELL}",);

    let grid_width = pixel_buffer_width / PIXELS_PER_CELL;
    let grid_height = pixel_buffer_height / PIXELS_PER_CELL;
    let surface_texture = SurfaceTexture::new(pixel_buffer_width as u32, pixel_buffer_height as u32, &window);
    let mut pixels = Pixels::new(pixel_buffer_width as u32, pixel_buffer_height as u32, surface_texture).unwrap();

    // The "grid" is a 2-dimensional state object that stores the alive / dead
    // status of each of its cells.
    let mut grid = vec![vec![false; grid_width]; grid_height];
    random_configuration(&mut grid, args.alive_random_chance);

    let sleep_duration = Duration::from_millis(100);

    let color_gen = match args.color_mode {
        ColorMode::Monochrome => || Rgb::WHITE,
        ColorMode::Random => || Rgb::random(),
    };

    std::thread::spawn(move || loop {
        log::trace!("Tick");
        let frame = pixels.frame_mut();

        // Clear the screen with black.
        fill_rect(
            frame,
            pixel_buffer_width,
            PixelCoords::origin(),
            pixel_buffer_width,
            pixel_buffer_height,
            Rgb::BLACK,
        );

        // Draw the current state of the grid.
        for y in 0..grid_height {
            for x in 0..grid_width {
                if grid[y][x] {
                    fill_cell(frame, pixel_buffer_width, GridCoords { x, y }, color_gen());
                }
            }
        }

        pixels.render().unwrap();

        // Tick the game. Update the state of the grid based on the rules of Conway's
        // Game of Life.
        let mut alive_count = 0;
        let mut next_grid = vec![vec![false; grid_width]; grid_height];
        for y in 0..grid_height {
            for x in 0..grid_width {
                let alive = grid[y][x];
                let alive_neighbors = alive_neighbors(&grid, x as i32, y as i32);

                // Increment the alive count so we don't exit the game prematurely.
                if alive {
                    alive_count += 1;
                }

                match (alive, alive_neighbors) {
                    (true, 2..=3) | (false, 3) => next_grid[y][x] = true,
                    _ => {},
                };
            }
        }

        // If there are no cells left alive, there is nothing left to do but exit!
        if alive_count == 0 {
            std::process::exit(0);
        }

        grid = next_grid;
        // TODO: Don't sleep, use a timer.
        std::thread::sleep(sleep_duration);
    });

    event_loop.run(|_, _, _| {});
}
