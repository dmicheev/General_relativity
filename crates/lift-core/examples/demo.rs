use lift_core::{photon_lift, photon_external, Mode, PhotonLaunch};

const W: f64 = 3.0;
const H: f64 = 3.0;
const LASER_Y: f64 = 1.8;
const C_VIS: f64 = 10.0;
const COLS: usize = 60;
const ROWS: usize = 21;

fn plot(title: &str, points: &[(f64, f64)], marker: char) {
    println!("\n{title}");
    let mut grid = vec![vec![' '; COLS]; ROWS];
    for row in grid.iter_mut() {
        row[0] = '|';
    }
    for row in grid.iter_mut().take(ROWS - 1) {
        row[COLS - 1] = '|';
    }
    let floor = ROWS - 1;
    for col in grid[floor].iter_mut() {
        *col = '=';
    }
    for &(x, y) in points {
        if x < 0.0 || x > W || y < 0.0 || y > H {
            continue;
        }
        let col = (x / W * (COLS - 1) as f64).round() as usize;
        let row = (y / H * (ROWS - 1) as f64).round() as usize;
        let row = (ROWS - 1) - row;
        grid[row][col] = marker;
    }
    for row in &grid {
        println!("{}", row.iter().collect::<String>());
    }
}

fn sample_lift(mode: Mode) -> Vec<(f64, f64)> {
    let frame = mode.params();
    let launch = PhotonLaunch { origin: [0.0, LASER_Y, 0.0], c: C_VIS };
    let t_cross = W / C_VIS;
    (0..=COLS)
        .map(|i| {
            let t = t_cross * i as f64 / COLS as f64;
            let p = photon_lift(t, &launch, &frame);
            (p[0], p[1])
        })
        .collect()
}

fn sample_external(mode: Mode) -> Vec<(f64, f64)> {
    let frame = mode.params();
    let launch = PhotonLaunch { origin: [0.0, LASER_Y, 0.0], c: C_VIS };
    let t_cross = W / C_VIS;
    (0..=COLS)
        .map(|i| {
            let t = t_cross * i as f64 / COLS as f64;
            let p = photon_external(t, &launch, frame.external_g);
            (p[0], p[1])
        })
        .collect()
}

fn main() {
    println!("Лифт Эйнштейна: луч света, c_vis = {C_VIS} м/с, лазер на y = {LASER_Y} м");
    println!("{:>10} {:>10} {:>10} {:>12}", "режим", "g_ext", "a_lift", "g_eff");
    for mode in [Mode::Planet, Mode::Rocket, Mode::FreeFall, Mode::Inertial] {
        let p = mode.params();
        println!(
            "{:>10} {:>10.2} {:>10.2} {:>12.2}",
            format!("{mode:?}"),
            p.external_g,
            p.lift_a,
            p.g_eff() + 0.0
        );
    }

    for mode in [Mode::Planet, Mode::Rocket, Mode::FreeFall, Mode::Inertial] {
        let label = format!("{mode:?}");
        plot(
            &format!("КАБИНА (кадр лифта), режим {label}:"),
            &sample_lift(mode),
            '*',
        );
    }

    println!("\nДля сравнения: тот же луч во ВНЕШНЕМ кадре (не виден наблюдателю в кабине)");
    for mode in [Mode::Rocket, Mode::FreeFall] {
        let label = format!("{mode:?}");
        plot(&format!("внешний кадр, режим {label}:"), &sample_external(mode), '.');
    }

    let p = Mode::Planet.params();
    let t = W / C_VIS;
    let dy = LASER_Y - photon_lift(t, &PhotonLaunch { origin: [0.0, LASER_Y, 0.0], c: C_VIS }, &p)[1];
    println!("\nОтклонение луча в кабине при g_eff = 9.81: {dy:.3} м за t = {t:.1} с");
}
