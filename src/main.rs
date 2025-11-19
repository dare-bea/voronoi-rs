use clap::Parser;
use rand::distr::weighted::WeightedIndex;
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::path::PathBuf;
use image::imageops::fast_blur;

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


fn score<const N: usize>(
    &pixel: &(u32, u32, [u8; N]),
    &point: &(u32, u32, [u8; N]),
    _: &image::RgbImage
) -> f64 {
    let (x, y, color) = pixel;
    let (px, py, pcolor) = point;
    let pos_dist = x.abs_diff(px).pow(2) as f64 + y.abs_diff(py).pow(2) as f64;
    let color_dist = Iterator::zip(color.iter(), pcolor.iter())
        .map(|(c1, c2)| c1.abs_diff(*c2) as f64)
        .sum::<f64>();
    
    pos_dist + color_dist
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
    println!("Image dimensions: {img_width}x{img_height}");

    let mut rng = {
        let seed = match args.seed {
            Some(seed) => seed,
            None => rand::rng().random::<u64>(),
        };
        println!("Seed: {seed}");
        StdRng::seed_from_u64(seed)
    };

    let all_points = {
        eprint!("Indexing {img_size} points...");
        let mut all_points = Vec::with_capacity(img_size as usize);
        for (x, y, px) in img.enumerate_pixels() {
            all_points.push((x, y, px.clone().0));
            if x == 0 {
                eprint!(
                    "\rIndexing {img_size} points... {y} / {img_height} rows"
                );
            }
        }
        eprintln!("\rIndexing {img_size} points... {img_height} / {img_height} rows",);
        all_points
    }

    let points = {
        eprint!("Generating {} points...", args.points);
        let mut points: Vec<(u32, u32, [u8; 3])> = Vec::with_capacity(args.points);
        let weights = WeightedIndex::new(
            all_points
                .iter()
                .map(|px| weight(px, img_width, img_height)),
        ).unwrap();
        for _ in 0..args.points {
            let idx = weights.sample(&mut rng);
            points.push(all_points[idx]);
        }
        eprintln!("\rGenerating {} points... Done", args.points);
        points
    };

    let voronoi = {
        eprint!("Calculating voronoi diagram... 0 / {img_height}");
        let mut voronoi = fast_blur(&img, 1.0);
        for (x, y, pixel) in voronoi.enumerate_pixels_mut() {
            let mut min_score = f64::MAX;
            let mut min_color = [0, 0, 0];
            for &(px, py, pcolor) in &points {
                let s = score(&(x, y, pixel.clone().0), &(px, py, pcolor), &img);
                if s < min_score {
                    min_score = s;
                    min_color = pcolor;
                }
            }
    
            *pixel = image::Rgb(min_color);
    
            if x == 0 {
                eprint!("\rCalculating voronoi diagram... {y} / {img_height} rows");
            }
        }
        eprintln!("\rCalculating voronoi diagram... {img_height} / {img_height} rows");
        voronoi
    }

    let save_result = voronoi.save(&args.output);
    if let Err(err) = save_result {
        eprintln!("Failed to save image: {err}");
        std::process::exit(1);
    }
    eprintln!("Saved voronoi diagram to {}", &args.output.display());
}
