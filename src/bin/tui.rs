use intcode::ipc::Channel;
use intcode::process::Process;
use intcode::{app::App, tui};

use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::{command, Parser, ValueEnum};
use tokio::sync::mpsc::{self};

#[derive(Clone, Copy, Default, ValueEnum)]
#[clap(rename_all = "snake_case")]
enum Day {
    #[default]
    Day2,
    Day5,
    Day7,
    Day9,
}

// Create a flag so we can run the tui.
#[derive(Parser)]
#[command(author, about, version)]
struct Cli {
    #[arg(short, long)]
    day: Day,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let app = match args.day {
        Day::Day2 => day2().await?,
        Day::Day5 => day5().await?,
        Day::Day7 => day7().await?,
        Day::Day9 => day9().await?,
    };

    tui::run(app).await
}

async fn day2() -> Result<App> {
    let input = include_str!("../../inputs/day02");

    let channels = vec![];
    let mut states = Vec::new();
    let mut notifiers = Vec::new();

    let (_, sender, receiver) = Channel::new(true);

    let (notifier, mut notifier_receiver) = mpsc::channel::<()>(32);
    notifiers.push(notifier);

    let mut process = Process::new(input, receiver, sender.clone());
    process.set_memory(1, 12);
    process.set_memory(2, 2);

    let state = Arc::new(Mutex::new(process.state()));
    states.push(state.clone());

    tokio::spawn(async move {
        while notifier_receiver.recv().await.is_some() {
            if process.state().halted {
                break;
            }
            process.step().await.unwrap();
            *state.lock().unwrap() = process.state();
        }
    });

    Ok(App::new(channels, states, notifiers))
}

async fn day5() -> Result<App> {
    let input = include_str!("../../inputs/day05");

    let mut states = Vec::new();
    let mut notifiers = Vec::new();

    let (i, mut input_sender, input_receiver) = Channel::new(true);
    let (o, output_sender, _) = Channel::new(true);
    let channels = vec![i, o];

    let (notifier, mut notifier_receiver) = mpsc::channel::<()>(32);
    notifiers.push(notifier);

    let mut process = Process::new(input, input_receiver, output_sender);
    input_sender.send(5).await?;

    let state = Arc::new(Mutex::new(process.state()));
    states.push(state.clone());

    tokio::spawn(async move {
        while notifier_receiver.recv().await.is_some() {
            if process.state().halted {
                break;
            }
            process.step().await.unwrap();
            *state.lock().unwrap() = process.state();
        }
    });

    Ok(App::new(channels, states, notifiers))
}

async fn day7() -> Result<App> {
    let input = include_str!("../../inputs/day07");
    let permutation = [0, 1, 2, 3, 4];
    let (channel, mut sender, mut receiver) = Channel::new(false);
    let first = sender.clone();

    let mut channels = vec![channel];
    let mut states = Vec::new();
    let mut notifiers = Vec::new();

    for (i, p) in permutation.iter().enumerate() {
        sender.send(*p as isize + 5).await?;
        if i == 0 {
            sender.send(0).await?;
        }
        let (channel, new_sender, new_receiver) = Channel::new(false);
        let new_sender = if i == 4 { first.clone() } else { new_sender };
        if i != 4 {
            channels.push(channel);
        }

        let mut process = Process::new(input, receiver, new_sender.clone());
        let state = Arc::new(Mutex::new(process.state()));
        states.push(state.clone());

        let (notifier, mut notifier_receiver) = mpsc::channel::<()>(32);
        notifiers.push(notifier);

        tokio::spawn(async move {
            while notifier_receiver.recv().await.is_some() {
                if process.state().halted {
                    break;
                }
                process.step().await.unwrap();
                *state.lock().unwrap() = process.state();
            }
        });
        (sender, receiver) = (new_sender, new_receiver);
    }

    Ok(App::new(channels, states, notifiers))
}

async fn day9() -> Result<App> {
    let input = include_str!("../../inputs/day09");

    let (i, mut tx, rx) = Channel::new(true);
    let (o, tx2, _) = Channel::new(true);
    tx.send(2).await.unwrap();
    let channels = vec![i, o];

    let mut process = Process::new(input, rx, tx2);
    let state = Arc::new(Mutex::new(process.state()));
    let states = vec![state.clone()];

    let (notifier, mut notifier_receiver) = mpsc::channel::<()>(32);
    let notifiers = vec![notifier];

    tokio::spawn(async move {
        while notifier_receiver.recv().await.is_some() {
            if process.state().halted {
                break;
            }
            process.step().await.unwrap();
            *state.lock().unwrap() = process.state();
        }
    });

    Ok(App::new(channels, states, notifiers))
}
