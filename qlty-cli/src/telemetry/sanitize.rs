use crate::arguments::Arguments;
use clap::CommandFactory;
use lazy_static::lazy_static;
use regex::Regex;

const ALLOWED_ARG_STRINGS: [&str; 4] = ["true", "false", "yes", "no"];
const ALLOWED_ARG_PATTERNS: [&str; 2] = [r"^(\d+\.\d+\.\d+)$", r"^(\d+\.?\d*)$"];

lazy_static! {
    static ref ALLOWED_ARG_REGEXES: Vec<Regex> = ALLOWED_ARG_PATTERNS
        .iter()
        .map(|pattern| Regex::new(pattern).unwrap())
        .collect::<Vec<Regex>>();
}

pub fn sanitize_command(command: &str) -> (String, String, String) {
    let args: Vec<_> = command.split(' ').collect();

    if args.len() < 2 {
        return (args[0].to_owned(), "".to_owned(), "".to_owned());
    }

    let program = args[0].to_owned();

    let subcommand = subcommand(command);
    let subcommand_parts = subcommand.split(' ').collect::<Vec<&str>>();

    let mut sanitized_args = vec![];

    for arg in &args[subcommand_parts.len() + 1..] {
        sanitized_args.push(sanitize_arg(arg));
    }

    (program, subcommand, sanitized_args.join(" "))
}

fn subcommand(command: &str) -> String {
    let mut result = vec![];
    let arg_matches = Arguments::command().get_matches_from(command.split(' '));

    // This works for up to 3 levels of subcommands. There's probably a better way to do this.
    if let Some((name, arg_matches)) = arg_matches.subcommand() {
        result.push(name.to_owned());

        if let Some((name, arg_matches)) = arg_matches.subcommand() {
            result.push(name.to_owned());

            if let Some((name, _arg_matches)) = arg_matches.subcommand() {
                result.push(name.to_owned());
            }
        }
    }

    result.join(" ")
}

fn sanitize_arg(arg: &str) -> String {
    if arg.contains('=') {
        let parts: Vec<&str> = arg.split('=').collect();

        let sanitized_parts = parts
            .iter()
            .map(|part| sanitize_arg(part).to_owned())
            .collect::<Vec<String>>();

        sanitized_parts.join("=")
    } else if arg.starts_with('-')
        || ALLOWED_ARG_STRINGS.contains(&arg)
        || ALLOWED_ARG_REGEXES.iter().any(|regex| regex.is_match(arg))
    {
        arg.to_owned()
    } else {
        "VALUE".to_owned()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sanitize_cmd_path() {
        assert_eq!(
            sanitize_command("qlty"),
            ("qlty".to_owned(), "".to_owned(), "".to_owned())
        );
        assert_eq!(
            sanitize_command("qlty check some/path"),
            ("qlty".to_owned(), "check".to_owned(), "VALUE".to_owned())
        );
        assert_eq!(
            sanitize_command("qlty plugins enable eslint=1.2.3"),
            (
                "qlty".to_owned(),
                "plugins enable".to_owned(),
                "VALUE=1.2.3".to_owned()
            )
        );
        assert_eq!(
            sanitize_command("qlty plugins enable 123 4.56"),
            (
                "qlty".to_owned(),
                "plugins enable".to_owned(),
                "123 4.56".to_owned()
            )
        );
        assert_eq!(
            sanitize_command("qlty check --level=medium"),
            (
                "qlty".to_owned(),
                "check".to_owned(),
                "--level=VALUE".to_owned()
            )
        );
    }

    #[test]
    fn sanitize_redact() {
        assert_eq!(
            sanitize_command("qlty check foo bar.rb foo/bar"),
            (
                "qlty".to_owned(),
                "check".to_owned(),
                "VALUE VALUE VALUE".to_owned()
            )
        );

        assert_eq!(
            sanitize_command("qlty check --level=low"),
            (
                "qlty".to_owned(),
                "check".to_owned(),
                "--level=VALUE".to_owned()
            )
        );

        assert_eq!(
            sanitize_command("qlty check --all --no-progress --filter=flake8"),
            (
                "qlty".to_owned(),
                "check".to_owned(),
                "--all --no-progress --filter=VALUE".to_owned()
            )
        );
    }
}
