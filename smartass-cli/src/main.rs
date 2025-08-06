use anyhow::{Context, Error, Result};
use clap::Parser;
use ignore::gitignore::Gitignore;
use llm::{self, chat::ChatMessage};
use std::{ffi::OsStr, process::Stdio};
use tracing::warn;

#[derive(Debug, clap::Parser)]
struct Arguments {
    base: String,
    compare: String,
}

fn get_change_files(from: &str, to: &str) -> Result<impl IntoIterator<Item = String>> {
    let output = std::process::Command::new("git")
        .arg("--no-pager")
        .arg("diff")
        .arg("--no-color")
        .arg("--name-only")
        .arg(from)
        .arg(to)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to exec `git diff --name-only {} {}`", from, to))?;

    if !output.status.success() {
        return Err(
            Error::msg(String::from_utf8_lossy(&output.stderr).to_string()).context(format!(
                "bad result from `git diff --name-only`. return code: {}",
                output.status.code().unwrap_or_default(),
            )),
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let files = stdout
        .split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_owned())
        .collect::<Vec<_>>();

    Ok(files)
}

fn get_diff<I, S>(from: &str, to: &str, files: I) -> Result<Option<String>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut files = files.into_iter().peekable();
    if files.peek().is_none() {
        return Ok(None);
    }

    let output = std::process::Command::new("git")
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
        .output()
        .with_context(|| format!("failed to exec `git diff {} {} -- <FILES>`", from, to))?;

    if !output.status.success() {
        return Err(
            Error::msg(String::from_utf8_lossy(&output.stderr).to_string()).context(format!(
                "bad result from `git diff`. return code: {}",
                output.status.code().unwrap_or_default(),
            )),
        );
    }

    Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}

fn build_ignore_filter() -> Result<Gitignore> {
    let mut filter_builder = ignore::gitignore::GitignoreBuilder::new(std::env::current_dir()?);
    if let Some(errs) = filter_builder.add("smartass.ignore") {
        warn!("Failed to add ignore file: {:?}", errs);
    }
    Ok(filter_builder.build()?)
}

fn generate_diff(from: &str, to: &str) -> Result<Option<String>> {
    let files = get_change_files(from, to)?;
    let filter = build_ignore_filter()?;
    let filtered_files = files
        .into_iter()
        .filter(|file| filter.matched(file, false).is_none());

    get_diff(from, to, filtered_files)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().context("failed to parse env file")?;
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    let diff = generate_diff(&args.base, &args.compare)?;

    let diff = match diff {
        None => {
            println!("no changes detected!");
            return Ok(());
        }
        Some(diff) => diff,
    };

    let api = llm::builder::LLMBuilder::new()
        .backend(llm::builder::LLMBackend::Anthropic)
        .api_key(std::env::var("CLAUDE_KEY").context("missing CLAUDE_KEY environment variable")?)
        .model("claude-sonnet-4-20250514")
        .max_tokens(1024)
        .temperature(0.7)
        .system([
            "Generate a short code review for the following change.",
           "If there is nothing wrong, do not generate any output.",
           "Avoid commenting on things the usual linters would also find, focus on potential bugs.",
           "For each comment, use the following template: <<< {{file}} ({{ optional line number or numbers }}): {{ commentary }} >>>"
            ].join(" "))
        .build()?;

    let chat = vec![ChatMessage::user().content(diff).build()];

    let output = api.chat(&chat).await?;

    dbg!(output);
    Ok(())
}
