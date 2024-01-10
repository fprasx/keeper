install target:
    cargo build \
        --release \
        --bin {{target}} \
        && cp target/release/{{target}} ~/.local/bin/{{target}} \
        && chmod +x ~/.local/bin/{{target}}
