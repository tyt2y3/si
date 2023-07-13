use crate::args::{CheckArgs, Commands, InstallArgs};
use color_eyre::Result;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use std::thread;
use std::time::Duration;
use std::{cmp::min, fmt::Write};

mod args;

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = args::parse();

    match args.command {
        Commands::Install(args) => {
            let command_args = args;
            if !command_args.skip_check {
                check_system(CheckArgs {})?;
            }
            download_containers(command_args)
        }
        Commands::Check(args) => check_system(args),
    }
}

fn check_system(_args: CheckArgs) -> Result<()> {
    println!("Preparing for System Initiative Installation");
    let mut table = comfy_table::Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(100)
        .set_header(vec![
            comfy_table::Cell::new("Dependency").add_attribute(Attribute::Bold),
            comfy_table::Cell::new("Success?").add_attribute(Attribute::Bold),
        ])
        .add_row(vec![
            comfy_table::Cell::new("Detected Docker Engine").add_attribute(Attribute::Bold),
            comfy_table::Cell::new("    ✅    "),
        ])
        .add_row(vec![
            comfy_table::Cell::new("Detected Docker Command").add_attribute(Attribute::Bold),
            comfy_table::Cell::new("    ✅    "),
        ])
        .add_row(vec![
            comfy_table::Cell::new("Docker Compose Available").add_attribute(Attribute::Bold),
            comfy_table::Cell::new("    ✅    "),
        ])
        .add_row(vec![
            comfy_table::Cell::new("Found `bash` in Nix environment")
                .add_attribute(Attribute::Bold),
            comfy_table::Cell::new("    ✅    "),
        ])
        .add_row(vec![
            comfy_table::Cell::new("Found nix environment").add_attribute(Attribute::Bold),
            comfy_table::Cell::new("    ✅    "),
        ])
        .add_row(vec![
            comfy_table::Cell::new("Reasonable value for max open files")
                .add_attribute(Attribute::Bold),
            comfy_table::Cell::new("    ❌    "),
        ]);

    println!("{table}");

    Ok(())
}

fn download_containers(args: InstallArgs) -> Result<()> {
    format_args!("Starting {:?} install of System Initiative", args.mode());
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "{spinner:.red} [{elapsed_precise}] [{wide_bar:.yellow/blue}] {bytes}/{total_bytes} ({eta})",
    )
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
            write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
        })
        .progress_chars("#>-");

    let mut downloaded = 0;
    let total_size = 231231231;

    let pb = m.add(ProgressBar::new(total_size));
    pb.set_style(sty.clone());

    let pb2 = m.insert_after(&pb, ProgressBar::new(total_size));
    pb2.set_style(sty.clone());

    let pb3 = m.insert_after(&pb2, ProgressBar::new(total_size * 2));
    pb3.set_style(sty);

    m.println("Downloading System Initiative artifacts")
        .unwrap();

    let h1 = thread::spawn(move || {
        while downloaded < total_size {
            let new = min(downloaded + 223211, total_size);
            downloaded = new;
            pb.set_position(new);
            thread::sleep(Duration::from_millis(12));
        }
    });

    let h2 = thread::spawn(move || {
        while downloaded < total_size {
            let new = min(downloaded + 223211, total_size);
            downloaded = new;
            pb2.set_position(new);
            thread::sleep(Duration::from_millis(12));
        }
    });

    let h3 = thread::spawn(move || {
        while downloaded < total_size {
            let new = min(downloaded + 223211, total_size);
            downloaded = new;
            pb3.set_position(new);
            thread::sleep(Duration::from_millis(12));
        }
    });

    let _ = h1.join();
    let _ = h2.join();
    let _ = h3.join();

    m.println("System Initiative Successfully Installed")
        .unwrap();
    m.clear().unwrap();

    Ok(())
}
