use macroquad::{miniquad::window::screen_size, prelude::*};
use mandelbrot::{
    calculate_mandelbrot_escape_times_and_paths, escape_time_to_grayscale, pixel_to_complex,
};
use num::Complex;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

fn complex_to_screen_coordinate(
    z: Complex<f32>,
    center: Complex<f32>,
    dimensions: Complex<f32>,
) -> Vec2 {
    let top_left = Complex::new(
        center.re - dimensions.re / 2.0,
        center.im + dimensions.im / 2.0,
    );

    let x_percent = (z.re - top_left.re) / dimensions.re;
    let y_percent = 1.0 - (top_left.im - z.im) / dimensions.im;

    let x = x_percent * screen_width();
    let y = y_percent * screen_height();

    vec2(x, y)
}

fn calculate_complex_dimensions(scale: f32) -> Complex<f32> {
    let dimensions = Complex::new(screen_width(), screen_height());
    (dimensions / dimensions.norm()).scale(scale)
}

fn serialize_index(row_index: usize, column_index: usize, width: usize) -> usize {
    row_index * width + column_index
}

fn calculate_pixel_index(screen_position: Vec2) -> usize {
    let row_index = screen_position.y as usize;
    let column_index = screen_position.x as usize;
    let width = screen_width() as usize;

    serialize_index(row_index, column_index, width)
}

// use the raw mandelbrot data to form a grayscale image
fn create_mandelbrot_image(
    mandelbrot_data: &[(Option<usize>, Vec<Complex<f32>>)],
    center: Complex<f32>,
    dimensions: Complex<f32>,
) -> Image {
    // start with a blank image
    let mut image = Image::gen_image_color(screen_width() as u16, screen_height() as u16, BLACK);

    // update each pixel color in parallel
    let w = screen_width() as usize;
    let h = screen_height() as usize;
    image
        .get_image_data_mut() // we need the image pixel data to change
        .par_iter_mut() // we want to edit all pixels at once
        .zip(mandelbrot_data.par_iter()) // we zip each pixel color with it's mandelbrot data
        .enumerate()
        .for_each(|(i, (pixel_color, &(escape_time, _)))| {
            let row_index = i / w;
            let column_index = i % w;
            let c = pixel_to_complex(column_index, row_index, w, h, center, dimensions);
            // draw x axis
            if (-0.001..0.001).contains(&c.re) {
                *pixel_color = [0, 255, 0, 255];
            }
            // draw y axis
            else if c.im == 0.0 {
                *pixel_color = [255, 0, 0, 255];
            }
            // draw mandelbrot
            else {
                *pixel_color = escape_time_to_grayscale(escape_time).as_array();
            }
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
    let scale = 4.0;
    let center = Complex::new(-0.4, 0.0);

    // define how many iterations of the mandelbrot formula should be performed to determine detail level
    let iteration_max = 500;

    let mut dimensions = calculate_complex_dimensions(scale);

    // this is the c value in the mandelbrot formula zₙ₊₁ = zₙ² + c.
    // control this value with the arrow keys!
    let mut c_screen_position = Vec2::ZERO;

    // A collection of (escape_time, z_values).
    let mut mandelbrot_data = calculate_mandelbrot_escape_times_and_paths(
        screen_width() as usize,
        screen_height() as usize,
        center,
        dimensions,
        iteration_max,
    );

    // create an image and texture from the mandelbrot_data
    let mut image = create_mandelbrot_image(&mandelbrot_data, center, dimensions);
    let mut texture = Texture2D::from_image(&image);

    /* MAIN LOOP */
    loop {
        /* DRAW LOGIC */
        // clear the background each frame
        clear_background(LIGHTGRAY);

        // draw the mandelbrot picture we generated
        draw_texture(&texture, 0.0, 0.0, WHITE);

        // draw a circle at each z value and a line connecting to the next z value
        let z_values = mandelbrot_data
            .get(calculate_pixel_index(c_screen_position))
            .map(|(_escape_time, escape_path)| escape_path.as_slice())
            .unwrap_or(&[]);
        for i in 0..z_values.len().saturating_sub(1) {
            // make size an opacity proportional to the index as a percentage
            let age = (1.0 - (i as f32 / z_values.len() as f32)).clamp(0.5, 1.0);
            let dot_color = match i {
                0 => LIGHTGRAY,
                1 => RED,
                _ => ORANGE,
            }
            .with_alpha(age);
            let line_color = SKYBLUE.with_alpha(age);
            let size = 3.0 * age;

            let start = complex_to_screen_coordinate(z_values[i], center, dimensions);
            let end = complex_to_screen_coordinate(z_values[i + 1], center, dimensions);

            draw_line(start.x, start.y, end.x, end.y, size / 3.0, line_color);
            draw_circle(start.x, start.y, size, dot_color);
        }

        /* INPUT LOGIC */
        c_screen_position = Vec2::from(mouse_position()).clamp(Vec2::ZERO, screen_size().into());

        // if the screen changes size we need a new mandelbrot image!
        if dimensions != calculate_complex_dimensions(scale) {
            dimensions = calculate_complex_dimensions(scale);
            mandelbrot_data = calculate_mandelbrot_escape_times_and_paths(
                screen_width() as usize,
                screen_height() as usize,
                center,
                dimensions,
                iteration_max,
            );
            image = create_mandelbrot_image(&mandelbrot_data, center, dimensions);
            texture = Texture2D::from_image(&image);
        }

        // this frame is done.
        // tell macroquad it can take control until next frame
        next_frame().await;
    }
}
