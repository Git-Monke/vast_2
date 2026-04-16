use std::env;
use universe::generator::star_info_at;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: star_finder <x> <y> <radius>");
        return;
    }

    let target_x: i32 = args[1].parse().expect("Invalid x coordinate");
    let target_y: i32 = args[2].parse().expect("Invalid y coordinate");
    let radius: i32 = args[3].parse().expect("Invalid radius");

    let mut found_stars = Vec::new();

    for x in (target_x - radius)..=(target_x + radius) {
        for y in (target_y - radius)..=(target_y + radius) {
            let dx = x - target_x;
            let dy = y - target_y;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq <= radius * radius {
                if let Some((star_type, size)) = star_info_at(x, y) {
                    found_stars.push((x, y, star_type, size, dist_sq));
                }
            }
        }
    }

    found_stars.sort_by_key(|&(_, _, _, _, dist_sq)| dist_sq);

    println!(
        "Found {} stars within radius {}:",
        found_stars.len(),
        radius
    );
    println!(
        "{:<10} {:<10} {:<15} {:<10} {:<10}",
        "X", "Y", "Type", "Size", "Dist"
    );
    for (x, y, star_type, size, dist_sq) in found_stars {
        let dist = (dist_sq as f64).sqrt();
        let star_type_str = format!("{:?}", star_type);
        println!(
            "{:<10} {:<10} {:<15} {:<10.2} {:<10.2}",
            x, y, star_type_str, size, dist
        );
    }
}
