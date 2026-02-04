use agent_tui::cli_command;
use anyhow::Context;
use anyhow::Result;
use clap::ColorChoice;
use clap::Command;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .context("failed to resolve repository root from CARGO_MANIFEST_DIR")?;

    let output_dir = repo_root.join("docs").join("cli");
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create docs dir {}", output_dir.display()))?;

    let output_path = output_dir.join("agent-tui.md");

    let mut output = String::new();
    output.push_str("# agent-tui CLI Reference\n\n");
    output.push_str("Generated from clap. Run `just cli-docs` to update.\n\n");

    let cmd = cli_command();
    let root_path = vec!["agent-tui".to_string()];
    render_command(cmd, root_path, &mut output);

    fs::write(&output_path, output)
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    println!("Wrote {}", output_path.display());
    Ok(())
}

fn render_command(mut cmd: Command, path: Vec<String>, output: &mut String) {
    cmd = cmd.color(ColorChoice::Never);
    let heading = format!("## `{}`", path.join(" "));
    output.push_str(&heading);
    output.push_str("\n\n```text\n");
    let help = cmd.render_long_help().to_string();
    output.push_str(help.trim_end());
    output.push_str("\n```\n\n");

    let subcommands: Vec<Command> = cmd
        .get_subcommands()
        .cloned()
        .collect();

    for sub in subcommands {
        let name = sub.get_name().to_string();
        let mut sub_path = path.clone();
        sub_path.push(name);
        render_command(sub, sub_path, output);
    }
}
