use std::time::Duration;

use pixels::{Pixels, SurfaceTexture};
use rand::Rng;
use winit::event_loop::EventLoop;
use winit::window::Window;

/// A "cell" in the grid has this number of pixels along its height and width,
/// and each cell is offset by a multiple of this number.
const PIXELS_PER_CELL: usize = 16;

/// A color with red, green, and blue components.
#[derive(Clone, Copy)]
struct Rgb(u8, u8, u8);

impl Rgb {
    /// Plain black.
    const BLACK: Rgb = Rgb(0, 0, 0);

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

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let size = window.inner_size();
    let pixel_buffer_width = size.width as usize;
    let pixel_buffer_height = size.height as usize;

    log::info!("Window size: w = {}, h = {}", size.width, size.height);

    // In order to properly render the game, we *need* the screen size in each
    // direction to be a multiple of the PIXELS_PER_CELL value.
    assert!(size.width as usize % PIXELS_PER_CELL == 0, "screen width must be a multiple of {PIXELS_PER_CELL}",);
    assert!(size.height as usize % PIXELS_PER_CELL == 0, "screen height must be a multiple of {PIXELS_PER_CELL}",);

    let grid_width = pixel_buffer_width / PIXELS_PER_CELL;
    let grid_height = pixel_buffer_height / PIXELS_PER_CELL;
    let surface_texture = SurfaceTexture::new(size.width, size.height, &window);
    let mut pixels = Pixels::new(size.width, size.height, surface_texture).unwrap();

    // The "grid" is a 2-dimensional state object that stores the alive / dead
    // status of each of its cells.
    let mut grid = vec![vec![false; grid_width]; grid_height];

    // Set up a glider configuration from the top left.
    grid[0][1] = true;
    grid[1][2] = true;
    grid[2][0] = true;
    grid[2][1] = true;
    grid[2][2] = true;

    let sleep_duration = Duration::from_millis(100);

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
                    fill_cell(frame, pixel_buffer_width, GridCoords { x, y }, Rgb::random());
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
