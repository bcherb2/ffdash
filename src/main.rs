mod app;
mod cli;

fn main() {
    let cli = cli::parse();
    app::run(cli);
}
