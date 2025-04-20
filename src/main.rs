#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use kira::{
	AudioManager, AudioManagerSettings, DefaultBackend,
	sound::static_sound::StaticSoundData
};
use std::collections::HashMap;
use std::thread;
use std::sync::mpsc;
use std::fs;

enum AudioAction {
	Play(String),
}

fn main() -> eframe::Result {
	env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
		..Default::default()
	};
	if !fs::exists("./Audios").unwrap() || !fs::metadata("./Audios").unwrap().is_dir() {
		fs::create_dir("./Audios").unwrap();
	}

	let mut audio_files: Vec<String> = Vec::new();
	let paths = fs::read_dir("./Audios").unwrap();

	for path in paths {
		audio_files.push(path.unwrap().file_name().into_string().unwrap());
	}

	let (tx, rx) = mpsc::channel();
	thread::spawn(move || {
		let mut cache: HashMap<String, StaticSoundData> = HashMap::new();
		let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

		loop {
			let action: AudioAction = rx.recv().unwrap();
			match action {
				AudioAction::Play(name) => {
					if let Some(sound) = cache.get(&name) {
						manager.play(sound.clone()).unwrap();
					} else {
						let path = String::from("Audios/") + &name;
						let sound_data = StaticSoundData::from_file(path).unwrap();
						manager.play(sound_data.clone()).unwrap();
						cache.insert(name, sound_data);
					}
				}
			}
			
		}
	});
	eframe::run_native(
		"soundboard-rs",
		options,
		Box::new(|_cc| {
			Ok(Soundboard::new(tx, audio_files))
		}),
	)
}

struct Soundboard {
	transmitter: mpsc::Sender<AudioAction>,
	audio_files: Vec<String>
}

impl Soundboard {
	fn new(transmitter: mpsc::Sender<AudioAction>, audio_files: Vec<String>) -> Box<Self> {
		Box::new(Self { transmitter, audio_files })
	}
}

impl eframe::App for Soundboard {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {
			ui.heading("Soundboard");
			egui::ScrollArea::vertical().show(ui, |ui| {
				for file in &self.audio_files {
					if ui.button(file).clicked() {
						self.transmitter.send(AudioAction::Play(file.to_owned())).unwrap();
					}
				}
			});
		});
	}
}