#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

//! ProtonDB search CLI utility.
//!
//! This program is not affiliated with [ProtonDB](https://www.protondb.com), Algolia, or Steam in any way.

use crate::apis::{ALGOLIA_LIMIT, AlgoliaEntry, ProtonResponse, query_algolia, query_protondb};
use clap::Parser;
use comfy_table::{Attribute, Cell, Table, presets::UTF8_FULL};
use std::process::exit;

mod apis;

/// ProtonDB search CLI utility
#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Cli {
    /// Game to search for
    game: String,

    /// Enable extra logging
    #[clap(short, long, default_value_t = false)]
    debug: bool,
}

/// Log to the console if the debug flag is enabled.
fn debug(cli: &Cli, msg: &str) {
    if cli.debug {
        println!("[DEBUG] {msg}");
    }
}

/// Capitalize the first letter of the string.
fn title_case(s: &str) -> String {
    s.chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
        .collect()
}

fn hyperlink(url: &str, text: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\\x1b[4;34m{text}\x1b[0m\x1b]8;;\x1b\\")
}

fn format_results(games: &[(&AlgoliaEntry, ProtonResponse)]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        Cell::new("Name").add_attribute(Attribute::Bold),
        Cell::new("Release Year").add_attribute(Attribute::Bold),
        Cell::new("Tier").add_attribute(Attribute::Bold),
        Cell::new("Confidence").add_attribute(Attribute::Bold),
        Cell::new("Steam Deck").add_attribute(Attribute::Bold),
    ]);
    for (algolia, proton) in games {
        let steam_deck = if let Some(s) = algolia
            .os_list
            .iter()
            .find(|s| s.starts_with("Steam Deck "))
        {
            s.split_whitespace().nth(2).unwrap_or_default()
        } else {
            "Unverified"
        };
        table.add_row(vec![
            &algolia.name,
            &algolia
                .release_year
                .as_ref()
                .map(std::string::ToString::to_string)
                .unwrap_or_default(),
            &title_case(&proton.tier),
            &title_case(&proton.confidence),
            steam_deck,
        ]);
    }
    // Build the table with plain names so comfy-table measures widths correctly,
    // then replace each name with a hyperlink in the rendered string.
    let mut output = table.to_string();
    let mut replacements: Vec<_> = games
        .iter()
        .map(|(algolia, _)| {
            let url = format!("https://www.protondb.com/app/{}", algolia.object_id);
            (algolia.name.clone(), hyperlink(&url, &algolia.name))
        })
        .collect();
    // Replace longest names first to avoid a shorter name clobbering a longer one.
    replacements.sort_by_key(|b| std::cmp::Reverse(b.0.len()));
    for (plain, linked) in replacements {
        output = output.replace(&plain, &linked);
    }
    println!("{output}");
}

fn main() {
    let cli = Cli::parse();
    debug(&cli, &format!("Game is: {}", cli.game));

    let matching_games = match query_algolia(&cli.game) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error getting data from Algolia: {e}");
            exit(1);
        }
    };
    debug(
        &cli,
        &format!(
            "Got {}/{ALGOLIA_LIMIT} matching games in Algolia response",
            matching_games.len()
        ),
    );
    let game_data: Vec<_> = matching_games
        .iter()
        .filter_map(|game| {
            if let Ok(d) = query_protondb(&game.object_id) {
                Some((game, d))
            } else {
                debug(&cli, &format!("Could not get game data for {}", game.name));
                None
            }
        })
        .collect();

    format_results(&game_data);
}
