install:
    cargo build --release \
        && cp target/release/keeper ~/.local/bin/keeper \
        && chmod +x ~/.local/bin/keeper
