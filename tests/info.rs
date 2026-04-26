mod common;

use common::TestSession;
use predicates::prelude::*;
use std::fs;

fn python_string_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn shell_through_python_wrapper_session(argv: &[&str]) -> TestSession {
    let argv_literal = format!(
        "[{}]",
        argv.iter()
            .map(|arg| python_string_literal(arg))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let path = std::env::temp_dir().join(format!(
        "tb-info-wrapper-{}-{}.py",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    fs::write(
        &path,
        format!(
            concat!(
                "import os\n",
                "import pty\n",
                "import select\n",
                "import sys\n",
                "pid, master = pty.fork()\n",
                "if pid == 0:\n",
                "    os.execvp({program}, {argv})\n",
                "while True:\n",
                "    readable, _, _ = select.select([master, sys.stdin.fileno()], [], [])\n",
                "    if master in readable:\n",
                "        data = os.read(master, 1024)\n",
                "        if not data:\n",
                "            break\n",
                "        os.write(sys.stdout.fileno(), data)\n",
                "    if sys.stdin.fileno() in readable:\n",
                "        data = os.read(sys.stdin.fileno(), 1024)\n",
                "        if data:\n",
                "            os.write(master, data)\n"
            ),
            program = python_string_literal(argv[0]),
            argv = argv_literal,
        ),
    )
    .unwrap();

    let command = format!("exec python3 {}", path.display());
    let session = TestSession::new_with_startup_command(Some(&command));
    session.wait_for_shell_ready();
    let _ = fs::remove_file(path);
    session
}

mod info_shell_assessment {
    use super::*;

    #[test]
    fn pane_probing_detects_fish_through_wrapper_process() {
        let session = shell_through_python_wrapper_session(&["fish"]);

        session
            .tb_command()
            .args(["info", "--target", session.target()])
            .assert()
            .success()
            .stdout(predicate::str::contains("fish"))
            .stdout(predicate::str::contains("confident"));
    }

    #[test]
    fn pane_probing_detects_bash_through_wrapper_process() {
        let session =
            shell_through_python_wrapper_session(&["bash", "--noprofile", "--norc", "-i"]);

        session
            .tb_command()
            .args(["info", "--target", session.target()])
            .assert()
            .success()
            .stdout(predicate::str::contains("bash"))
            .stdout(predicate::str::contains("confident"));
    }

    #[test]
    fn pane_probing_detects_sh_through_wrapper_process() {
        let session = shell_through_python_wrapper_session(&["sh", "-i"]);

        session
            .tb_command()
            .args(["info", "--target", session.target()])
            .assert()
            .success()
            .stdout(predicate::str::contains("sh"))
            .stdout(predicate::str::contains("confident"));
    }

    #[test]
    fn pane_probing_reports_unknown_without_foreground_command_hint() {
        let session = TestSession::new_with_startup_command(Some("sleep 30"));

        session
            .tb_command()
            .args(["info", "--target", session.target()])
            .assert()
            .success()
            .stdout(predicate::str::contains("unknown"))
            .stdout(predicate::str::contains("unsafe"))
            .stdout(predicate::str::contains("Foreground command").not());
    }
}
