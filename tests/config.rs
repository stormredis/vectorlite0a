use vector::topology::{self, Config};

fn load(config: &str) -> Result<Vec<String>, Vec<String>> {
    Config::load(config.as_bytes())
        .and_then(|c| topology::builder::build_pieces(&c))
        .map(|(_topology, warnings)| warnings)
}

#[test]
fn happy_path() {
    load(
        r#"
        [sources.in]
        type = "tcp"
        address = "127.0.0.1:1235"

        [transforms.sampler]
        type = "sampler"
        inputs = ["in"]
        rate = 10
        pass_list = ["error"]

        [sinks.out]
        type = "tcp"
        inputs = ["sampler"]
        address = "127.0.0.1:9999"
      "#,
    )
    .unwrap();

    load(
        r#"
        [sources]
        in = {type = "tcp", address = "127.0.0.1:1235"}

        [transforms]
        sampler = {type = "sampler", inputs = ["in"], rate = 10, pass_list = ["error"]}

        [sinks]
        out = {type = "tcp", inputs = ["sampler"], address = "127.0.0.1:9999"}
      "#,
    )
    .unwrap();
}

#[test]
fn early_eof() {
    let err = load("[sinks]\n[sin").unwrap_err();

    assert_eq!(err, vec!["expected a right bracket, found eof at line 2"]);
}

#[test]
fn bad_syntax() {
    let err = load(r#"{{{"#).unwrap_err();

    assert_eq!(
        err,
        vec!["expected a table key, found a left brace at line 1"]
    );
}

#[test]
fn missing_key() {
    let err = load(
        r#"
        [sources.in]
        type = "tcp"

        [sinks.out]
        type = "tcp"
        inputs = ["in"]
        address = "127.0.0.1:9999"
      "#,
    )
    .unwrap_err();

    assert_eq!(err, vec!["missing field `address` for key `sources.in`"]);
}

#[test]
fn bad_type() {
    let err = load(
        r#"
        [sources.in]
        type = "tcp"
        address = "127.0.0.1:1234"

        [sinks.out]
        type = "jabberwocky"
        inputs = ["in"]
        address = "127.0.0.1:9999"
      "#,
    )
    .unwrap_err();

    assert_eq!(err.len(), 1);
    assert!(err[0].starts_with("unknown variant `jabberwocky`, expected one of "));
}

#[test]
fn nonexistant_input() {
    let err = load(
        r#"
        [sources.in]
        type = "tcp"
        address = "127.0.0.1:1235"

        [transforms.sampler]
        type = "sampler"
        inputs = ["qwerty"]
        rate = 10
        pass_list = ["error"]

        [sinks.out]
        type = "tcp"
        inputs = ["asdf"]
        address = "127.0.0.1:9999"
      "#,
    )
    .unwrap_err();

    assert_eq!(
        err,
        vec![
            "Input \"asdf\" for sink \"out\" doesn't exist.",
            "Input \"qwerty\" for transform \"sampler\" doesn't exist.",
        ]
    );
}

#[test]
fn bad_regex() {
    let err = load(
        r#"
        [sources.in]
        type = "tcp"
        address = "127.0.0.1:1235"

        [transforms.sampler]
        type = "sampler"
        inputs = ["in"]
        rate = 10
        pass_list = ["(["]

        [sinks.out]
        type = "tcp"
        inputs = ["sampler"]
        address = "127.0.0.1:9999"
      "#,
    )
    .unwrap_err();

    assert_eq!(err, vec!["Transform \"sampler\": regex parse error:\n    ([\n     ^\nerror: unclosed character class"]);

    let err = load(
        r#"
        [sources.in]
        type = "tcp"
        address = "127.0.0.1:1235"

        [transforms.parser]
        type = "regex_parser"
        inputs = ["in"]
        regex = "(["

        [sinks.out]
        type = "tcp"
        inputs = ["parser"]
        address = "127.0.0.1:9999"
      "#,
    )
    .unwrap_err();

    assert_eq!(err, vec!["Transform \"parser\": regex parse error:\n    ([\n     ^\nerror: unclosed character class"]);
}

#[test]
fn good_regex_parser() {
    let result = load(
        r#"
        [sources.in]
        type = "tcp"
        address = "127.0.0.1:1235"

        [transforms.parser]
        type = "regex_parser"
        inputs = ["in"]
        regex = "(?P<out>.+)"

        [transforms.parser.types]
        out = "integer"

        [sinks.out]
        type = "tcp"
        inputs = ["parser"]
        address = "127.0.0.1:9999"
      "#,
    );

    assert!(result.is_ok());
}

#[test]
fn good_tokenizer() {
    let result = load(
        r#"
        [sources.in]
        type = "tcp"
        address = "127.0.0.1:1235"

        [transforms.parser]
        type = "tokenizer"
        inputs = ["in"]
        field_names = ["one", "two", "three", "four"]

        [transforms.parser.types]
        one = "integer"
        two = "boolean"

        [sinks.out]
        type = "tcp"
        inputs = ["parser"]
        address = "127.0.0.1:9999"
      "#,
    );

    assert!(result.is_ok());
}

#[test]
fn warnings() {
    let warnings = load(
        r#"
        [sources.in1]
        type = "tcp"
        address = "127.0.0.1:1235"

        [sources.in2]
        type = "tcp"
        address = "127.0.0.1:1236"

        [transforms.sampler1]
        type = "sampler"
        inputs = ["in1"]
        rate = 10
        pass_list = ["error"]

        [transforms.sampler2]
        type = "sampler"
        inputs = ["in1"]
        rate = 10
        pass_list = ["error"]

        [sinks.out]
        type = "tcp"
        inputs = ["sampler1"]
        address = "127.0.0.1:9999"
      "#,
    )
    .unwrap();

    assert_eq!(
        warnings,
        vec![
            "Transform \"sampler2\" has no outputs",
            "Source \"in2\" has no outputs",
        ]
    )
}

#[test]
fn cycle() {
    let errors = load(
        r#"
        [sources.in]
        type = "tcp"
        address = "127.0.0.1:1235"

        [transforms.one]
        type = "sampler"
        inputs = ["in"]
        rate = 10
        pass_list = []

        [transforms.two]
        type = "sampler"
        inputs = ["one", "four"]
        rate = 10
        pass_list = []

        [transforms.three]
        type = "sampler"
        inputs = ["two"]
        rate = 10
        pass_list = []

        [transforms.four]
        type = "sampler"
        inputs = ["three"]
        rate = 10
        pass_list = []

        [sinks.out]
        type = "tcp"
        inputs = ["four"]
        address = "127.0.0.1:9999"
      "#,
    )
    .unwrap_err();

    assert_eq!(errors, vec!["Configured topology contains a cycle"])
}

#[test]
fn disabled_healthcheck() {
    load(
        r#"
      [sources.in]
      type = "tcp"
      address = "127.0.0.1:1234"

      [sinks.out]
      type = "tcp"
      inputs = ["in"]
      address = "0.0.0.0:0"
      healthcheck = false
      "#,
    )
    .unwrap();
}
