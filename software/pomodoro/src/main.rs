use std::io::{self, BufRead, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

const WORK_MINUTES: u64 = 25;
const BREAK_MINUTES: u64 = 5;

enum Command {
    TogglePause,
    Quit,
    InputError(String),
}

#[derive(Clone, Copy)]
enum Phase {
    Work,
    Break,
}

impl Phase {
    fn label(self) -> &'static str {
        match self {
            Self::Work => "Trabajo",
            Self::Break => "Descanso",
        }
    }

    fn duration_seconds(self) -> u64 {
        match self {
            Self::Work => WORK_MINUTES * 60,
            Self::Break => BREAK_MINUTES * 60,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Work => Self::Break,
            Self::Break => Self::Work,
        }
    }
}

fn main() {
    println!(
        "Temporizador Pomodoro: {WORK_MINUTES} minutos de trabajo y {BREAK_MINUTES} de descanso."
    );
    println!("Escribe p y presiona Enter para pausar/reanudar, o q y Enter para salir.");

    let (sender, receiver) = mpsc::channel();
    let input_thread = thread::spawn(move || read_commands(sender));
    let timer_thread = thread::spawn(move || run_timer(receiver));

    if timer_thread.join().is_err() {
        eprintln!("El hilo del temporizador terminó inesperadamente.");
    }
    if input_thread.join().is_err() {
        eprintln!("El hilo de entrada terminó inesperadamente.");
    }
}

fn read_commands(sender: Sender<Command>) {
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        input.clear();
        match stdin.lock().read_line(&mut input) {
            Ok(0) => {
                let _ = sender.send(Command::Quit);
                return;
            }
            Ok(_) => {
                for character in input.chars() {
                    match character {
                        'p' | 'P' => {
                            if sender.send(Command::TogglePause).is_err() {
                                return;
                            }
                        }
                        'q' | 'Q' => {
                            let _ = sender.send(Command::Quit);
                            return;
                        }
                        _ => {}
                    }
                }
            }
            Err(error) => {
                let _ = sender.send(Command::InputError(error.to_string()));
                return;
            }
        }
    }
}

fn run_timer(receiver: Receiver<Command>) {
    let mut phase = Phase::Work;
    let mut paused = false;

    loop {
        let mut remaining = phase.duration_seconds();

        while remaining > 0 {
            if let Err(error) = render(phase, remaining, paused) {
                eprintln!("\nError al actualizar el temporizador: {error}");
                return;
            }

            if paused {
                match receiver.recv() {
                    Ok(Command::TogglePause) => paused = false,
                    Ok(Command::Quit) => {
                        println!("\nTemporizador finalizado.");
                        return;
                    }
                    Ok(Command::InputError(error)) => {
                        eprintln!("\nError al leer la entrada: {error}");
                        return;
                    }
                    Err(_) => return,
                }
            } else {
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(Command::TogglePause) => paused = true,
                    Ok(Command::Quit) => {
                        println!("\nTemporizador finalizado.");
                        return;
                    }
                    Ok(Command::InputError(error)) => {
                        eprintln!("\nError al leer la entrada: {error}");
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => remaining -= 1,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        }

        println!("\n{} terminado.", phase.label());
        phase = phase.next();
    }
}

fn render(phase: Phase, remaining: u64, paused: bool) -> io::Result<()> {
    let minutes = remaining / 60;
    let seconds = remaining % 60;
    let status = if paused { " [PAUSADO]" } else { "" };

    print!(
        "\r{} {:02}:{:02}{}   ",
        phase.label(),
        minutes,
        seconds,
        status
    );
    io::stdout().flush()
}
