use clap::Parser;
use llm::{self, chat::ChatMessage};
use std::{process::Stdio, slice};

#[derive(Debug, clap::Parser)]
struct Arguments {
    base: String,
    compare: String,
}

fn get_change_files(from: &str, to: &str) -> Result<Vec<String>, ()> {
    let diff_cmd = std::process::Command::new("git")
        .arg("--no-pager")
        .arg("diff")
        .arg("--no-color")
        .arg("--name-only")
        .arg(from)
        .arg(to)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let output = diff_cmd.wait_with_output().unwrap();

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let files = stdout
        .split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_owned())
        .collect();

    Ok(files)
}

fn get_diff(from: &str, to: &str, files: Vec<&str>) -> Result<String, ()> {
    let diff_cmd = std::process::Command::new("git")
        .arg("--no-pager")
        .arg("diff")
        .arg("--no-color")
        .arg(from)
        .arg(to)
        .arg("--")
        .args(files)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let output = diff_cmd.wait_with_output().unwrap();

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(());
    }

    return Ok(String::from_utf8_lossy(&output.stdout).to_string());
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("failed to parse env file");
    let args = Arguments::parse();

    let files = dbg!(get_change_files(&args.base, &args.compare).unwrap());

    let mut filter_builder =
        ignore::gitignore::GitignoreBuilder::new(std::env::current_dir().unwrap());
    if let Some(errs) = filter_builder.add("smartass.ignore") {
        panic!("Failed to add ignore file: {:?}", errs);
    }
    let filter = filter_builder.build().unwrap();

    let filtered_files = files
        .iter()
        .filter(|file| filter.matched(file, false).is_none())
        .map(|file| file.as_str())
        .collect::<Vec<_>>();

    let output = get_diff(&args.base, &args.compare, dbg!(filtered_files)).unwrap();
    dbg!(&output);

    let api = llm::builder::LLMBuilder::new()
        .backend(llm::builder::LLMBackend::Anthropic)
        .api_key(std::env::var("CLAUDE_KEY").unwrap())
        .model("claude-sonnet-4-20250514")
        .max_tokens(1024)
        .temperature(0.7)
        .build()
        .unwrap();

    let chat = vec![
        ChatMessage::user().content("Generate a short code review for the following change. If there is nothing wrong, do not generate any output. Avoid commenting on things the usual linters would also find, focus on potential bugs.").build(),
        ChatMessage::user().content(output).build()
    ];

    let output = api.chat(&chat).await.unwrap();

    dbg!(output);
}
