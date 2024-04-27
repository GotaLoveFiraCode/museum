#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui;

use crate::song::Song;
use color_eyre::eyre::{bail, Result};
use log::{error, info, warn};
use owo_colors::OwoColorize;
use rodio::{Decoder, OutputStream, Sink};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use std::fs::File;
use std::io::BufReader;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, sleep};

enum UserCommands {
    Pause,
    Skip,
    Stop,
    Unrecognized,
    Refresh,
    Error(color_eyre::eyre::Error),
    Msg(String),
}

struct Content {
    // SEND updated songs to other thread.
    tx_songs: mpsc::Sender<Vec<Song>>,
    // RECEIVE info.
    rx: mpsc::Receiver<UserCommands>,
    // SEND info.
    tx: mpsc::Sender<UserCommands>,
    // RECEIVE sink. USE sink.
    sink: Arc<Sink>,
    // SHARE new song info.
    new_song_info: Arc<Mutex<Vec<Song>>>,
    // CHECK other thread status.
    music_handle: thread::JoinHandle<()>,
    input: String,
    quit: mpsc::Sender<bool>,
}

pub fn play_queue_with_gui(queue: &[Song]) -> Result<Vec<Song>> {
    info!("Displaying GUI…");
    let options = eframe::NativeOptions::default();
    let music_queue = queue.to_vec();

    // This channel is for returning the updated list of songs.
    let (tx_songs, rx_songs) = mpsc::channel();

    // Used for communication with the GUI.
    let (tx, rx) = mpsc::channel();
    let (tx_quit, rx_quit) = mpsc::channel();
    // So the music playing thread can send info.
    let tx_music = tx.clone();

    // Control audio.
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Arc::new(Sink::try_new(&stream_handle).unwrap());
    let sink_music = Arc::clone(&sink);

    // SHARED new info.
    let updated_info: Arc<Mutex<Vec<Song>>> = Arc::new(Mutex::new(Vec::new()));
    let updated_info_music = Arc::clone(&updated_info);

    let music_handle = thread::spawn(move || {
        sleep(std::time::Duration::from_millis(500));

        let mut ip: u8 = 0;
        let queue_len = music_queue.len();

        for song in music_queue {
            ip += 1;
            match tx_music.send(UserCommands::Msg(format!(
                "[{ip}/{queue_len}] Now playing \"{}\"",
                song.path
            ))) {
                Ok(()) => {}
                Err(_) => {
                    return;
                }
            };

            info!("[{ip}/{queue_len}] Now playing \"{}\"", song.path);

            let file = BufReader::new(File::open(&song.path).unwrap());
            let source = Decoder::new(file).unwrap();

            // Add song to return with +1 touches.
            // This is done __before__ the song starts playing.
            // If the User skips, it will be changed.
            let mut updated_info = updated_info_music.lock().unwrap();
            updated_info.push(Song {
                id: song.id,
                path: song.path.clone(),
                touches: song.touches + 1,
                skips: song.skips,
                loved: song.loved,
                score: None,
            });
            // So it can be used by other threads (to add skips).
            drop(updated_info);

            sink_music.append(source);
            // returns early when `skip_one` is called.
            sink_music.sleep_until_end();
        }

        tx_music
            .send(UserCommands::Msg(
                "Played all songs. Please enter 'quit.'".to_owned(),
            ))
            .unwrap();
        tx_music.send(UserCommands::Stop).unwrap();
    });

    let new_song_info = Arc::clone(&updated_info);
    let condition = eframe::run_native(
        "Muse: Unleashing Music",
        options,
        Box::new(move |_cc| {
            let content = Content {
                tx_songs,
                rx,
                tx,
                sink,
                new_song_info,
                music_handle,
                input: String::new(),
                quit: tx_quit,
            };
            Box::new(content)
        }),
    );

    if condition.is_err() {
        error!("Failed to use EGUI");
        bail!("Failed to use EGUI");
    };

    loop {
        if let Ok(bool) = rx_quit.recv() {
            if bool {
                break;
            }
        }
    }

    let updated_info = rx_songs.try_recv().unwrap();

    // let updated_info = match rx_songs.try_recv() {
    //     Ok(songs) => songs,
    //     Err(err) => match err {
    //         mpsc::TryRecvError::Empty => {
    //             error!("No music to update!");
    //             bail!("No music to update!");
    //         }
    //         mpsc::TryRecvError::Disconnected => {
    //             warn!("Please quit the window (with the `quit` command), instead of just closing it! No music saved… I will fix this soon.");
    //             bail!("Please quit the window (with the `quit` command), instead of just closing it! I will fix this soon.");
    //         }
    //     }
    // };

    // TODO: don't use unwrap!!!
    log::debug!("Found: {:?}", updated_info.first().unwrap());
    log::debug!("{updated_info:?}");
    info!("Closed GUI.");

    Ok(updated_info)
}

impl eframe::App for Content {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Muse: Unleashing Music.");

            let command_label = ui.label("Command (e.g. help): ");
            let response = ui
                .text_edit_singleline(&mut self.input)
                .labelled_by(command_label.id);

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                parse_line(Ok(self.input.clone()), &self.tx);
            }

            let what_to_do = match self.rx.try_recv() {
                Ok(t) => t,
                Err(err) => match err {
                    mpsc::TryRecvError::Empty => return,
                    mpsc::TryRecvError::Disconnected => panic!("Channel disconnected."),
                },
            };

            match what_to_do {
                UserCommands::Refresh => {}
                UserCommands::Pause => {
                    if self.sink.is_paused() {
                        self.sink.play();
                    } else {
                        info!("{}…", "Pausing song".yellow());
                        self.sink.pause();
                    }
                }
                UserCommands::Skip => {
                    info!("{}…", "Skipping song".yellow());
                    let mut updated_info = self.new_song_info.lock().unwrap();
                    if let Some(last) = updated_info.last_mut() {
                        // Change the already added song to include the skip.
                        last.skips += 1;
                    }
                    drop(updated_info);
                    self.sink.skip_one();
                }

                UserCommands::Stop => {
                    info!("Stopping…");
                    let updated_info = self.new_song_info.lock().unwrap();
                    self.tx_songs.send(updated_info.to_vec()).unwrap();
                    self.quit.send(true).unwrap();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }

                UserCommands::Unrecognized => {
                    warn!(
                        "{} {}",
                        "Unrecognized command.".red(),
                        "Try 'pause', 'skip', or 'quit'".italic()
                    );
                }
                UserCommands::Msg(msg) => {
                    info!("{msg}");
                    ui.heading(msg);
                }
                UserCommands::Error(err) => {
                    // color_eyre::eyre::bail!("Error: {}", err);
                    log::error!("{}", err.to_string());
                }
            }

            if self.music_handle.is_finished() {
                let updated_info = self.new_song_info.lock().unwrap();
                self.tx_songs.send(updated_info.to_vec()).unwrap();
            }
        });
    }
}

/// Plays the inputed queue with user interaction.
/// Returns the same queue with updated information.
///
/// SO FUCKING BROKEN, PLS FIXME!
///
/// maybe use crossterm?
/// or daemon?
/// or just give up?
/// IDK!?
/// FUCK YOU!?
pub fn play_queue_with_cmds(queue: &[Song]) -> Result<Vec<Song>> {
    let queue = queue.to_vec();

    let (tx, rx) = mpsc::channel();
    let tx_music = tx.clone();

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Arc::new(Sink::try_new(&stream_handle)?);
    let sink_music = Arc::clone(&sink);

    let updated_info: Arc<Mutex<Vec<Song>>> = Arc::new(Mutex::new(Vec::new()));
    let updated_info_music = Arc::clone(&updated_info);

    let music_handle = thread::spawn(move || {
        let mut ip: u8 = 0;
        let queue_len = queue.len();

        for song in queue {
            ip += 1;
            // You have to send the message so the prompt doesn’t break.
            tx_music
                .send(UserCommands::Msg(format!(
                    "[{ip}/{queue_len}] Now playing \"{}\"",
                    song.path.blue()
                )))
                .unwrap();

            let file = BufReader::new(File::open(&song.path).unwrap());
            let source = Decoder::new(file).unwrap();

            // Add song to return with +1 touches.
            // Calc score when INSERTing into DB.
            let mut updated_info = updated_info_music.lock().unwrap();
            updated_info.push(Song {
                id: song.id,
                path: song.path.clone(),
                touches: song.touches + 1,
                skips: song.skips,
                loved: song.loved,
                score: None,
            });
            // So it can be used by other thread (to add skips).
            drop(updated_info);

            sink_music.append(source);
            // returns early when `skip_one` is called.
            sink_music.sleep_until_end();
        }

        tx_music
            .send(UserCommands::Msg(format!(
                "{} Please enter 'quit.'",
                "Played all songs.".green()
            )))
            .unwrap();
        // tx_music.send(UserCommands::Stop).unwrap();
    });

    let mut rl = DefaultEditor::new()?;
    // let mut quit = false;
    loop {
        let readline = rl.readline(">> ");

        parse_line(readline, &tx);

        match rx.recv().unwrap() {
            UserCommands::Refresh => {
                continue;
            }
            UserCommands::Pause => {
                if sink.is_paused() {
                    sink.play();
                } else {
                    info!("{}…", "Pausing song".yellow());
                    sink.pause();
                }
            }
            UserCommands::Skip => {
                info!("{}…", "Skipping song".yellow());
                let mut updated_info = updated_info.lock().unwrap();
                if let Some(last) = updated_info.last_mut() {
                    // Change the already added song to include the skip.
                    last.skips += 1;
                }
                drop(updated_info);
                sink.skip_one();
            }
            UserCommands::Stop => {
                info!("{}…", "Stopping".red());
                break;
            }
            UserCommands::Unrecognized => {
                warn!(
                    "{} {}",
                    "Unrecognized command.".red(),
                    "Try 'pause', 'skip', or 'quit'".italic()
                );
            }
            UserCommands::Msg(msg) => {
                info!("{msg}");
            }
            UserCommands::Error(err) => {
                color_eyre::eyre::bail!("Error: {}", err);
            }
        }

        // Redundant, but I’m paranoid.
        if music_handle.is_finished() {
            break;
        }
    }

    let updated_info = updated_info.lock().unwrap();
    Ok(updated_info.to_vec())
}

fn parse_line(readline: Result<String, ReadlineError>, tx: &mpsc::Sender<UserCommands>) {
    match readline {
        Ok(cmd) => match cmd.as_str() {
            "pause" => {
                tx.send(UserCommands::Pause).unwrap();
            }
            "skip" => {
                tx.send(UserCommands::Skip).unwrap();
            }
            "stop" | "quit" | "exit" => {
                tx.send(UserCommands::Stop).unwrap();
            }
            "refresh" => {
                tx.send(UserCommands::Refresh).unwrap();
            }
            _ => {
                tx.send(UserCommands::Unrecognized).unwrap();
            }
        },
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
            tx.send(UserCommands::Stop).unwrap();
        }
        Err(err) => {
            tx.send(UserCommands::Error(err.into())).unwrap();
        }
    }
}
