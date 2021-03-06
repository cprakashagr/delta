#[macro_use]
extern crate error_chain;

mod align;
mod bat;
mod cli;
mod config;
mod delta;
mod draw;
mod edits;
mod env;
mod paint;
mod parse;
mod style;

use std::io::{self, BufRead, ErrorKind, Read, Write};
use std::process;

use ansi_term;
use atty;
use structopt::StructOpt;
use syntect::highlighting::{Color, FontStyle, Style};

use crate::bat::assets::{list_languages, HighlightingAssets};
use crate::bat::output::{OutputType, PagingMode};
use crate::delta::delta;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            SyntectError(::syntect::LoadingError);
            ParseIntError(::std::num::ParseIntError);
        }
    }
}

fn main() -> std::io::Result<()> {
    let opt = cli::Opt::from_args();

    let assets = HighlightingAssets::new();

    if opt.list_languages {
        list_languages()?;
        process::exit(0);
    } else if opt.list_theme_names {
        list_theme_names()?;
        process::exit(0);
    } else if opt.list_themes {
        list_themes(&assets)?;
        process::exit(0);
    }

    let config = cli::process_command_line_arguments(&assets, &opt);

    if opt.show_background_colors {
        show_background_colors(&config);
        process::exit(0);
    }

    let mut output_type = OutputType::from_mode(config.paging_mode, None).unwrap();
    let mut writer = output_type.handle().unwrap();

    if let Err(error) = delta(
        io::stdin().lock().lines().map(|l| l.unwrap()),
        &config,
        &assets,
        &mut writer,
    ) {
        match error.kind() {
            ErrorKind::BrokenPipe => process::exit(0),
            _ => eprintln!("{}", error),
        }
    };
    Ok(())
}

fn show_background_colors(config: &config::Config) {
    println!(
        "delta \
         --minus-color=\"{minus_color}\" \
         --minus-emph-color=\"{minus_emph_color}\" \
         --plus-color=\"{plus_color}\" \
         --plus-emph-color=\"{plus_emph_color}\"",
        minus_color = get_painted_rgb_string(
            config.minus_style_modifier.background.unwrap(),
            config.true_color
        ),
        minus_emph_color = get_painted_rgb_string(
            config.minus_emph_style_modifier.background.unwrap(),
            config.true_color
        ),
        plus_color = get_painted_rgb_string(
            config.plus_style_modifier.background.unwrap(),
            config.true_color
        ),
        plus_emph_color = get_painted_rgb_string(
            config.plus_emph_style_modifier.background.unwrap(),
            config.true_color
        ),
    )
}

fn get_painted_rgb_string(color: Color, true_color: bool) -> String {
    let mut string = String::new();
    let style = Style {
        foreground: style::NO_COLOR,
        background: color,
        font_style: FontStyle::empty(),
    };
    paint::paint_text(
        &format!("#{:02x?}{:02x?}{:02x?}", color.r, color.g, color.b),
        style,
        &mut string,
        true_color,
    );
    string.push_str("\x1b[0m"); // reset
    string
}

fn list_themes(assets: &HighlightingAssets) -> std::io::Result<()> {
    let opt = cli::Opt::from_args();
    let mut input = String::new();
    if atty::is(atty::Stream::Stdin) {
        input = "\
diff --git a/example.rs b/example.rs
index f38589a..0f1bb83 100644
--- a/example.rs
+++ b/example.rs
@@ -1,5 +1,5 @@
-// Output the square of a number.
-fn print_square(num: f64) {
-    let result = f64::powf(num, 2.0);
-    println!(\"The square of {:.2} is {:.2}.\", num, result);
+// Output the cube of a number.
+fn print_cube(num: f64) {
+    let result = f64::powf(num, 3.0);
+    println!(\"The cube of {:.2} is {:.2}.\", num, result);
 }"
        .to_string()
    } else {
        io::stdin().read_to_string(&mut input)?;
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let style = ansi_term::Style::new().bold();

    for (theme, _) in assets.theme_set.themes.iter() {
        if opt.light && !style::is_light_theme(theme) || opt.dark && style::is_light_theme(theme) {
            continue;
        }

        writeln!(stdout, "\nTheme: {}\n", style.paint(theme))?;
        let new_opt = cli::Opt {
            theme: Some(theme.to_string()),
            ..opt.clone()
        };
        let config = cli::process_command_line_arguments(&assets, &new_opt);
        let mut output_type = OutputType::from_mode(PagingMode::QuitIfOneScreen, None).unwrap();
        let mut writer = output_type.handle().unwrap();

        if let Err(error) = delta(
            input.split('\n').map(String::from),
            &config,
            &assets,
            &mut writer,
        ) {
            match error.kind() {
                ErrorKind::BrokenPipe => process::exit(0),
                _ => eprintln!("{}", error),
            }
        };
    }
    Ok(())
}

pub fn list_theme_names() -> std::io::Result<()> {
    let assets = HighlightingAssets::new();
    let themes = &assets.theme_set.themes;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    writeln!(stdout, "Light themes:")?;
    for (theme, _) in themes.iter() {
        if style::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    writeln!(stdout, "Dark themes:")?;
    for (theme, _) in themes.iter() {
        if !style::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    Ok(())
}
