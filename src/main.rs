use clap::Parser;
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
}

fn weight(&pixel: &(u32, u32, [u8; 3]), width: u32, height: u32) -> f64 {
    // Calculate the weight of the pixel based on its distance to the center of the image.
    // Weight is inversely proportional to the distance.
    let center_x = width as f64 / 2.0;
    let center_y = height as f64 / 2.0;
    let x_dist = (pixel.0 as f64 - center_x) / width as f64;
    let y_dist = (pixel.1 as f64 - center_y) / height as f64;
    let dist = (x_dist.powi(2) + y_dist.powi(2)).sqrt().sqrt();
    let dist_weight = 1.0 / (dist + 1.0);

    dist_weight - 0.3
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
    let img_height = img.height();
    let img_width = img.width();
    let img_size = img_height * img_width;
    println!("Image dimensions: {}x{}", img_width, img_height);

    let seed = match args.seed {
        Some(seed) => seed,
        None => rand::rng().random::<u64>(),
    };
    println!("Seed: {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    eprint!("Indexing {} points...", img_size);
    let mut all_points = Vec::with_capacity(img_size as usize);
    for (x, y, px) in img.enumerate_pixels() {
        all_points.push((x, y, px.0));
        if x == 0 {
            eprint!(
                "\rIndexing {} points... {y} / {1} rows",
                img_size, img_height
            );
        }
    }
    eprintln!("\rIndexing {img_size} points... {img_height} / {img_height} rows",);

    eprint!("Generating {} points...", args.points);
    let mut points: Vec<(u32, u32, [u8; 3])> = Vec::with_capacity(args.points);
    let dist2 = WeightedIndex::new(
        all_points
            .iter()
            .map(|px| weight(px, img_width, img_height)),
    )
    .unwrap();
    for _ in 0..args.points {
        let idx = dist2.sample(&mut rng);
        points.push(all_points[idx]);
    }
    eprintln!("\rGenerating {} points... Done", args.points);

    eprint!("Calculating voronoi diagram... 0 / {img_height}");
    let mut voronoi = image::RgbImage::new(img_width, img_height);
    for (x, y, color) in voronoi.enumerate_pixels_mut() {
        let mut min_dist = f64::MAX;
        let mut min_color = [0, 0, 0];
        for (px, py, pcolor) in &points {
            let dist = ((x as i32 - *px as i32).pow(2) + (y as i32 - *py as i32).pow(2)) as f64;
            if dist < min_dist {
                min_dist = dist;
                min_color = *pcolor;
            }
        }

        *color = image::Rgb(min_color);

        if x == 0 {
            eprint!("\rCalculating voronoi diagram... {y} / {img_height} rows");
        }
    }
    eprintln!("\rCalculating voronoi diagram... {img_height} / {img_height} rows");

    let save_result = voronoi.save(&args.output);
    if let Err(err) = save_result {
        eprintln!("Failed to save image: {err}");
        std::process::exit(1);
    }
    eprintln!("Saved voronoi diagram to {}", &args.output.display());
}
