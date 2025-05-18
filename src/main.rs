use macroquad::{
    color::hsl_to_rgb,
    miniquad::window::screen_size,
    prelude::*,
    ui::{hash, root_ui, widgets::Window},
};
use mandelbrot::calculate_mandelbrot_escape_times_and_paths;
use num::Complex;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

fn rgba_to_array(color: Color) -> [u8; 4] {
    [
        (color.r * 255.0) as _,
        (color.g * 255.0) as _,
        (color.b * 255.0) as _,
        (color.a * 255.0) as _,
    ]
}

fn screen_position_to_complex(
    screen_position: Vec2,
    center: Complex<f32>,
    dimensions: Complex<f32>,
) -> Complex<f32> {
    let x_percent = screen_position.x / screen_width();
    let y_percent = screen_position.y / screen_height();

    let top_left = Complex::new(
        center.re - dimensions.re / 2.0,
        center.im + dimensions.im / 2.0,
    );

    Complex::new(
        top_left.re + x_percent * dimensions.re,
        y_percent * dimensions.im - top_left.im,
    )
}

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
    // Treat scale as a zoom level. Larger values = zoom in
    const BASE_WIDTH: f32 = 4.0; // default view of the Mandelbrot set
    let base_height = BASE_WIDTH * screen_height() / screen_width(); // maintain aspect ratio

    Complex::new(BASE_WIDTH, base_height) / scale
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

fn create_mandelbrot_image(
    mandelbrot_data: &[(Option<usize>, Vec<Complex<f32>>)],
    iteration_max: usize,
) -> Image {
    // start with a blank image
    let mut image = Image::gen_image_color(screen_width() as u16, screen_height() as u16, BLACK);

    // update each pixel color in parallel
    image
        .get_image_data_mut() // we need the image pixel data to change
        .par_iter_mut() // we want to edit all pixels at once
        .zip(mandelbrot_data.par_iter()) // we zip each pixel color with it's mandelbrot data
        .for_each(|(pixel_color, (escape_time, escape_path))| {
            let color = match escape_time {
                &Some(escape_time) => {
                    let last_z = escape_path.last().expect("all paths start at 0+0i");
                    let smoothed_iteration = escape_time as f32 + 1.0 - last_z.norm().log2().log2();
                    let normalized = smoothed_iteration / iteration_max as f32;

                    let hue = (normalized % 1.0).powf(0.7);
                    let saturation = 1.0;
                    let luminance = normalized.powf(0.3) * 0.5;

                    rgba_to_array(hsl_to_rgb(hue, saturation, luminance))
                }
                None => [0, 0, 0, 255],
            };
            *pixel_color = color;
        });

    image
}

fn controls_window(
    center: &mut Complex<f32>,
    scale: &mut f32,
    dimensions: &mut Complex<f32>,
    iteration_max: &mut usize,
    mandelbrot_data: &mut Vec<(Option<usize>, Vec<Complex<f32>>)>,
    image: &mut Image,
    texture: &mut Texture2D,
) {
    let window_size = vec2(250.0, 250.0);
    let generate_text_dimensions = measure_text("Generate Image", None, 16, 1.0);
    let generate_button_position = vec2(0.0, window_size.y - generate_text_dimensions.height * 4.0);
    let reset_button_position = vec2(
        generate_text_dimensions.width * 1.25,
        generate_button_position.y,
    );
    let c_label_dimensions = measure_text("c: ", None, 16, 1.0);
    let c_label_position = vec2(
        0.0,
        generate_button_position.y - c_label_dimensions.height * 4.0,
    );
    Window::new(hash!(), Vec2::ZERO, window_size)
        .label("controls")
        .titlebar(true)
        .ui(&mut *root_ui(), |ui| {
            ui.slider(hash!(), "Center Real", -2.0..2.0, &mut center.re);
            ui.slider(hash!(), "Center Imaginary", -2.0..2.0, &mut center.im);
            ui.slider(hash!(), "Scale", 1.0..1000.0, scale);

            let mut iteration_max_f32 = *iteration_max as f32;
            ui.slider(hash!(), "iterations", 100.0..5000.0, &mut iteration_max_f32);
            *iteration_max = iteration_max_f32 as usize;

            if let Some(c) = mandelbrot_data
                .get(calculate_pixel_index(mouse_position().into()))
                .and_then(|(_, zs)| zs.get(1))
            {
                ui.label(c_label_position, &format!("c: {c}"));
            }
            if ui.button(generate_button_position, "Generate Image") {
                *dimensions = calculate_complex_dimensions(*scale);
                *mandelbrot_data = calculate_mandelbrot_escape_times_and_paths(
                    screen_width() as usize,
                    screen_height() as usize,
                    *center,
                    *dimensions,
                    *iteration_max,
                );
                *image = create_mandelbrot_image(mandelbrot_data, *iteration_max);
                *texture = Texture2D::from_image(image);
            }
            if ui.button(reset_button_position, "Reset") {
                *scale = 1.0;
                *center = Complex::new(-0.4, 0.0);
            }
        });
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
    let mut scale = 1.0;
    let mut center = Complex::new(-0.4, 0.0);

    // define how many iterations of the mandelbrot formula should be performed to determine detail level
    let mut iteration_max = 500;

    let mut dimensions = calculate_complex_dimensions(scale);

    // this is the c value in the mandelbrot formula zₙ₊₁ = zₙ² + c.
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
    let mut image = create_mandelbrot_image(&mandelbrot_data, iteration_max);
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
            let age = (1.0 - (i as f32 / z_values.len() as f32)).clamp(0.3, 1.0);
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
        controls_window(
            &mut center,
            &mut scale,
            &mut dimensions,
            &mut iteration_max,
            &mut mandelbrot_data,
            &mut image,
            &mut texture,
        );

        // this frame is done.
        // tell macroquad it can take control until next frame
        next_frame().await;
    }
}
