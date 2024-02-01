use anyhow::Result;
use intcode::{ipc::Channel, process::Process};

#[tokio::main]
async fn main() -> Result<()> {
    let input = include_str!("../../inputs/day05");

    // Part 1 - Once we've added the given features to our Intcode computer, we can simply run it.
    // We'll want to make sure we get all zeros except for the last value. Then the last value is
    // the answer.
    let (_, mut input_sender, input_receiver) = Channel::new(true);
    let (_, output_sender, mut output_receiver) = Channel::new(true);
    let mut computer = Process::new(input, input_receiver, output_sender);
    tokio::spawn(async move { computer.run().await });
    input_sender.send(1).await?;

    let mut outputs = vec![];
    while let Some(output) = output_receiver.recv().await {
        outputs.push(output);
    }

    assert!(outputs.iter().take(outputs.len() - 1).all(|&x| x == 0));
    println!("p1: {}", outputs.last().unwrap());

    // Part 2 - We can do the same thing as part 1 but with a different input.
    let (_, mut input_sender, input_receiver) = Channel::new(true);
    let (_, output_sender, mut output_receiver) = Channel::new(true);
    let mut computer = Process::new(input, input_receiver, output_sender);
    tokio::spawn(async move { computer.run().await });
    input_sender.send(5).await?;
    let p2 = output_receiver.recv().await.unwrap();
    println!("p2: {}", p2);

    Ok(())
}
