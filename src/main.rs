#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use kira::clock::ClockSpeed;
use kira::sound::static_sound::StaticSoundHandle;
use kira::Tween;
use kira::{
	AudioManager, AudioManagerSettings, DefaultBackend,
	sound::static_sound::StaticSoundData
};
use std::collections::HashMap;
use std::thread;
use std::sync::mpsc;
use std::fs;
use std::time::Duration;

enum AudioAction {
	Play {
		file: String,
		reversed: bool,
		delay: f64,
	},
	Stop {
		file: String
	},
	StopAll
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
		let mut active: HashMap<String, Vec<StaticSoundHandle>> = HashMap::new();
		let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

		let mut clock_handle = manager.add_clock(ClockSpeed::TicksPerSecond(10.0)).unwrap();
		clock_handle.start();

		loop {
			let action: AudioAction = rx.recv().unwrap();
			match action {
				AudioAction::Play { file, reversed, delay } => {
					if let Some(sound) = cache.get(&file) {
						let sound = if delay == 0.0 {
							sound.clone()
						} else {
							sound.start_time(clock_handle.time() + delay*10.0)
						};

						let handle = if reversed {
							manager.play(sound.reverse(true)).unwrap()
						} else {
							manager.play(sound.clone()).unwrap()
						};

						if !active.contains_key(&file) {
							active.insert(file.clone(), vec![]);
						}
						active.get_mut(&file).unwrap().push(handle);
					} else {
						let path = String::from("Audios/") + &file;
						let sound = StaticSoundData::from_file(path).unwrap();

						let sound = if delay == 0.0 {
							sound.clone()
						} else {
							sound.start_time(clock_handle.time() + delay*10.0)
						};

						let handle = if reversed {
							manager.play(sound.reverse(true)).unwrap()
						} else {
							manager.play(sound.clone()).unwrap()
						};
						
						if !active.contains_key(&file) {
							active.insert(file.clone(), vec![]);
						}
						active.get_mut(&file).unwrap().push(handle);
						cache.insert(file, sound);
					}
				}

				AudioAction::Stop { file } => {
					if let Some(handle_list) = active.get_mut(&file) {
						for handle in handle_list {
							handle.stop(Tween {
								duration: Duration::from_secs(0),
								..Default::default()
							});
						}
						active.insert(file, vec![]);
					}
				}

				AudioAction::StopAll => {
					for (_file, handle_list) in &mut active {
						for handle in handle_list {
							handle.stop(Tween {
								duration: Duration::from_secs(0),
								..Default::default()
							});
						}
					}
					active.clear();
				}
			}
			
		}
	});
	eframe::run_native(
		"soundboard-rs",
		options,
		Box::new(|cc| {
			egui_material_icons::initialize(&cc.egui_ctx);
			Ok(Soundboard::new(tx, audio_files))
		}),
	)
}

struct Soundboard {
	transmitter: mpsc::Sender<AudioAction>,
	audio_files: Vec<String>,
	add_delay: bool,
	delay: f64
}

impl eframe::App for Soundboard {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		let window_dimensions = ctx.input(|i| i.viewport().outer_rect);
		if let None = window_dimensions {
			return;
		};
		let window_dimensions = window_dimensions.unwrap();

		egui::CentralPanel::default().show(ctx, |ui| {
			ui.heading("Soundboard");
			ui.checkbox(&mut self.add_delay, "Delay");
			if self.add_delay {
				ui.add(egui::Slider::new(&mut self.delay, 0.0..=10.0).text("Delay Length (seconds)"));
			}
			if ui.button("Stop All").clicked() {
				self.transmitter.send(AudioAction::StopAll).unwrap();
			}
			egui::ScrollArea::vertical().show(ui, |ui| {
				ui.set_min_width(window_dimensions.width());
				ui.set_max_width(window_dimensions.width());

				let ui_builder = egui::UiBuilder::new();

				ui.scope_builder(ui_builder, |ui| {
					egui::Grid::new("sounds")
						.num_columns(4)
						.spacing([4.0, 4.0])
						.striped(true)
						.show(ui, |ui| {
							for file in &self.audio_files {
								ui.label(file);
								if ui.button(egui_material_icons::icons::ICON_PLAY_ARROW).clicked() {
									self.transmitter.send(AudioAction::Play { file: file.to_owned(), reversed: false, delay: if self.add_delay { self.delay } else { 0.0 } }).unwrap();
								}
								if ui.button(egui_material_icons::icons::ICON_FAST_REWIND).clicked() {
									self.transmitter.send(AudioAction::Play { file: file.to_owned(), reversed: true, delay: if self.add_delay { self.delay } else { 0.0 } }).unwrap();
								}
								if ui.button(egui_material_icons::icons::ICON_STOP).clicked() {
									self.transmitter.send(AudioAction::Stop { file: file.to_owned() }).unwrap();
								}
								ui.end_row();
							}
						});
				});
			});
		});
	}
}

impl Soundboard {
	fn new(transmitter: mpsc::Sender<AudioAction>, audio_files: Vec<String>) -> Box<Self> {
		Box::new(Self { transmitter, audio_files, add_delay: false, delay: 0.0 })
	}
}