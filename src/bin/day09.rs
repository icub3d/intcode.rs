use intcode::ipc::Channel;
use intcode::process::Process;

#[tokio::main]
async fn main() {
    let input = include_str!("inputs/day09");

    // Once we have the updates to the Intcode computer, we can use the new
    // code to run the input program. For part 1, we send a 1. For part 2, we
    // send a 2.
    for part in 1..=2 {
        let (_, mut tx, rx) = Channel::new(true);
        let (_, tx2, mut output) = Channel::new(true);
        tx.send(part).await.unwrap();
        tokio::spawn(async move {
            let mut process = Process::new(input, rx, tx2);
            process.run().await.unwrap();
        });

        while let Some(value) = output.recv().await {
            println!("p{}: {}", part, value);
        }
    }
}
