use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sanitize_filename::{sanitize_with_options, Options};

use crate::threadpool::Threadpool;
use crate::types::Message;

/// Enum used for communicating back to the main controller upon
/// successful or unsuccessful downloading of a file. i32 value
/// represents the episode ID, and PathBuf the location of the new file.
#[derive(Debug)]
pub enum DownloadMsg
{
	Complete(EpData),
	ResponseError(EpData),
	FileCreateError(EpData),
	FileWriteError(EpData),
}

/// Enum used to communicate relevant data to the threadpool.
#[derive(Debug, Clone)]
pub struct EpData
{
	pub id: i64,
	pub pod_id: i64,
	pub title: String,
	pub url: String,
	pub pubdate: Option<DateTime<Utc>>,
	pub file_path: Option<PathBuf>,
}

/// This is the function the main controller uses to indicate new
/// files to download. It uses the threadpool to start jobs
/// for every episode to be downloaded. New jobs can be requested
/// by the user while there are still ongoing jobs.
pub fn download_list(
	episodes: Vec<EpData>,
	dest: &Path,
	max_retries: usize,
	filename_prefix: &str,
	filename_suffix: &str,
	threadpool: &Threadpool,
	tx_to_main: Sender<Message>,
) {
	// parse episode details and push to queue
	for ep in episodes.into_iter()
	{
		let tx = tx_to_main.clone();
		let dest2 = dest.to_path_buf();
		let prefix = filename_prefix.to_owned();
		let suffix = filename_suffix.to_owned();
		threadpool.execute(move || {
			let result = download_file(ep, dest2, max_retries, prefix, suffix);
			tx.send(Message::Dl(result))
				.expect("Thread messaging error");
		});
	}
}


/// Downloads a file to a local filepath, returning DownloadMsg variant
/// indicating success or failure.
fn download_file(
	mut ep_data: EpData,
	dest: PathBuf,
	mut max_retries: usize,
	filename_prefix: String,
	filename_suffix: String,
) -> DownloadMsg
{
	let agent_builder = ureq::builder()
		.timeout_connect(Duration::from_secs(10))
		.redirects(10);

	#[cfg(feature = "native_tls")]
	let tls_connector = std::sync::Arc::new(native_tls::TlsConnector::new().unwrap());
	#[cfg(feature = "native_tls")]
	let agent_builder = agent_builder.tls_connector(tls_connector);
	let agent = agent_builder.build();

	let request: Result<ureq::Response, ()> = loop
	{
		let response = agent.get(&ep_data.url).call();
		match response
		{
			Ok(resp) => break Ok(resp),
			Err(_) => {
				max_retries -= 1;
				if max_retries == 0 {
					break Err(());
				}
			}
		}
	};

	if request.is_err()
	{
		return DownloadMsg::ResponseError(ep_data);
	};

	let response = request.unwrap();

	// figure out the file type
	let ext = match response.header("content-type")
	{
		Some("audio/3gpp")   => "3gp",
		Some("audio/aac")    => "aac",
		Some("audio/x-m4a")  => "m4a",
		Some("audio/midi")   => "mid",
		Some("audio/x-midi") => "mid",
		Some("audio/mpeg")   => "mp3",
		Some("audio/ogg")    => "oga",
		Some("audio/opus")   => "opus",
		Some("audio/wav")    => "wav",
		Some("audio/webm")   => "weba",
		Some("video/quicktime") => "mov",
		Some("video/mp4")       => "mp4",
		Some("video/x-m4v")     => "m4v",
		_ => "mp3", // assume .mp3 unless we figure out otherwise
		// TODO why not just take the extension from ep_data.url?
	};

	let mut file_name = sanitize_with_options(&ep_data.title, Options {
		truncate: true,
		// for simplicity, we'll just use Windows-friendly paths for everyone
		windows: true,
		replacement: "",
	});

	if let Some(pubdate) = ep_data.pubdate
	{
		// Note: chrono::DateTime::format panics when its input string contains
		// invalid identifiers. However, there is a check in
		// config::Config::new that prevents the program from running when
		// filename_prefix or filename_suffix is invalid
		// TODO chrono::DateTime::format is deprecated since 0.4.32. It is
		// recommended to use DelayedFormat::fmt instead
		file_name = format!(
			"{}{}{}",
			pubdate.format(&filename_prefix),
			file_name,
			pubdate.format(&filename_suffix)
		);
	}

	let mut file_path = dest;
	file_path.push(format!("{file_name}.{ext}"));

	let dst = File::create(&file_path);
	ep_data.file_path = Some(file_path);
	if dst.is_err()
	{
		return DownloadMsg::FileCreateError(ep_data);
	};

	let mut reader = response.into_reader();
	return match std::io::copy(&mut reader, &mut dst.unwrap())
	{
		Ok(_) => DownloadMsg::Complete(ep_data),
		Err(_) => DownloadMsg::FileWriteError(ep_data),
	};
}
