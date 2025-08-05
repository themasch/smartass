use clap::Parser;
use llm::{self, chat::ChatMessage};
use std::process::Stdio;

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
    let output = get_diff(&args.base, &args.compare).unwrap();
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
        ChatMessage::user().content("Generate a short code review for the following change. If there is nothing wrong, do not generate any output").build(),
        ChatMessage::user().content(output).build()
    ];

    let output = api.chat(&chat).await.unwrap();
    dbg!(output);
}
