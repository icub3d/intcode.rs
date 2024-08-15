const INPUT: &str = include_str!("../../inputs/day21");

use intcode::ipc::Channel;
use intcode::process::Process;

#[tokio::main]
async fn main() {
    let (_, mut input_tx, input_rx) = Channel::new(true);
    let (_, output_tx, mut output_rx) = Channel::new(true);
    tokio::spawn(async move {
        let mut process = Process::new(INPUT, input_rx, output_tx);
        process.run().await.unwrap();
    });

    tokio::spawn(async move {
        let program = "NOT B J
NOT C T
OR T J
AND D J
NOT A T
OR T J
WALK
";
        for c in program.chars() {
            input_tx.send(c as isize).await.unwrap();
        }
    });
    let mut damage = 0;
    while let Some(output) = output_rx.recv().await {
        if output > 255 {
            damage = output;
        } else {
            print!("{}", output as u8 as char);
        }
    }
    println!("p1: {}", damage);

    let (_, mut input_tx, input_rx) = Channel::new(true);
    let (_, output_tx, mut output_rx) = Channel::new(true);
    tokio::spawn(async move {
        let mut process = Process::new(INPUT, input_rx, output_tx);
        process.run().await.unwrap();
    });

    tokio::spawn(async move {
        let program = "NOT B J
NOT C T
OR T J
AND D J
AND H J
NOT A T
OR T J
RUN
";
        for c in program.chars() {
            input_tx.send(c as isize).await.unwrap();
        }
    });
    let mut damage = 0;
    while let Some(output) = output_rx.recv().await {
        if output > 255 {
            damage = output;
        } else {
            print!("{}", output as u8 as char);
        }
    }
    println!("p2: {}", damage);
}
