use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc;

use anyhow::{anyhow, Context, Result};
use clap::{Arg, Command};

mod config;
mod db;
mod downloads;
mod feeds;
mod keymap;
mod main_controller;
mod opml;
mod play_file;
mod threadpool;
mod types;
mod ui;

use crate::config::Config;
use crate::db::Database;
use crate::feeds::{FeedMsg, PodcastFeed};
use crate::main_controller::{MainController, MainMessage};
use crate::threadpool::Threadpool;
use crate::types::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Main controller for shellcaster program.
///
/// *Main command:*
/// Setup involves connecting to the sqlite database (creating it if
/// necessary), then querying the list of podcasts and episodes. This
/// is then passed off to the UI, which instantiates the menus displaying
/// the podcast info.
///
/// After this, the program enters a loop that listens for user keyboard
/// input, and dispatches to the proper module as necessary. User input
/// to quit the program breaks the loop, tears down the UI, and ends the
/// program.
///
/// *Sync subcommand:*
/// Connects to the sqlite database, then initiates a full sync of all
/// podcasts. No UI is created for this, as the intention is to be used
/// in a programmatic way (e.g., setting up a cron job to sync
/// regularly.)
///
/// *Import subcommand:*
/// Reads in an OPML file and adds feeds to the database that do not
/// already exist. If the `-r` option is used, the database is wiped
/// first.
///
/// *Export subcommand:*
/// Connects to the sqlite database, and reads all podcasts into an OPML
/// file, with the location specified from the command line arguments.
fn main() -> Result<()>
{
	// SETUP -----------------------------------------------------------

	// set up the possible command line arguments and subcommands
	let args = Command::new(clap::crate_name!())
		.version(clap::crate_version!())
		// .author(clap::crate_authors!(", "))
		.author(
			"2020-2023 Jeff Hughes <jeff.hughes at gmail dot com>\n\
			 2024      alpou       <alpou at tutanota dot com>"
		)
		.about(clap::crate_description!())
		.arg(Arg::new("config")
			.short('c')
			.long("config")
			.env("SHELLCASTER_CONFIG")
			.global(true)
			.takes_value(true)
			.value_name("FILE")
			.help(
				"Sets a custom config file location. Can also be set with environment variable."
			)
		)
		.arg(Arg::new("short-filename")
			.short('s')
			.long("short-filename")
			.global(true)
			.help(
				"Truncates filenames to 140 characters to stay under Windows path length limits."
			)
		)
		.subcommand(Command::new("sync")
			.about("Syncs all podcasts in database")
			.arg(Arg::new("quiet")
				.short('q')
				.long("quiet")
				.help("Suppresses output messages to stdout.")))
		.subcommand(Command::new("import")
			.about("Imports podcasts from an OPML file")
			.arg(Arg::new("file")
				.short('f')
				.long("file")
				.takes_value(true)
				.value_name("FILE")
				.help(
					"Specifies the filepath to the OPML file to be imported. If this flag is not set, the command will read from stdin."
				)
			)
			.arg(Arg::new("replace")
				.short('r')
				.long("replace")
				.takes_value(false)
				.help(
					"If set, the contents of the OPML file will replace all existing data in the shellcaster database."
				)
			)
			.arg(Arg::new("quiet")
				.short('q')
				.long("quiet")
				.help("Suppresses output messages to stdout.")))
		.subcommand(Command::new("export")
			.about("Exports podcasts to an OPML file")
			.arg(Arg::new("file")
				.short('f')
				.long("file")
				.takes_value(true)
				.value_name("FILE")
				.help(
					"Specifies the filepath for where the OPML file will be exported. If this flag is not set, the command will print to stdout."
				)
			)
		)
		.get_matches();

	// figure out where config file is located -- either specified from
	// command line args, set via $SHELLCASTER_CONFIG, or using default
	// config location for OS
	let config_path = get_config_path(args.value_of("config"))
		.unwrap_or_else(|| {
			eprintln!(
				"Could not identify your operating system's default directory to store configuration files. Please specify paths manually using config.toml and use `-c` or `--config` flag to specify where config.toml is located when launching the program."
			);
			process::exit(1);
		});
	let mut config = Config::new(&config_path)?;
	config.short_filename = args.is_present("short-filename");

	let mut db_path = config_path;
	if !db_path.pop()
	{
		return Err(anyhow!(
			"Could not correctly parse the config file location. Please specify a valid path to the config file."
		));
	}


	return match args.subcommand()
	{
		// SYNC SUBCOMMAND ----------------------------------------------
		Some(("sync", sub_args)) => sync_podcasts(&db_path, config, sub_args),

		// IMPORT SUBCOMMAND --------------------------------------------
		Some(("import", sub_args)) => import(&db_path, config, sub_args),

		// EXPORT SUBCOMMAND --------------------------------------------
		Some(("export", sub_args)) => export(&db_path, sub_args),

		// MAIN COMMAND -------------------------------------------------
		_ => {
			let mut main_ctrl = MainController::new(config, &db_path)?;

			main_ctrl.loop_msgs(); // main loop

			main_ctrl.tx_to_ui.send(MainMessage::UiTearDown).unwrap();
			// wait for UI thread to finish teardown
			main_ctrl.ui_thread.join().unwrap();
			Ok(())
		}
	};
}


/// Gets the path to the config file if one is specified in the command-
/// line arguments, or else returns the default config path for the
/// user's operating system.
/// Returns None if default OS config directory cannot be determined.
///
/// Note: Right now we only have one possible command-line argument,
/// specifying a config path. If the command-line API is
/// extended in the future, this will have to be refactored.
fn get_config_path(config: Option<&str>) -> Option<PathBuf>
{
	return match config
	{
		Some(path) => Some(PathBuf::from(path)),
		None => {
			let default_config = dirs::config_dir();
			match default_config
			{
				Some(mut path) => {
					path.push("shellcaster");
					path.push("config.toml");
					Some(path)
				}
				None => None,
			}
		}
	};
}


/// Synchronizes RSS feed data for all podcasts, without setting up a UI.
fn sync_podcasts(
	db_path: &Path,
	config: Config,
	args: &clap::ArgMatches
) -> Result<()>
{
	let db_inst = Database::connect(db_path)?;
	let podcast_list = db_inst.get_podcasts()?;

	if podcast_list.is_empty()
	{
		if !args.is_present("quiet")
		{
			println!("No podcasts to sync.");
		}
		return Ok(());
	}

	let threadpool = Threadpool::new(config.simultaneous_downloads);
	let (tx_to_main, rx_to_main) = mpsc::channel();

	for pod in podcast_list.iter()
	{
		let feed = PodcastFeed::new(
			Some(pod.id),
			pod.url.clone(),
			Some(pod.title.clone())
		);
		feeds::check_feed(
			feed,
			config.max_retries,
			&threadpool,
			tx_to_main.clone()
		);
	}

	let mut msg_counter: usize = 0;
	let mut failure = false;
	while let Some(message) = rx_to_main.iter().next()
	{
		match message
		{
			Message::Feed(FeedMsg::SyncData((pod_id, pod))) => {
				let title = pod.title.clone();
				let db_result = db_inst.update_podcast(pod_id, pod);
				match db_result
				{
					Ok(_) => {
						if !args.is_present("quiet")
						{
							println!("Synced {title}");
						}
					}
					Err(_err) => {
						failure = true;
						eprintln!("Error synchronizing {title}");
					}
				}
			}

			Message::Feed(FeedMsg::Error(feed)) => {
				failure = true;
				match feed.title
				{
					Some(t) => eprintln!("Error retrieving RSS feed for {}.", t),
					None => eprintln!("Error retrieving RSS feed."),
				}
			}
			_ => (),
		}

		msg_counter += 1;
		if msg_counter >= podcast_list.len()
		{
			break;
		}
	}

	if failure
	{
		return Err(anyhow!("Process finished with errors."));
	}
	else if !args.is_present("quiet")
	{
		println!("Sync successful.");
	}
	return Ok(());
}


/// Imports a list of podcasts from OPML format, either reading from a
/// file or from stdin. If the `replace` flag is set, this replaces all
/// existing data in the database.
fn import(
	db_path: &Path,
	config: Config,
	args: &clap::ArgMatches
) -> Result<()>
{
	// read from file or from stdin
	let xml = match args.value_of("file")
	{
		Some(filepath) => {
			let mut f = File::open(filepath)
				.with_context(|| format!(
					"Could not open OPML file: {filepath}"
				))?;
			let mut contents = String::new();
			f.read_to_string(&mut contents)
				.with_context(|| format!(
					"Failed to read from OPML file: {filepath}"
				))?;
			contents
		}
		None => {
			let mut contents = String::new();
			std::io::stdin()
				.read_to_string(&mut contents)
				.with_context(|| "Failed to read OPML file from stdin")?;
			contents
		}
	};

	let mut podcast_list = opml::import(xml).with_context(|| {
		"Could not properly parse OPML file -- file may be formatted improperly or corrupted."
	})?;

	if podcast_list.is_empty()
	{
		if !args.is_present("quiet")
		{
			println!("No podcasts to import.");
		}
		return Ok(());
	}

	let db_inst = Database::connect(db_path)?;

	// delete database if we are replacing the data
	if args.is_present("replace")
	{
		db_inst
			.clear_db()
			.with_context(|| "Error clearing database")?;
	}
	else
	{
		let old_podcasts = db_inst.get_podcasts()?;

		// if URL is already in database, remove it from import
		podcast_list = podcast_list
			.into_iter()
			.filter(|pod| {
				for op in &old_podcasts
				{
					if pod.url == op.url
					{
						return false;
					}
				}
				return true;
			})
			.collect();
	}

	// check again, now that we may have removed feeds after looking at
	// the database
	if podcast_list.is_empty()
	{
		if !args.is_present("quiet")
		{
			println!("No podcasts to import.");
		}
		return Ok(());
	}

	println!("Importing {} podcasts...", podcast_list.len());

	let threadpool = Threadpool::new(config.simultaneous_downloads);
	let (tx_to_main, rx_to_main) = mpsc::channel();

	for pod in podcast_list.iter()
	{
		feeds::check_feed(
			pod.clone(),
			config.max_retries,
			&threadpool,
			tx_to_main.clone(),
		);
	}

	let mut msg_counter: usize = 0;
	let mut failure = false;
	while let Some(message) = rx_to_main.iter().next()
	{
		match message
		{
			Message::Feed(FeedMsg::NewData(pod)) => {
				let title = pod.title.clone();
				let db_result = db_inst.insert_podcast(pod);
				match db_result
				{
					Ok(_) => {
						if !args.is_present("quiet")
						{
							println!("Added {title}");
						}
					}
					Err(_err) => {
						failure = true;
						eprintln!("Error adding {title}");
					}
				}
			}

			Message::Feed(FeedMsg::Error(feed)) => {
				failure = true;
				if let Some(t) = feed.title
				{
					eprintln!("Error retrieving RSS feed: {t}");
				}
				else
				{
					eprintln!("Error retrieving RSS feed");
				}
			}
			_ => (),
		}

		msg_counter += 1;
		if msg_counter >= podcast_list.len()
		{
			break;
		}
	}

	if failure
	{
		return Err(anyhow!("Process finished with errors."));
	}
	else if !args.is_present("quiet")
	{
		println!("Import successful.");
	}
	return Ok(());
}


/// Exports all podcasts to OPML format, either printing to stdout or
/// exporting to a file.
fn export(db_path: &Path, args: &clap::ArgMatches) -> Result<()> {
	let db_inst = Database::connect(db_path)?;
	let podcast_list = db_inst.get_podcasts()?;
	let opml = opml::export(podcast_list);

	let xml = opml
		.to_string()
		.map_err(|err| anyhow!(err))
		.with_context(|| "Could not create OPML format")?;

	match args.value_of("file")
	{
		// export to file
		Some(file) => {
			let mut dst = File::create(file)
				.with_context(|| format!(
					"Could not create output file: {file}"
				))?;
			dst.write_all(xml.as_bytes())
				.with_context(|| format!(
					"Could not copy OPML data to output file: {file}"
				))?;
		}
		// print to stdout
		None => println!("{xml}"),
	}
	return Ok(());
}
