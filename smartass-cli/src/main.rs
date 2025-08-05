use clap::Parser;

#[derive(Debug, clap::Parser)]
struct Arguments {
    base: String,
    compare: String,
}

fn get_diff(from: &str, to: &str) -> Result<String, ()> {
    let diff_cmd = std::process::Command::new("git")
        .arg("--no-pager")
        .arg("diff")
        .arg("--no-color")
        .arg(from)
        .arg(to)
        .spawn()
        .unwrap();

    let output = diff_cmd.wait_with_output().unwrap();

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(());
    }

    return Ok(String::from_utf8_lossy(&output.stdout).to_string());
}

fn main() {
    let args = Arguments::parse();

    let output = get_diff(&args.base, &args.compare).unwrap();
    dbg!(output);
}
