use crate::song::Song;
use color_eyre::eyre::Result;
use owo_colors::OwoColorize;
use rodio::{Decoder, OutputStream, Sink};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

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
}

/// Plays the inputed queue with user interaction.
/// Returns the same queue with updated information.
/// Function is too long, but I can’t refactor becaues of threads…?
pub fn play_queue_with_cmds(queue: &[Song]) -> Result<Vec<Song>> {
    let (tx, rx) = mpsc::channel();
    let tx_copy = tx.clone();
    let mut rl = DefaultEditor::new()?;

    thread::spawn(move || {
        // let mut input = String::new();
        loop {
            let readline = rl.readline(">> ");
            // input.clear();
            // std::io::stdin().read_line(&mut input).unwrap();
            // match input.trim() {
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
    });

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Arc::new(Sink::try_new(&stream_handle)?);
    let sink_copy = Arc::clone(&sink);

    let updated_info: Arc<Mutex<Vec<Song>>> = Arc::new(Mutex::new(Vec::new()));
    let updated_info_copy = Arc::clone(&updated_info);

    let queue_copy = queue.to_vec();
    let queue_len = queue_copy.len();
    let mut ip: u8 = 0;

    thread::spawn(move || {
        for song in queue_copy {
            ip += 1;
            println!(
                "==> [{}/{}] Now playing \"{}\"",
                ip,
                queue_len,
                song.path.blue()
            );
            // Have to use unwrap in closure…
            let file = BufReader::new(File::open(&song.path).unwrap());
            let source = Decoder::new(file).unwrap();
            // Add song to return with an added `touch`.
            let mut updated_info = updated_info_copy.lock().unwrap();
            updated_info.push(Song {
                id: song.id,
                path: song.path.clone(),
                touches: song.touches + 1,
                skips: song.skips,
                score: None,
            });
            drop(updated_info);
            sink_copy.append(source);
            // returns early when `skip_one` is called.
            sink_copy.sleep_until_end();
        }

        tx_copy.send(UserCommands::Stop).unwrap();
        println!("==> {}", "Played all songs.".green());
    });

    loop {
        match rx.recv().unwrap() {
            UserCommands::Pause => {
                println!("==> {}…", "Pausing song".yellow());
                if sink.is_paused() {
                    sink.play();
                } else {
                    sink.pause();
                }
            }
            UserCommands::Skip => {
                println!("==> {}…", "Skipping song".yellow());
                let mut updated_info = updated_info.lock().unwrap();
                if let Some(last) = updated_info.last_mut() {
                    // Change the already created added song to include the skip.
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
                    "Try 'pause', 'skip', or 'stop'".italic()
                );
            }
            UserCommands::Error(err) => {
                color_eyre::eyre::bail!("Error: {}", err);
            }
        }
    }

    let updated_info = updated_info.lock().unwrap();
    Ok(updated_info.to_vec())
}
