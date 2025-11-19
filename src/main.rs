use clap::Parser;
use image::imageops::fast_blur;
use rand::distr::weighted::WeightedIndex;
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input image file path
    input: PathBuf,

    /// Output image file path
    output: PathBuf,

    /// Number of points to generate
    #[arg(short, long, default_value_t = 100)]
    points: usize,

    /// Seed for random number generator
    #[arg(long)]
    seed: Option<u64>,

    /// Color distance weight
    #[arg(short, long, default_value_t = 3.5)]
    weight: f64,

    /// Blur amount before processing
    #[arg(short, long, default_value_t = 1.0)]
    blur: f32,

    /// Add circles at point locations
    #[arg(long)]
    point_radius: Option<u32>,
}

fn weight<const N: usize>(&pixel: &(u32, u32, [u8; N]), width: u32, height: u32) -> f64 {
    // Calculate the weight of the pixel based on its distance to the center of the image.
    // Weight is inversely proportional to the distance.
    let center_x = f64::from(width) / 2.0;
    let center_y = f64::from(height) / 2.0;
    let x_dist = (f64::from(pixel.0) - center_x) / f64::from(width);
    let y_dist = (f64::from(pixel.1) - center_y) / f64::from(height);
    let dist = (x_dist.powi(2) + y_dist.powi(2)).sqrt().sqrt();
    let dist_weight = 1.0 / (dist + 1.0);

    dist_weight - 0.3
}

const COLOR_WEIGHT_MULT: f64 = 10000.0;

fn score<const N: usize>(
    &pixel: &(u32, u32, [u8; N]),
    &point: &(u32, u32, [u8; N]),
    _img: &image::RgbImage,
    color_weight: f64,
    max_color_dist: f64,
    max_pos_dist: f64,
) -> f64 {
    let (x, y, color) = pixel;
    let (px, py, pcolor) = point;

    let pos_dist = f64::from(x.abs_diff(px).pow(2)) + f64::from(y.abs_diff(py).pow(2));

    if let 0.0 = color_weight {
        pos_dist
    } else {
        let color_dist = Iterator::zip(color.iter(), pcolor.iter())
            .map(|(c1, c2)| f64::from(c1.abs_diff(*c2)))
            .sum::<f64>();

        pos_dist / max_pos_dist + color_dist / max_color_dist * color_weight / COLOR_WEIGHT_MULT
    }
}

fn main() {
    let args = Args::parse();

    let img = match image::open(args.input) {
        Err(err) => {
            eprintln!("Failed to open image: {err}");
            std::process::exit(1);
        }
        Ok(img) => img.into_rgb8(),
    };
    let (img_width, img_height) = img.dimensions();
    let img_size = img_height * img_width;
    println!("Image dimensions: {img_width}x{img_height}");

    let max_pos_dist = f64::from(img_width.pow(2)) + f64::from(img_height.pow(2));
    let max_color_dist = 255.0 * f64::from(<image::Rgb<u8> as image::Pixel>::CHANNEL_COUNT);

    let mut rng = {
        let seed = match args.seed {
            Some(seed) => seed,
            None => rand::rng().random::<u64>(),
        };
        println!("Seed: {seed}");
        StdRng::seed_from_u64(seed)
    };

    println!("Points: {}", args.points);
    println!("Color weight: {}", args.weight);

    let pixels = {
        eprint!("Indexing {img_size} pixels...");
        let mut pixels = Vec::with_capacity(img_size as usize);
        for (x, y, px) in img.enumerate_pixels() {
            pixels.push((x, y, px.0));
            if x == 0 {
                eprint!("\rIndexing {img_size} pixels... {y} / {img_height} rows");
            }
        }
        eprintln!("\rIndexing {img_size} pixels... {img_height} / {img_height} rows",);
        pixels
    };

    let points = {
        eprint!("Generating {} points...", args.points);
        let mut points: Vec<(u32, u32, [u8; 3])> = Vec::with_capacity(args.points);
        let weights =
            WeightedIndex::new(pixels.iter().map(|px| weight(px, img_width, img_height))).unwrap();
        for _ in 0..args.points {
            let idx = weights.sample(&mut rng);
            points.push(pixels[idx]);
        }
        eprintln!("\rGenerating {} points... Done", args.points);
        points
    };

    let voronoi = {
        eprint!("Calculating voronoi diagram... 0 / {img_height}");
        let mut voronoi = fast_blur(&img, args.blur);
        for (x, y, pixel) in voronoi.enumerate_pixels_mut() {
            let mut min_score = f64::MAX;
            let mut min_color = [0, 0, 0];
            let mut min_pos = (0, 0);
            for &(px, py, pcolor) in &points {
                let s = score(
                    &(x, y, pixel.0),
                    &(px, py, pcolor),
                    &img,
                    args.weight,
                    max_color_dist,
                    max_pos_dist,
                );
                if s < min_score {
                    min_score = s;
                    min_color = pcolor;
                    min_pos = (px, py);
                }
            }

            if let Some(radius) = args.point_radius && {
                let dx = x.abs_diff(min_pos.0);
                let dy = y.abs_diff(min_pos.1);
                dx * dx + dy * dy <= radius * radius
            } {
                min_color = min_color.map(|c| u8::MAX - c);
            }

            *pixel = image::Rgb(min_color);

            if x == 0 {
                eprint!("\rCalculating voronoi diagram... {y} / {img_height} rows");
            }
        }
        eprintln!("\rCalculating voronoi diagram... {img_height} / {img_height} rows");
        voronoi
    };

    let save_result = voronoi.save(&args.output);
    if let Err(err) = save_result {
        eprintln!("Failed to save image: {err}");
        std::process::exit(1);
    }
    eprintln!("Saved voronoi diagram to {}", &args.output.display());
}
