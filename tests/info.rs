mod common;

use common::TestSession;
use predicates::prelude::*;
use std::time::Duration;

fn shell_session(shell: &str) -> TestSession {
    let session = TestSession::new();
    session.enter_shell(shell);
    session
}

mod info_shell_assessment {
    use super::*;

    #[test]
    fn reports_confident_fish_shell() {
        let session = shell_session("fish");

        session
            .tb_command()
            .args(["info", "--target", session.target()])
            .assert()
            .success()
            .stdout(predicate::str::contains("fish"))
            .stdout(predicate::str::contains("confident"));
    }

    #[test]
    fn reports_confident_bash_shell() {
        let session = shell_session("bash");

        session
            .tb_command()
            .args(["info", "--target", session.target()])
            .assert()
            .success()
            .stdout(predicate::str::contains("bash"))
            .stdout(predicate::str::contains("confident"));
    }

    #[test]
    fn reports_confident_sh_shell() {
        let session = shell_session("sh");

        session
            .tb_command()
            .args(["info", "--target", session.target()])
            .assert()
            .success()
            .stdout(predicate::str::contains("sh"))
            .stdout(predicate::str::contains("confident"));
    }

    #[test]
    fn reports_unknown_when_target_is_not_confidently_a_shell() {
        let session = TestSession::new();
        session.send_main_pane_command("sleep 30");
        session.wait_for_current_command("sleep", Duration::from_secs(10));

        session
            .tb_command()
            .args(["info", "--target", session.target()])
            .assert()
            .success()
            .stdout(predicate::str::contains("unknown"))
            .stdout(predicate::str::contains("unsafe"));
    }
}
