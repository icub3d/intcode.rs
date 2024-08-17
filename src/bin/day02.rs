use anyhow::Result;
use intcode::{ipc::Channel, process::Process};

#[tokio::main]
async fn main() -> Result<()> {
    let input = include_str!("inputs/day02");

    // For part 1, we can simply run the program with the two given inputs.
    let (_, sender, receiver) = Channel::new(true);
    let mut process = Process::new(input, receiver, sender.clone());
    process.set_memory(1, 12);
    process.set_memory(2, 2);
    process.run().await?;
    println!("p1: {}", process.state()[0]);

    // For part 2, we are looking for a specific output. The numbers are small enough to brute
    // force, so we just look for the correct output.
    'outer: for noun in 0..=99 {
        for verb in 0..=99 {
            let (_, sender, receiver) = Channel::new(true);
            let mut process = Process::new(input, receiver, sender.clone());
            process.set_memory(1, noun);
            process.set_memory(2, verb);
            process.run().await?;
            if process.state()[0] == 19_690_720 {
                println!("p2: {}", 100 * noun + verb);
                break 'outer;
            }
        }
    }

    Ok(())
}
