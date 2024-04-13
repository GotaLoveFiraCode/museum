use crate::song::Song;
use color_eyre::eyre::Result;
use owo_colors::OwoColorize;
use rodio::{Decoder, OutputStream, Sink};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use crossterm::cursor;
use crossterm::terminal;

use std::fs::File;
use std::io::BufReader;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

enum UserCommands {
    Pause,
    Skip,
    Stop,
    Unrecognized,
    Error(color_eyre::eyre::Error),
    Msg(String),
}

/// Plays the inputed queue with user interaction.
/// Returns the same queue with updated information.
/// 
/// SO FUCKING BROKEN PLS FIXME!
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
            tx_music.send(UserCommands::Msg(format!(
                "==> [{ip}/{queue_len}] Now playing \"{}\"",
                song.path.blue()
            ))).unwrap();

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
                score: None,
            });
            // So it can be used by other thread (to add skips).
            drop(updated_info);

            sink_music.append(source);
            // returns early when `skip_one` is called.
            sink_music.sleep_until_end();
        }

        tx_music.send(UserCommands::Msg(format!("==> {} Please enter 'quit.'",
            "Played all songs.".green()))).unwrap();
        // tx_music.send(UserCommands::Stop).unwrap();
    });

    let mut rl = DefaultEditor::new()?;
    // let mut quit = false;
    loop {
        let readline = rl.readline(">> ");

        parse_line(readline, &tx);

        match rx.recv().unwrap() {
            UserCommands::Pause => {
                if sink.is_paused() {
                    sink.play();
                } else {
                    println!("==> {}…", "Pausing song".yellow());
                    sink.pause();
                }
            }
            UserCommands::Skip => {
                println!("==> {}…", "Skipping song".yellow());
                let mut updated_info = updated_info.lock().unwrap();
                if let Some(last) = updated_info.last_mut() {
                    // Change the already added song to include the skip.
                    last.skips += 1;
                }
                drop(updated_info);
                sink.skip_one();
            }
            UserCommands::Stop => {
                println!("==> {}…", "Stopping".red());
                break;
            }
            UserCommands::Unrecognized => {
                println!(
                    "==> {} {}",
                    "Unrecognized command.".red(),
                    "Try 'pause', 'skip', or 'quit'".italic()
                );
            }
            UserCommands::Msg(msg) => {
                println!("{msg}");
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
        Ok(cmd) => {
            match cmd.as_str() {
                "pause" => {
                    tx.send(UserCommands::Pause).unwrap();
                }
                "skip" => {
                    tx.send(UserCommands::Skip).unwrap();
                }
                "stop" | "quit" | "exit" => {
                    tx.send(UserCommands::Stop).unwrap();
                }
                _ => {
                    tx.send(UserCommands::Unrecognized).unwrap();
                }
            }
        }
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
            tx.send(UserCommands::Stop).unwrap();
        }
        Err(err) => {
            tx.send(UserCommands::Error(err.into())).unwrap();
        }
    }
}




// fn play(queue: &[Song]) -> Result<Vec<Song>> {
//     let queue = queue.to_vec();
//     let (input_tx, input_rx) = mpsc::channel();
//     let (output_tx, output_rx) = mpsc::channel();
//     let tx_music = input_tx.clone();
// }


