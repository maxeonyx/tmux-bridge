use assert_cmd::cargo::cargo_bin;
use help_test::HelpTest;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static UNIQUE_PREFIX_COUNTER: AtomicU64 = AtomicU64::new(1);

#[test]
fn help_examples() {
    let top_level_help = help_output(&[]);
    let run_help = help_output(&["run"]);
    let launch_help = help_output(&["launch"]);

    assert!(
        top_level_help.contains(
            "Workflow: human runs `tb start`, agent uses `tb info` -> `tb run` / `tb launch` / `tb check` / `tb done`."
        ),
        "top-level help should include the workflow overview\n{top_level_help}"
    );
    assert!(
        run_help.contains("$ tb run --target a7x -- ls -la"),
        "run help should include the basic execution example\n{run_help}"
    );
    assert!(
        run_help.contains("$ tb run --target a7x --shell bash -- 'echo hello && pwd'"),
        "run help should include the explicit shell example\n{run_help}"
    );
    assert!(
        launch_help.contains("$ tb launch --target a7x -- npm run dev"),
        "launch help should include the background-task example\n{launch_help}"
    );

    assert_examples_use_allowed_flags(&run_help, &["tb", "run"], &["t"]);
    assert_examples_use_allowed_flags(&launch_help, &["tb", "launch"], &["t"]);

    if !tmux_is_available() {
        eprintln!(
            "Skipping help example execution: tmux not available; validated help text and flag style only."
        );
        return;
    }

    HelpTest::new("tb")
        .allow_short_flags(&["t"])
        .page(&[], |_fx| {})
        .page(&["start"], |_fx| {})
        .page(&["run"], |fixture| {
            let prefix = unique_prefix("run");
            fixture.env("TB_SESSION_PREFIX", prefix.clone());
            fixture.command(
                "tmux",
                &[
                    "new-session",
                    "-d",
                    "-s",
                    &format!("{prefix}a7x"),
                    "sh",
                    "-lc",
                    "IFS= read -r line; eval \"$line\"; sleep 1",
                ],
            );
        })
        .page(&["info"], |_fx| {})
        .page(&["launch"], |fixture| {
            let prefix = unique_prefix("launch");
            fixture.env("TB_SESSION_PREFIX", prefix.clone());
            fixture.env(
                "PATH",
                format!(
                    "bin:{}",
                    std::env::var("PATH").expect("PATH should exist for help examples")
                ),
            );
            fixture.dir("bin");
            fixture.file(
                "bin/npm",
                "#!/bin/sh\nif [ \"$1\" = run ] && [ \"$2\" = dev ]; then\n  printf 'dev\\n'\n  sleep 1\n  exit 0\nfi\nprintf 'unexpected npm args: %s\\n' \"$*\" >&2\nexit 64\n",
            );
            fixture.command("sh", &["-lc", "chmod +x bin/npm"]);
            fixture.command(
                "tmux",
                &[
                    "new-session",
                    "-d",
                    "-s",
                    &format!("{prefix}a7x"),
                    "sh",
                    "-lc",
                    "sleep 5",
                ],
            );
        })
        .page(&["check"], |_fx| {})
        .page(&["done"], |_fx| {})
        .run();
}

fn help_output(command_path: &[&str]) -> String {
    let output = Command::new(cargo_bin("tb"))
        .args(command_path)
        .arg("--help")
        .output()
        .expect("tb --help should run");

    assert!(
        output.status.success(),
        "tb {:?} --help should succeed\nstdout:\n{}\nstderr:\n{}",
        command_path,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("help output should be valid UTF-8")
}

fn assert_examples_use_allowed_flags(
    help: &str,
    command_words: &[&str],
    allow_short_flags: &[&str],
) {
    let command_words = command_words
        .iter()
        .map(|word| (*word).to_string())
        .collect::<Vec<_>>();

    let example_lines = help
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with('$').then_some(trimmed)
        })
        .collect::<Vec<_>>();

    assert!(
        !example_lines.is_empty(),
        "expected help page to include at least one example\n{help}"
    );

    for line in example_lines {
        let args = parse_example_args(line, &command_words);
        let short_flags = args
            .iter()
            .take_while(|arg| arg.as_str() != "--")
            .filter(|arg| is_short_flag(arg))
            .filter(|arg| {
                let flag = arg.trim_start_matches('-');
                !allow_short_flags.contains(&flag)
            })
            .cloned()
            .collect::<Vec<_>>();

        assert!(
            short_flags.is_empty(),
            "example should use long flags except {:?}: {}",
            allow_short_flags,
            line
        );
    }
}

fn parse_example_args(line: &str, command_words: &[String]) -> Vec<String> {
    let example_text = line
        .trim_start()
        .strip_prefix('$')
        .expect("example lines should start with $")
        .trim_start();
    let words = split_shell_words(example_text).expect("example should be parseable shell words");

    assert!(
        words.starts_with(command_words),
        "example should start with {:?}: {}",
        command_words,
        line
    );

    words[command_words.len()..].to_vec()
}

fn split_shell_words(input: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut quote = None;

    while let Some(ch) = chars.next() {
        match quote {
            Some('\'') => match ch {
                '\'' => quote = None,
                _ => current.push(ch),
            },
            Some('"') => match ch {
                '"' => quote = None,
                '\\' => match chars.next() {
                    Some(escaped @ ('"' | '\\' | '$' | '`')) => current.push(escaped),
                    Some('\n') => {}
                    Some(other) => {
                        current.push('\\');
                        current.push(other);
                    }
                    None => current.push('\\'),
                },
                _ => current.push(ch),
            },
            None => match ch {
                '\'' | '"' => quote = Some(ch),
                '\\' => match chars.next() {
                    Some('\n') => {}
                    Some(other) => current.push(other),
                    None => return Err("trailing backslash".to_string()),
                },
                ch if ch.is_whitespace() => {
                    if !current.is_empty() {
                        words.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            },
            Some(other) => unreachable!("unexpected quote state: {other}"),
        }
    }

    if let Some(quote) = quote {
        return Err(format!("unterminated {quote} quote"));
    }

    if !current.is_empty() {
        words.push(current);
    }

    Ok(words)
}

fn is_short_flag(arg: &str) -> bool {
    let mut chars = arg.chars();
    matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some('-'), Some(letter), None) if letter.is_ascii_alphabetic()
    )
}

fn tmux_is_available() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn unique_prefix(label: &str) -> String {
    let id = UNIQUE_PREFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("tb-help-{label}-{}-{}-", std::process::id(), id)
}
