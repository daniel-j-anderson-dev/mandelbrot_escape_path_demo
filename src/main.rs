use macroquad::{miniquad::window::screen_size, prelude::*};
use mandelbrot::{calculate_mandelbrot_escape_times_and_paths, escape_time_to_grayscale}; // my library
use num::Complex;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

fn complex_to_screen_coordinate(
    z: Complex<f32>,
    top_left: Complex<f32>,
    bottom_right: Complex<f32>,
) -> Vec2 {
    let x_percent = (z.re - top_left.re) / (bottom_right.re - top_left.re);
    let y_percent = (z.im - top_left.im) / (bottom_right.im - top_left.im);

    vec2(x_percent * screen_width(), y_percent * screen_height())
}

fn serialize_index(row_index: usize, column_index: usize, width: usize) -> usize {
    row_index * width + column_index
}

fn calculate_pixel_index(screen_position: Vec2) -> usize {
    let row_index = screen_position.y as usize;
    let column_index = screen_position.x as usize;
    serialize_index(row_index, column_index, screen_width() as usize)
}

// use the raw mandelbrot data to form a grayscale image
fn create_mandelbrot_image(mandelbrot_data: &[(Option<usize>, Vec<Complex<f32>>)]) -> Image {
    // start with a blank image
    let mut image = Image::gen_image_color(screen_width() as u16, screen_height() as u16, BLACK);

    // update each pixel color in parallel
    image
        .get_image_data_mut() // we need the image pixel data to change
        .par_iter_mut() // we want to edit all pixels at once
        .zip(mandelbrot_data.par_iter()) // we zip each pixel color with it's mandelbrot data
        .for_each(|(pixel_color, &(escape_time, _))| {
            *pixel_color = escape_time_to_grayscale(escape_time).as_array();
        });

    image
}

fn macroquad_configuration() -> Conf {
    Conf {
        window_title: String::from("mandelbrot demo"),
        window_width: 800,
        window_height: 800,
        window_resizable: true,
        high_dpi: true,
        fullscreen: false,
        sample_count: 0,
        icon: None,
        platform: Default::default(),
    }
}

#[macroquad::main(macroquad_configuration)]
async fn main() {
    /* SETUP */
    // define the area of the complex plane being viewed
    const TOP_LEFT: Complex<f32> = Complex::new(-2.0, 1.2);
    const BOTTOM_RIGHT: Complex<f32> = Complex::new(0.5, -1.2);

    // define how many iterations of the mandelbrot formula should be performed to determine detail level
    const ITERATION_MAX: usize = 1000;

    const SIZE: f32 = 3.0;

    // keep track of the screen size to detect a screen size change!
    let mut old_screen_size = screen_size();

    // this is the c value in the mandelbrot formula zₙ₊₁ = zₙ² + c.
    // control this value with the arrow keys!
    let mut c_screen_position = Vec2::ZERO;

    // the c value corresponds to a pixel in the mandelbrot image
    let mut mandelbrot_pixel_index = calculate_pixel_index(c_screen_position);

    // A collection of (escape_time, z_values).
    let mut mandelbrot_data = calculate_mandelbrot_escape_times_and_paths(
        screen_width() as usize,
        screen_height() as usize,
        TOP_LEFT,
        BOTTOM_RIGHT,
        ITERATION_MAX,
    );

    // create an image and texture from the mandelbrot_data
    let mut image = create_mandelbrot_image(&mandelbrot_data);
    let mut texture = Texture2D::from_image(&image);

    /* MAIN LOOP */
    loop {
        /* DRAW LOGIC */
        // clear the background each frame
        clear_background(LIGHTGRAY);

        // draw the mandelbrot picture we generated
        draw_texture(&texture, 0.0, 0.0, WHITE);
        
        let z_values = &mandelbrot_data[mandelbrot_pixel_index].1;

        // draw a circle at each z value and a line connecting to the next z value
        let mut i = z_values.len().saturating_sub(1);
        while i > 0 {
            // make the first z value RED
            let color = if i == 1 { RED } else { ORANGE };

            let start = complex_to_screen_coordinate(z_values[i - 1], TOP_LEFT, BOTTOM_RIGHT);
            let end = complex_to_screen_coordinate(z_values[i], TOP_LEFT, BOTTOM_RIGHT);

            draw_circle(start.x, start.y, SIZE, color);
            draw_line(start.x, start.y, end.x, end.y, 1.0, SKYBLUE);

            i -= 1;
        }

        /* INPUT LOGIC */
        c_screen_position = Vec2::from(mouse_position()).clamp(Vec2::ZERO, screen_size().into());
        draw_hexagon(
            c_screen_position.x,
            c_screen_position.y,
            SIZE,
            SIZE / 2.0,
            false,
            BLUE,
            RED,
        );

        // calculate which pixel the c value corresponds to
        mandelbrot_pixel_index = calculate_pixel_index(c_screen_position);

        // if the screen changes size we need a new mandelbrot image!
        if old_screen_size != screen_size() {
            old_screen_size = screen_size();
            mandelbrot_data = calculate_mandelbrot_escape_times_and_paths(
                screen_width() as usize,
                screen_height() as usize,
                TOP_LEFT,
                BOTTOM_RIGHT,
                ITERATION_MAX,
            );
            image = create_mandelbrot_image(&mandelbrot_data);
            texture = Texture2D::from_image(&image);
        }

        // this frame is done.
        // tell macroquad it can take control until next frame
        next_frame().await;
    }
}
