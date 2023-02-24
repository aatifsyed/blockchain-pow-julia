use clap::Parser as _;
use color_eyre::eyre::Context as _;
use tracing::{debug, info};

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long)]
    num_threads: Option<usize>,
    #[arg(short = 'r', long, default_value_t = 0.5)]
    julia_c_real: f64,
    #[arg(short = 'i', long, default_value_t = 0.5)]
    julia_c_imag: f64,
    #[arg(short = 'l', long, default_value_t = 0.0)]
    real_lower_bound: f64,
    #[arg(short = 'u', long, default_value_t = 0.5)]
    real_upper_bound: f64,
    #[arg(short = 't', long, default_value_t = 10)]
    target_iterations: u16,
    #[arg(short, long)]
    keep_going: bool,
}

// suspicious: solutions are found basically right away or not within ~2h
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env()
        .context("couldn't parse RUST_LOG environment variable")?;
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let args = Args::parse();
    debug!(?args);

    let num_threads = match (args.num_threads, std::thread::available_parallelism()) {
        (Some(user), _) => user,
        (None, Ok(default)) => default.get(),
        (_, Err(err)) => {
            return Err(color_eyre::Report::from(err))
                .context("couldn't get a default for num threads")
        }
    };
    info!("spawning {num_threads} threads");

    let (sender, receiver) = std::sync::mpsc::channel();

    for thread_num in 0..num_threads {
        let sender = sender.clone();
        std::thread::spawn(move || loop {
            let found = blockchain_pow_julia::do_work(
                num::Complex {
                    re: args.julia_c_real,
                    im: args.julia_c_imag,
                },
                args.real_lower_bound,
                args.real_upper_bound,
                args.target_iterations,
            );
            info!(%thread_num, %found);
            if !args.keep_going {
                sender.send(found).unwrap();
            }
        });
    }

    let _found = receiver.recv().unwrap();

    Ok(())
}
